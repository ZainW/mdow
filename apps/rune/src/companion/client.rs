//! A small client for the OpenCode 2 HTTP API (`opencode serve`), ported from the Swift app's
//! `OpencodeClient.swift`. The published SDK still targets the pre-2.0 event names, so Mdow
//! speaks to the endpoints it needs directly.

use super::events::{OpencodeEvent, SseParser};
use serde_json::{Map, Value, json};
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
        if provider.is_empty() || id.is_empty() {
            return None;
        }
        Some(ModelRef { provider_id: provider.into(), id: id.into() })
    }

    pub fn value(&self) -> String {
        format!("{}/{}", self.provider_id, self.id)
    }

    fn json(&self) -> Value {
        json!({ "providerID": self.provider_id, "id": self.id })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    pub provider_id: String,
    pub name: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderInfo {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionRule {
    pub action: String,
    pub resource: String,
    /// `allow`, `deny` or `ask`.
    pub effect: String,
}

impl PermissionRule {
    pub fn new(action: &str, resource: &str, effect: &str) -> PermissionRule {
        PermissionRule { action: action.into(), resource: resource.into(), effect: effect.into() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionInfo {
    pub id: String,
    pub outcome: Option<String>,
    /// When the session last went idle, in milliseconds since 1970.
    pub idle: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionReply {
    Once,
    /// Never sent: it would save the rule in OpenCode's global config.
    #[allow(dead_code)]
    Always,
    Reject,
}

impl PermissionReply {
    fn as_str(self) -> &'static str {
        match self {
            PermissionReply::Once => "once",
            PermissionReply::Always => "always",
            PermissionReply::Reject => "reject",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpError {
    /// 0 when the request never got an answer.
    pub status: u16,
    pub message: String,
}

impl HttpError {
    pub fn new(status: u16, message: impl Into<String>) -> HttpError {
        HttpError { status, message: message.into() }
    }

    /// The server's own message when the body is a JSON error, else the start of the body.
    pub fn from_body(status: u16, body: &[u8]) -> HttpError {
        if let Ok(json) = serde_json::from_slice::<Value>(body) {
            let message = json
                .pointer("/error/message")
                .and_then(Value::as_str)
                .or_else(|| json.get("message").and_then(Value::as_str));
            if let Some(m) = message
                && !m.is_empty()
            {
                return HttpError::new(status, m);
            }
        }
        let text = String::from_utf8_lossy(body);
        let text = text.trim();
        if text.is_empty() {
            HttpError::new(status, format!("OpenCode request failed ({status})"))
        } else {
            HttpError::new(status, text.chars().take(200).collect::<String>())
        }
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// One request, as the transport sends it.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub method: &'static str,
    /// The path and query after the server's address.
    pub path: String,
    pub body: Option<Value>,
}

/// Sends requests; tests swap in a fake.
pub trait Transport: Send + Sync {
    fn send(
        &self,
        url: &str,
        authorization: &str,
        request: &Request,
    ) -> Result<(u16, Vec<u8>), String>;
}

/// The real transport, over loopback HTTP.
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    pub fn new() -> UreqTransport {
        UreqTransport { agent: agent(Some(Duration::from_secs(60))) }
    }
}

fn agent(timeout: Option<Duration>) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .proxy(None)
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_global(timeout)
        .build();
    ureq::Agent::new_with_config(config)
}

impl Transport for UreqTransport {
    fn send(
        &self,
        url: &str,
        authorization: &str,
        request: &Request,
    ) -> Result<(u16, Vec<u8>), String> {
        let result = match (request.method, &request.body) {
            ("GET", _) => self.agent.get(url).header("authorization", authorization).call(),
            (method, body) => {
                let builder = match method {
                    "PUT" => self.agent.put(url),
                    _ => self.agent.post(url),
                }
                .header("authorization", authorization);
                match body {
                    Some(body) => {
                        builder.header("content-type", "application/json").send(body.to_string())
                    }
                    None => builder.send_empty(),
                }
            }
        };
        let response = result.map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        let body = response
            .into_body()
            .with_config()
            .limit(64 * 1024 * 1024)
            .read_to_vec()
            .map_err(|e| e.to_string())?;
        Ok((status, body))
    }
}

/// Escapes one path segment or query value, like `encodeURIComponent`.
pub fn encode_component(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn location_query(directory: &str) -> String {
    format!("location%5Bdirectory%5D={}", encode_component(directory))
}

/// HTTP Basic credentials for the private server.
pub fn basic_auth(password: &str) -> String {
    format!("Basic {}", base64(format!("opencode:{password}").as_bytes()))
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[derive(Clone)]
pub struct Client {
    pub base: String,
    pub authorization: String,
    transport: Arc<dyn Transport>,
}

impl Client {
    pub fn new(base: &str, password: &str, transport: Arc<dyn Transport>) -> Client {
        Client {
            base: base.trim_end_matches('/').to_string(),
            authorization: basic_auth(password),
            transport,
        }
    }

    fn send(
        &self,
        method: &'static str,
        path: String,
        body: Option<Value>,
    ) -> Result<Vec<u8>, HttpError> {
        let request = Request { method, path, body };
        let url = format!("{}{}", self.base, request.path);
        let (status, data) = self
            .transport
            .send(&url, &self.authorization, &request)
            .map_err(|e| HttpError::new(0, e))?;
        if !(200..300).contains(&status) {
            return Err(HttpError::from_body(status, &data));
        }
        Ok(data)
    }

    /// The `data` of a `{ data }` envelope.
    fn get(&self, path: String) -> Result<Value, HttpError> {
        let data = self.send("GET", path, None)?;
        let json: Value = serde_json::from_slice(&data)
            .map_err(|e| HttpError::new(200, format!("Unexpected answer from OpenCode: {e}")))?;
        Ok(json.get("data").cloned().unwrap_or(Value::Null))
    }

    pub fn list_models(&self, directory: &str) -> Result<Vec<ModelInfo>, HttpError> {
        let data = self.get(format!("/api/model?{}", location_query(directory)))?;
        Ok(data
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|m| {
                        Some(ModelInfo {
                            id: m.get("id")?.as_str()?.to_string(),
                            provider_id: m.get("providerID")?.as_str()?.to_string(),
                            name: m.get("name").and_then(Value::as_str).map(Into::into),
                            status: m.get("status").and_then(Value::as_str).map(Into::into),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    pub fn list_providers(&self, directory: &str) -> Result<Vec<ProviderInfo>, HttpError> {
        let data = self.get(format!("/api/provider?{}", location_query(directory)))?;
        Ok(data
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|p| {
                        Some(ProviderInfo {
                            id: p.get("id")?.as_str()?.to_string(),
                            name: p.get("name").and_then(Value::as_str).map(Into::into),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    pub fn default_model(&self, directory: &str) -> Result<Option<ModelRef>, HttpError> {
        let data = self.get(format!("/api/model/default?{}", location_query(directory)))?;
        Ok((|| {
            Some(ModelRef {
                provider_id: data.get("providerID")?.as_str()?.to_string(),
                id: data.get("id")?.as_str()?.to_string(),
            })
        })())
    }

    pub fn session_body(
        directory: &str,
        model: Option<&ModelRef>,
        permissions: &[PermissionRule],
    ) -> Value {
        let mut body = Map::new();
        body.insert("location".into(), json!({ "directory": directory }));
        body.insert(
            "permissions".into(),
            Value::Array(
                permissions.iter().map(|p| json!({ "action": p.action, "resource": p.resource, "effect": p.effect })).collect(),
            ),
        );
        if let Some(model) = model {
            body.insert("model".into(), model.json());
        }
        Value::Object(body)
    }

    pub fn create_session(
        &self,
        directory: &str,
        model: Option<&ModelRef>,
        permissions: &[PermissionRule],
    ) -> Result<String, HttpError> {
        let data = self.send(
            "POST",
            "/api/session".into(),
            Some(Self::session_body(directory, model, permissions)),
        )?;
        let json: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
        json.pointer("/data/id")
            .and_then(Value::as_str)
            .map(Into::into)
            .ok_or_else(|| HttpError::new(200, "OpenCode did not create a session."))
    }

    pub fn set_instructions(
        &self,
        session_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), HttpError> {
        let path = format!(
            "/api/experimental/session/{session_id}/instructions/entries/{}",
            encode_component(key)
        );
        self.send("PUT", path, Some(json!({ "value": value }))).map(drop)
    }

    /// Whether the session has finished its latest run, and how.
    pub fn get_session(&self, session_id: &str) -> Result<SessionInfo, HttpError> {
        let data = self.get(format!("/api/session/{session_id}"))?;
        Ok(SessionInfo {
            id: data.get("id").and_then(Value::as_str).unwrap_or(session_id).to_string(),
            outcome: data.get("outcome").and_then(Value::as_str).map(Into::into),
            idle: data.pointer("/time/idle").and_then(Value::as_f64),
        })
    }

    pub fn switch_model(&self, session_id: &str, model: &ModelRef) -> Result<(), HttpError> {
        self.send(
            "POST",
            format!("/api/session/{session_id}/model"),
            Some(json!({ "model": model.json() })),
        )
        .map(drop)
    }

    pub fn prompt(&self, session_id: &str, text: &str) -> Result<(), HttpError> {
        self.send(
            "POST",
            format!("/api/session/{session_id}/prompt"),
            Some(json!({ "text": text })),
        )
        .map(drop)
    }

    pub fn interrupt(&self, session_id: &str) -> Result<(), HttpError> {
        self.send("POST", format!("/api/session/{session_id}/interrupt"), None).map(drop)
    }

    pub fn reply_permission(
        &self,
        session_id: &str,
        request_id: &str,
        decision: PermissionReply,
        message: Option<&str>,
    ) -> Result<(), HttpError> {
        let mut body = Map::new();
        body.insert("decision".into(), Value::String(decision.as_str().into()));
        if let Some(m) = message.filter(|m| !m.is_empty()) {
            body.insert("message".into(), Value::String(m.into()));
        }
        self.send(
            "POST",
            format!("/api/session/{session_id}/permission/{request_id}/reply"),
            Some(Value::Object(body)),
        )
        .map(drop)
    }

    pub fn event_url(&self, directory: &str) -> String {
        format!("{}/api/event?{}", self.base, location_query(directory))
    }
}

/// Item from the event stream: it (re)connected, or an event arrived.
#[derive(Clone, Debug, PartialEq)]
pub enum StreamItem {
    /// `reconnected` is false the first time.
    Opened {
        reconnected: bool,
    },
    Event(OpencodeEvent),
}

/// Streams server events for a folder until `cancel` is set, reconnecting with backoff when
/// the connection drops. Returns an error only for a 401. Blocks; run it on its own thread.
pub fn stream_events(
    client: &Client,
    directory: &str,
    cancel: &AtomicBool,
    mut on: impl FnMut(StreamItem),
) -> Result<(), HttpError> {
    let agent = agent(None);
    let url = client.event_url(directory);
    let mut attempt: u32 = 0;
    let mut opened = false;
    while !cancel.load(Ordering::Relaxed) {
        let response = agent
            .get(&url)
            .header("authorization", &client.authorization)
            .header("accept", "text/event-stream")
            .call();
        match response {
            Ok(response) if response.status().as_u16() == 401 => {
                return Err(HttpError::new(401, "OpenCode refused the companion's password."));
            }
            Ok(response) if response.status().as_u16() == 200 => {
                attempt = 0;
                on(StreamItem::Opened { reconnected: opened });
                opened = true;
                let mut reader = BufReader::new(response.into_body().into_reader());
                let mut parser = SseParser::default();
                let mut line: Vec<u8> = Vec::new();
                loop {
                    if cancel.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    line.clear();
                    match reader.read_until(b'\n', &mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            while line.last() == Some(&b'\n') {
                                line.pop();
                            }
                            if let Some(payload) = parser.feed_line(&String::from_utf8_lossy(&line))
                                && let Some(event) = OpencodeEvent::parse(&payload)
                            {
                                on(StreamItem::Event(event));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        attempt += 1;
        let delay = (250u64 << attempt.min(5)).min(5_000);
        let until = std::time::Instant::now() + Duration::from_millis(delay);
        while std::time::Instant::now() < until && !cancel.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::companion::logic;
    use std::sync::Mutex;

    /// Records requests and answers with a canned response.
    struct Fake {
        requests: Mutex<Vec<(String, String, Request)>>,
        status: u16,
        body: Vec<u8>,
    }

    impl Fake {
        fn new(status: u16, body: &str) -> Arc<Fake> {
            Arc::new(Fake {
                requests: Mutex::new(Vec::new()),
                status,
                body: body.as_bytes().to_vec(),
            })
        }
        fn last(&self) -> (String, String, Request) {
            self.requests.lock().unwrap().last().cloned().unwrap()
        }
    }

    impl Transport for Fake {
        fn send(
            &self,
            url: &str,
            authorization: &str,
            request: &Request,
        ) -> Result<(u16, Vec<u8>), String> {
            self.requests.lock().unwrap().push((url.into(), authorization.into(), request.clone()));
            Ok((self.status, self.body.clone()))
        }
    }

    fn client(fake: &Arc<Fake>, base: &str, password: &str) -> Client {
        Client::new(base, password, fake.clone())
    }

    #[test]
    fn sends_basic_auth_and_scopes_requests_to_a_folder() {
        let fake = Fake::new(200, "{\"data\":[]}");
        let c = client(&fake, "http://127.0.0.1:9", "secret");
        c.list_models("/my docs").unwrap();
        let (url, auth, _) = fake.last();
        assert_eq!(url, "http://127.0.0.1:9/api/model?location%5Bdirectory%5D=%2Fmy%20docs");
        assert_eq!(auth, "Basic b3BlbmNvZGU6c2VjcmV0");
    }

    #[test]
    fn surfaces_the_server_error_message() {
        let fake = Fake::new(400, "{\"error\":{\"message\":\"Unknown model\"}}");
        let c = client(&fake, "http://x", "p");
        assert_eq!(c.prompt("ses_1", "hi"), Err(HttpError::new(400, "Unknown model")));
        assert_eq!(fake.last().2.body, Some(json!({ "text": "hi" })));
        assert_eq!(HttpError::from_body(500, b"").message, "OpenCode request failed (500)");
    }

    #[test]
    fn replies_to_permissions_with_the_decision_and_an_optional_message() {
        let fake = Fake::new(204, "");
        let c = client(&fake, "http://x", "p");
        c.reply_permission("ses_1", "per_1", PermissionReply::Reject, Some("No thanks")).unwrap();
        let (url, _, request) = fake.last();
        assert_eq!(url, "http://x/api/session/ses_1/permission/per_1/reply");
        assert_eq!(request.method, "POST");
        assert_eq!(request.body, Some(json!({ "decision": "reject", "message": "No thanks" })));
        c.reply_permission("ses_1", "per_2", PermissionReply::Once, None).unwrap();
        assert_eq!(fake.last().2.body, Some(json!({ "decision": "once" })));
    }

    #[test]
    fn creates_a_locked_down_session_in_the_folder() {
        let fake = Fake::new(200, "{\"data\":{\"id\":\"ses_9\"}}");
        let c = client(&fake, "http://x", "p");
        let model = ModelRef::parse("openrouter/inclusionai/ling").unwrap();
        let id = c.create_session("/docs", Some(&model), &logic::session_permissions()).unwrap();
        assert_eq!(id, "ses_9");
        let body = fake.last().2.body.unwrap();
        assert_eq!(body["location"], json!({ "directory": "/docs" }));
        assert_eq!(body["model"], json!({ "providerID": "openrouter", "id": "inclusionai/ling" }));
        assert_eq!(
            body["permissions"][0],
            json!({ "action": "shell", "resource": "*", "effect": "deny" })
        );
        assert!(
            body["permissions"]
                .as_array()
                .unwrap()
                .contains(&json!({ "action": "edit", "resource": "*", "effect": "ask" }))
        );
        c.create_session("/docs", None, &[]).unwrap();
        assert!(fake.last().2.body.unwrap().get("model").is_none());
    }

    #[test]
    fn builds_the_other_endpoints() {
        let fake = Fake::new(204, "");
        let c = client(&fake, "http://x", "p");
        c.set_instructions("ses_1", "mdow", "Be brief.").unwrap();
        let (url, _, request) = fake.last();
        assert_eq!(request.method, "PUT");
        assert_eq!(url, "http://x/api/experimental/session/ses_1/instructions/entries/mdow");
        c.interrupt("ses_1").unwrap();
        assert_eq!(fake.last().2.path, "/api/session/ses_1/interrupt");
        c.switch_model("ses_1", &ModelRef { provider_id: "opencode".into(), id: "m".into() })
            .unwrap();
        assert_eq!(
            fake.last().2.body,
            Some(json!({ "model": { "providerID": "opencode", "id": "m" } }))
        );
        assert_eq!(c.event_url("/docs"), "http://x/api/event?location%5Bdirectory%5D=%2Fdocs");
    }

    #[test]
    fn decodes_models_and_the_default() {
        let fake = Fake::new(
            200,
            "{\"data\":[{\"id\":\"a\",\"providerID\":\"p\",\"name\":\"A\",\"status\":\"active\",\"cost\":[]}]}",
        );
        let listed = client(&fake, "http://x", "p").list_models("/").unwrap();
        assert_eq!(
            listed,
            vec![ModelInfo {
                id: "a".into(),
                provider_id: "p".into(),
                name: Some("A".into()),
                status: Some("active".into())
            }]
        );
        let empty = Fake::new(200, "{\"data\":null}");
        assert_eq!(client(&empty, "http://x", "p").default_model("/").unwrap(), None);
        let session = Fake::new(
            200,
            "{\"data\":{\"id\":\"s\",\"outcome\":\"succeeded\",\"time\":{\"created\":1,\"updated\":2,\"idle\":3}}}",
        );
        let info = client(&session, "http://x", "p").get_session("s").unwrap();
        assert_eq!(
            info,
            SessionInfo { id: "s".into(), outcome: Some("succeeded".into()), idle: Some(3.0) }
        );
    }

    #[test]
    fn splits_model_values_on_the_first_slash() {
        assert_eq!(
            ModelRef::parse("openrouter/inclusionai/ling"),
            Some(ModelRef { provider_id: "openrouter".into(), id: "inclusionai/ling".into() })
        );
        assert_eq!(ModelRef::parse("no-slash"), None);
        assert_eq!(ModelRef::parse("/x"), None);
        assert_eq!(ModelRef::parse("x/"), None);
    }

    #[test]
    fn encodes_base64() {
        assert_eq!(base64(b"opencode:pw"), "b3BlbmNvZGU6cHc=");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"abc"), "YWJj");
    }
}
