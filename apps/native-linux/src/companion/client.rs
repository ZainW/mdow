//! A small client for the OpenCode 2 HTTP API (`opencode serve`). The server is private and on
//! loopback, so this speaks plain HTTP/1.1 over a socket. That also lets the long-lived event
//! stream be cancelled by shutting the socket down.

use super::events::{Event, SseParser};
use gtk::glib;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelRef {
    pub provider_id: String,
    pub id: String,
}

impl ModelRef {
    /// `providerID/modelID`, split on the first slash (model ids may contain slashes).
    pub fn parse(value: &str) -> Option<ModelRef> {
        let (provider, id) = value.split_once('/')?;
        (!provider.is_empty() && !id.is_empty())
            .then(|| ModelRef { provider_id: provider.to_string(), id: id.to_string() })
    }

    pub fn value(&self) -> String {
        format!("{}/{}", self.provider_id, self.id)
    }

    fn json(&self) -> Value {
        json!({"providerID": self.provider_id, "id": self.id})
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    pub name: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ProviderInfo {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleEffect {
    Deny,
    Ask,
}

impl RuleEffect {
    fn id(self) -> &'static str {
        match self {
            RuleEffect::Deny => "deny",
            RuleEffect::Ask => "ask",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionRule {
    pub action: String,
    pub resource: String,
    pub effect: RuleEffect,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SessionTime {
    pub idle: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SessionInfo {
    pub outcome: Option<String>,
    pub time: SessionTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionReply {
    Once,
    Reject,
}

impl PermissionReply {
    fn id(self) -> &'static str {
        match self {
            PermissionReply::Once => "once",
            PermissionReply::Reject => "reject",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpError {
    pub status: u16,
    pub message: String,
}

/// The server's own message when the body is a JSON error, else the start of the body.
fn error_message(status: u16, body: &[u8]) -> String {
    if let Ok(json) = serde_json::from_slice::<Value>(body) {
        let message = json
            .pointer("/error/message")
            .or_else(|| json.get("message"))
            .and_then(Value::as_str)
            .filter(|m| !m.is_empty());
        if let Some(message) = message {
            return message.to_string();
        }
    }
    let text = String::from_utf8_lossy(body);
    let text = text.trim();
    if text.is_empty() {
        format!("OpenCode request failed ({status})")
    } else {
        text.chars().take(200).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    /// Path and query.
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

impl Request {
    #[cfg(test)]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }
}

pub trait Transport: Send + Sync {
    fn send(&self, request: &Request) -> Result<(u16, Vec<u8>), String>;
}

/// `host:port` of an `http://` address.
fn authority(base_url: &str) -> Option<String> {
    let rest = base_url.strip_prefix("http://")?;
    Some(rest.split('/').next()?.to_string())
}

fn write_request(stream: &mut impl Write, request: &Request, host: &str) -> std::io::Result<()> {
    let mut head = format!(
        "{} {} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n",
        request.method, request.path
    );
    for (name, value) in &request.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    if let Some(body) = &request.body {
        head.push_str(&format!("Content-Length: {}\r\n", body.len()));
    } else if request.method != "GET" {
        head.push_str("Content-Length: 0\r\n");
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    if let Some(body) = &request.body {
        stream.write_all(body)?;
    }
    stream.flush()
}

struct ResponseHead {
    status: u16,
    chunked: bool,
    length: Option<u64>,
}

fn read_head(reader: &mut impl BufRead) -> std::io::Result<ResponseHead> {
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let status = line
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse::<u16>().ok())
            .ok_or_else(|| std::io::Error::other("OpenCode sent a malformed response"))?;
        let (mut chunked, mut length) = (false, None);
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header)? == 0 || header.trim().is_empty() {
                break;
            }
            let Some((name, value)) = header.split_once(':') else { continue };
            let (name, value) = (name.trim().to_ascii_lowercase(), value.trim());
            if name == "transfer-encoding" {
                chunked = value.to_ascii_lowercase().contains("chunked");
            } else if name == "content-length" {
                length = value.parse().ok();
            }
        }
        if (100..200).contains(&status) {
            continue;
        }
        return Ok(ResponseHead { status, chunked, length });
    }
}

/// Decodes `Transfer-Encoding: chunked`.
struct ChunkedReader<R> {
    inner: R,
    remaining: u64,
    done: bool,
}

impl<R: BufRead> Read for ChunkedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.done || buf.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            let mut line = String::new();
            if self.inner.read_line(&mut line)? == 0 {
                self.done = true;
                return Ok(0);
            }
            let size = line.trim().split(';').next().unwrap_or_default();
            self.remaining = u64::from_str_radix(size, 16)
                .map_err(|_| std::io::Error::other("OpenCode sent a malformed chunk"))?;
            if self.remaining == 0 {
                self.done = true;
                return Ok(0);
            }
        }
        let want = buf.len().min(self.remaining as usize);
        let read = self.inner.read(&mut buf[..want])?;
        if read == 0 {
            self.done = true;
            return Ok(0);
        }
        self.remaining -= read as u64;
        if self.remaining == 0 {
            let mut end = String::new();
            self.inner.read_line(&mut end)?;
        }
        Ok(read)
    }
}

fn read_body(mut reader: impl BufRead, head: &ResponseHead) -> std::io::Result<Vec<u8>> {
    let mut body = Vec::new();
    if head.chunked {
        ChunkedReader { inner: reader, remaining: 0, done: false }.read_to_end(&mut body)?;
    } else if let Some(length) = head.length {
        reader.take(length).read_to_end(&mut body)?;
    } else {
        reader.read_to_end(&mut body)?;
    }
    Ok(body)
}

fn connect(host: &str, read_timeout: Option<Duration>) -> std::io::Result<TcpStream> {
    let address = host
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| std::io::Error::other("OpenCode's address could not be resolved"))?;
    let stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))?;
    stream.set_read_timeout(read_timeout)?;
    stream.set_nodelay(true)?;
    Ok(stream)
}

pub struct TcpTransport {
    host: String,
}

impl Transport for TcpTransport {
    fn send(&self, request: &Request) -> Result<(u16, Vec<u8>), String> {
        let exchange = || -> std::io::Result<(u16, Vec<u8>)> {
            let mut stream = connect(&self.host, Some(Duration::from_secs(60)))?;
            write_request(&mut stream, request, &self.host)?;
            let mut reader = BufReader::new(stream);
            let head = read_head(&mut reader)?;
            let body = read_body(reader, &head)?;
            Ok((head.status, body))
        };
        exchange().map_err(|err| err.to_string())
    }
}

/// Stops an event stream from another thread.
#[derive(Default)]
pub struct StreamCancel {
    cancelled: AtomicBool,
    stream: Mutex<Option<TcpStream>>,
}

impl StreamCancel {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Ok(stream) = self.stream.lock()
            && let Some(stream) = stream.as_ref()
        {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    fn set(&self, stream: Option<TcpStream>) {
        if let Ok(mut slot) = self.stream.lock() {
            *slot = stream;
        }
    }
}

/// Item from the event stream: it (re)connected, or an event arrived.
#[derive(Debug)]
pub enum StreamItem {
    /// `reconnected` is false the first time.
    Opened {
        reconnected: bool,
    },
    Event(Event),
    /// The stream gave up; only a 401 does this.
    Failed(String),
}

#[derive(Clone)]
pub struct Client {
    base_url: String,
    authorization: String,
    transport: Arc<dyn Transport>,
}

/// Escapes one URL component, like `encodeURIComponent`.
fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn location_query(directory: &str) -> String {
    format!("location%5Bdirectory%5D={}", encode(directory))
}

impl Client {
    pub fn new(base_url: &str, password: &str) -> Client {
        let host = authority(base_url).unwrap_or_default();
        Client::with_transport(base_url, password, Arc::new(TcpTransport { host }))
    }

    pub fn with_transport(base_url: &str, password: &str, transport: Arc<dyn Transport>) -> Client {
        let credentials = glib::base64_encode(format!("opencode:{password}").as_bytes());
        Client {
            base_url: base_url.trim_end_matches('/').to_string(),
            authorization: format!("Basic {credentials}"),
            transport,
        }
    }

    fn make_request(&self, method: &str, path: &str, body: Option<Value>) -> Request {
        let mut headers = vec![("authorization".to_string(), self.authorization.clone())];
        let body = body.map(|body| {
            headers.push(("content-type".to_string(), "application/json".to_string()));
            body.to_string().into_bytes()
        });
        Request { method: method.to_string(), path: path.to_string(), headers, body }
    }

    fn send(&self, method: &str, path: &str, body: Option<Value>) -> Result<Vec<u8>, HttpError> {
        let request = self.make_request(method, path, body);
        let (status, data) =
            self.transport.send(&request).map_err(|message| HttpError { status: 0, message })?;
        if !(200..300).contains(&status) {
            return Err(HttpError { status, message: error_message(status, &data) });
        }
        Ok(data)
    }

    fn get<T: for<'a> Deserialize<'a>>(&self, path: &str) -> Result<T, HttpError> {
        #[derive(Deserialize)]
        struct Envelope<T> {
            data: T,
        }
        let data = self.send("GET", path, None)?;
        serde_json::from_slice::<Envelope<T>>(&data)
            .map(|envelope| envelope.data)
            .map_err(|err| HttpError { status: 0, message: err.to_string() })
    }

    pub fn list_models(&self, directory: &str) -> Result<Vec<ModelInfo>, HttpError> {
        self.get(&format!("/api/model?{}", location_query(directory)))
    }

    pub fn list_providers(&self, directory: &str) -> Result<Vec<ProviderInfo>, HttpError> {
        self.get(&format!("/api/provider?{}", location_query(directory)))
    }

    pub fn default_model(&self, directory: &str) -> Result<Option<ModelRef>, HttpError> {
        #[derive(Deserialize)]
        struct Default {
            #[serde(rename = "providerID")]
            provider_id: String,
            id: String,
        }
        let found: Option<Default> =
            self.get(&format!("/api/model/default?{}", location_query(directory)))?;
        Ok(found.map(|d| ModelRef { provider_id: d.provider_id, id: d.id }))
    }

    pub fn session_body(
        directory: &str,
        model: Option<&ModelRef>,
        permissions: &[PermissionRule],
    ) -> Value {
        let mut body = Map::new();
        body.insert("location".to_string(), json!({"directory": directory}));
        body.insert(
            "permissions".to_string(),
            Value::Array(
                permissions
                    .iter()
                    .map(|rule| {
                        json!({
                            "action": rule.action,
                            "resource": rule.resource,
                            "effect": rule.effect.id(),
                        })
                    })
                    .collect(),
            ),
        );
        if let Some(model) = model {
            body.insert("model".to_string(), model.json());
        }
        Value::Object(body)
    }

    pub fn create_session(
        &self,
        directory: &str,
        model: Option<&ModelRef>,
        permissions: &[PermissionRule],
    ) -> Result<String, HttpError> {
        let body = Client::session_body(directory, model, permissions);
        let data = self.send("POST", "/api/session", Some(body))?;
        let json: Value = serde_json::from_slice(&data)
            .map_err(|err| HttpError { status: 0, message: err.to_string() })?;
        json.pointer("/data/id").and_then(Value::as_str).map(str::to_string).ok_or_else(|| {
            HttpError { status: 0, message: "OpenCode sent no session.".to_string() }
        })
    }

    pub fn set_instructions(&self, session: &str, key: &str, value: &str) -> Result<(), HttpError> {
        let path =
            format!("/api/experimental/session/{session}/instructions/entries/{}", encode(key));
        self.send("PUT", &path, Some(json!({"value": value}))).map(|_| ())
    }

    /// Whether the session has finished its latest run, and how.
    pub fn get_session(&self, session: &str) -> Result<SessionInfo, HttpError> {
        self.get(&format!("/api/session/{session}"))
    }

    pub fn switch_model(&self, session: &str, model: &ModelRef) -> Result<(), HttpError> {
        self.send(
            "POST",
            &format!("/api/session/{session}/model"),
            Some(json!({"model": model.json()})),
        )
        .map(|_| ())
    }

    pub fn prompt(&self, session: &str, text: &str) -> Result<(), HttpError> {
        self.send("POST", &format!("/api/session/{session}/prompt"), Some(json!({"text": text})))
            .map(|_| ())
    }

    pub fn interrupt(&self, session: &str) -> Result<(), HttpError> {
        self.send("POST", &format!("/api/session/{session}/interrupt"), None).map(|_| ())
    }

    pub fn reply_permission(
        &self,
        session: &str,
        request: &str,
        decision: PermissionReply,
        message: Option<&str>,
    ) -> Result<(), HttpError> {
        let mut body = Map::new();
        body.insert("decision".to_string(), json!(decision.id()));
        if let Some(message) = message.filter(|m| !m.is_empty()) {
            body.insert("message".to_string(), json!(message));
        }
        let path = format!("/api/session/{session}/permission/{request}/reply");
        self.send("POST", &path, Some(Value::Object(body))).map(|_| ())
    }

    pub fn event_request(&self, directory: &str) -> Request {
        let mut request =
            self.make_request("GET", &format!("/api/event?{}", location_query(directory)), None);
        request.headers.push(("accept".to_string(), "text/event-stream".to_string()));
        request
    }

    /// Reads one connection's events until it drops. Returns the HTTP status if it was refused.
    fn read_events(
        &self,
        host: &str,
        request: &Request,
        cancel: &StreamCancel,
        opened: &mut bool,
        attempt: &mut u32,
        on_item: &mut impl FnMut(StreamItem) -> bool,
    ) -> Result<(), u16> {
        let Ok(mut stream) = connect(host, None) else { return Ok(()) };
        cancel.set(stream.try_clone().ok());
        if cancel.is_cancelled() || write_request(&mut stream, request, host).is_err() {
            return Ok(());
        }
        let mut reader = BufReader::new(stream);
        let Ok(head) = read_head(&mut reader) else { return Ok(()) };
        if head.status != 200 {
            return Err(head.status);
        }
        *attempt = 0;
        if !on_item(StreamItem::Opened { reconnected: *opened }) {
            return Ok(());
        }
        *opened = true;
        let body: Box<dyn Read + Send> = if head.chunked {
            Box::new(ChunkedReader { inner: reader, remaining: 0, done: false })
        } else {
            Box::new(reader)
        };
        let mut lines = BufReader::new(body);
        let mut parser = SseParser::default();
        let mut line = Vec::new();
        loop {
            line.clear();
            match lines.read_until(b'\n', &mut line) {
                Ok(0) | Err(_) => return Ok(()),
                Ok(_) => {}
            }
            while line.last() == Some(&b'\n') {
                line.pop();
            }
            let text = String::from_utf8_lossy(&line);
            if let Some(payload) = parser.feed_line(&text)
                && let Some(event) = Event::parse(&payload)
                && !on_item(StreamItem::Event(event))
            {
                return Ok(());
            }
            if cancel.is_cancelled() {
                return Ok(());
            }
        }
    }

    /// Streams server events for a folder until cancelled or `on_item` returns false, reconnecting
    /// with backoff when the connection drops. Blocks; run it on its own thread.
    pub fn stream_events(
        &self,
        directory: &str,
        cancel: &StreamCancel,
        mut on_item: impl FnMut(StreamItem) -> bool,
    ) {
        let Some(host) = authority(&self.base_url) else {
            on_item(StreamItem::Failed("OpenCode's address is not usable.".to_string()));
            return;
        };
        let request = self.event_request(directory);
        let (mut attempt, mut opened) = (0, false);
        while !cancel.is_cancelled() {
            let result =
                self.read_events(&host, &request, cancel, &mut opened, &mut attempt, &mut on_item);
            cancel.set(None);
            if result == Err(401) {
                on_item(StreamItem::Failed("OpenCode event stream unavailable".to_string()));
                return;
            }
            attempt += 1;
            let delay = (250u64 << attempt.min(5)).min(5_000);
            let mut waited = 0;
            while waited < delay && !cancel.is_cancelled() {
                std::thread::sleep(Duration::from_millis(50));
                waited += 50;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::companion::logic;
    use std::io::Cursor;

    /// Records requests and answers with a canned response.
    struct Fake {
        requests: Mutex<Vec<Request>>,
        status: u16,
        body: Vec<u8>,
    }

    impl Transport for Fake {
        fn send(&self, request: &Request) -> Result<(u16, Vec<u8>), String> {
            self.requests.lock().unwrap().push(request.clone());
            Ok((self.status, self.body.clone()))
        }
    }

    fn fake(status: u16, body: &str) -> (Arc<Fake>, Client) {
        let fake = Arc::new(Fake {
            requests: Mutex::new(Vec::new()),
            status,
            body: body.as_bytes().to_vec(),
        });
        let client = Client::with_transport("http://x", "p", fake.clone());
        (fake, client)
    }

    fn last(fake: &Fake) -> Request {
        fake.requests.lock().unwrap().last().unwrap().clone()
    }

    fn body(request: &Request) -> Value {
        serde_json::from_slice(request.body.as_ref().unwrap()).unwrap()
    }

    #[test]
    fn sends_basic_auth_and_scopes_requests_to_a_folder() {
        let (fake, client) = fake(200, "{\"data\":[]}");
        client.list_models("/my docs").unwrap();
        let request = last(&fake);
        assert_eq!(request.path, "/api/model?location%5Bdirectory%5D=%2Fmy%20docs");
        let expected = glib::base64_encode(b"opencode:p");
        assert_eq!(request.header("authorization"), Some(format!("Basic {expected}").as_str()));
    }

    #[test]
    fn surfaces_the_server_error_message() {
        let (fake, client) = fake(400, "{\"error\":{\"message\":\"Unknown model\"}}");
        let err = client.prompt("ses_1", "hi").unwrap_err();
        assert_eq!(err, HttpError { status: 400, message: "Unknown model".to_string() });
        let request = last(&fake);
        assert_eq!(body(&request), json!({"text": "hi"}));
        assert_eq!(request.header("content-type"), Some("application/json"));
    }

    #[test]
    fn replies_to_permissions_with_the_decision_and_an_optional_message() {
        let (fake, client) = fake(204, "");
        client
            .reply_permission("ses_1", "per_1", PermissionReply::Reject, Some("No thanks"))
            .unwrap();
        let request = last(&fake);
        assert_eq!(request.path, "/api/session/ses_1/permission/per_1/reply");
        assert_eq!(request.method, "POST");
        assert_eq!(body(&request), json!({"decision": "reject", "message": "No thanks"}));
        client.reply_permission("ses_1", "per_2", PermissionReply::Once, None).unwrap();
        assert_eq!(body(&last(&fake)), json!({"decision": "once"}));
    }

    #[test]
    fn creates_a_locked_down_session_in_the_folder() {
        let (fake, client) = fake(200, "{\"data\":{\"id\":\"ses_9\"}}");
        let model = ModelRef::parse("openrouter/inclusionai/ling");
        let id =
            client.create_session("/docs", model.as_ref(), &logic::session_permissions()).unwrap();
        assert_eq!(id, "ses_9");
        let sent = body(&last(&fake));
        assert_eq!(sent["location"], json!({"directory": "/docs"}));
        assert_eq!(sent["model"], json!({"providerID": "openrouter", "id": "inclusionai/ling"}));
        let permissions = sent["permissions"].as_array().unwrap();
        assert_eq!(permissions[0], json!({"action": "shell", "resource": "*", "effect": "deny"}));
        assert!(permissions.contains(&json!({"action": "edit", "resource": "*", "effect": "ask"})));
        client.create_session("/docs", None, &[]).unwrap();
        assert!(body(&last(&fake)).get("model").is_none());
    }

    #[test]
    fn builds_the_other_endpoints() {
        let (fake, client) = fake(204, "");
        client.set_instructions("ses_1", "mdow", "Be brief.").unwrap();
        assert_eq!(last(&fake).method, "PUT");
        assert_eq!(last(&fake).path, "/api/experimental/session/ses_1/instructions/entries/mdow");
        client.interrupt("ses_1").unwrap();
        assert_eq!(last(&fake).path, "/api/session/ses_1/interrupt");
        let model = ModelRef { provider_id: "opencode".to_string(), id: "m".to_string() };
        client.switch_model("ses_1", &model).unwrap();
        assert_eq!(body(&last(&fake)), json!({"model": {"providerID": "opencode", "id": "m"}}));
        let events = client.event_request("/docs");
        assert_eq!(events.path, "/api/event?location%5Bdirectory%5D=%2Fdocs");
        assert_eq!(events.header("accept"), Some("text/event-stream"));
    }

    #[test]
    fn decodes_models_and_the_default() {
        let (_, client) = fake(
            200,
            "{\"data\":[{\"id\":\"a\",\"providerID\":\"p\",\"name\":\"A\",\"status\":\"active\",\"cost\":[]}]}",
        );
        assert_eq!(
            client.list_models("/").unwrap(),
            [ModelInfo {
                id: "a".to_string(),
                provider_id: "p".to_string(),
                name: Some("A".to_string()),
                status: Some("active".to_string()),
            }]
        );
        let (_, empty) = fake(200, "{\"data\":null}");
        assert_eq!(empty.default_model("/").unwrap(), None);
    }

    #[test]
    fn splits_model_values_on_the_first_slash() {
        assert_eq!(
            ModelRef::parse("openrouter/inclusionai/ling"),
            Some(ModelRef {
                provider_id: "openrouter".to_string(),
                id: "inclusionai/ling".to_string()
            })
        );
        assert_eq!(ModelRef::parse("no-slash"), None);
        assert_eq!(ModelRef::parse("/x"), None);
        assert_eq!(ModelRef::parse("x/"), None);
    }

    #[test]
    fn reads_chunked_and_sized_responses() {
        let raw = "HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6;x=y\r\n world\r\n0\r\n\r\n";
        let mut reader = Cursor::new(raw.as_bytes().to_vec());
        let head = read_head(&mut reader).unwrap();
        assert_eq!(head.status, 200);
        assert!(head.chunked);
        assert_eq!(read_body(reader, &head).unwrap(), b"hello world");

        let raw = "HTTP/1.1 404 Not Found\r\ncontent-length: 3\r\n\r\nabcdef";
        let mut reader = Cursor::new(raw.as_bytes().to_vec());
        let head = read_head(&mut reader).unwrap();
        assert_eq!(head.status, 404);
        assert_eq!(read_body(reader, &head).unwrap(), b"abc");
    }

    #[test]
    fn talks_to_a_real_socket_and_cancels_a_stream() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            while request.read_line(&mut line).unwrap() > 2 {
                line.clear();
            }
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n\
                      2e\r\ndata: {\"type\":\"session.idle\",\"data\":{\"a\":1}}\n\n\r\n",
                )
                .unwrap();
            // Holds the connection open; the client has to cancel it.
            std::thread::sleep(Duration::from_secs(30));
        });
        let client = Client::new(&format!("http://127.0.0.1:{port}"), "p");
        let cancel = Arc::new(StreamCancel::default());
        let canceller = cancel.clone();
        let mut items = Vec::new();
        client.stream_events("/docs", &cancel, |item| {
            let is_event = matches!(item, StreamItem::Event(_));
            items.push(item);
            if is_event {
                let canceller = canceller.clone();
                std::thread::spawn(move || canceller.cancel());
            }
            true
        });
        assert!(matches!(items[0], StreamItem::Opened { reconnected: false }));
        let StreamItem::Event(event) = &items[1] else { panic!("expected an event") };
        assert_eq!(event.kind, "session.idle");
    }
}
