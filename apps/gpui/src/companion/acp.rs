//! Agent Client Protocol wire format: newline-delimited JSON-RPC 2.0 over stdio.
//!
//! Hand-rolled on `serde_json` like Electron's `acp-client.ts`: the companion uses a small,
//! read-only slice of the protocol, and parsing stays lenient so older and newer agents that
//! disagree on optional fields still work.

use super::types::{
    ModelOption, ModelState, PermissionKind, PermissionOption, PermissionRequest, ToolState,
    ToolUpdate,
};
use serde_json::{Map, Value, json};
use std::path::Path;

pub const PROTOCOL_VERSION: u64 = 1;
/// JSON-RPC "method not found".
pub const METHOD_NOT_FOUND: i64 = -32601;
/// ACP's generic server error, also used for "authentication required".
pub const SERVER_ERROR: i64 = -32000;

#[derive(Debug, Clone, PartialEq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl RpcError {
    /// ACP agents reject `session/new` with `auth_required` (-32000) until the user signs in.
    pub fn is_auth_required(&self) -> bool {
        let message = self.message.to_lowercase();
        let data = self
            .data
            .as_ref()
            .map(|data| data.to_string().to_lowercase())
            .unwrap_or_default();
        message.contains("auth") || message.contains("login") || data.contains("auth_required")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    Response {
        id: Value,
        result: Result<Value, RpcError>,
    },
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    Notification {
        method: String,
        params: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedMessage;

pub fn parse_line(line: &str) -> Result<Incoming, MalformedMessage> {
    let value: Value = serde_json::from_str(line.trim()).map_err(|_| MalformedMessage)?;
    let object = value.as_object().ok_or(MalformedMessage)?;
    let id = object.get("id").filter(|id| !id.is_null()).cloned();
    if let Some(id) = id.clone()
        && (object.contains_key("result") || object.contains_key("error"))
    {
        let result = match object.get("error").filter(|error| !error.is_null()) {
            Some(error) => Err(RpcError {
                code: error.get("code").and_then(Value::as_i64).unwrap_or(0),
                message: error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown agent error")
                    .to_owned(),
                data: error.get("data").cloned(),
            }),
            None => Ok(object.get("result").cloned().unwrap_or(Value::Null)),
        };
        return Ok(Incoming::Response { id, result });
    }
    let method = object
        .get("method")
        .and_then(Value::as_str)
        .ok_or(MalformedMessage)?
        .to_owned();
    let params = object.get("params").cloned().unwrap_or(Value::Null);
    Ok(match id {
        Some(id) => Incoming::Request { id, method, params },
        None => Incoming::Notification { method, params },
    })
}

fn line(value: Value) -> String {
    let mut text = value.to_string();
    text.push('\n');
    text
}

pub fn encode_request(id: u64, method: &str, params: Value) -> String {
    line(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
}

pub fn encode_notification(method: &str, params: Value) -> String {
    line(json!({ "jsonrpc": "2.0", "method": method, "params": params }))
}

pub fn encode_result(id: &Value, result: Value) -> String {
    line(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

pub fn encode_error(id: &Value, code: i64, message: &str) -> String {
    line(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    }))
}

pub fn initialize_params(version: &str) -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "clientInfo": { "name": "mdow", "title": "Mdow", "version": version },
        "clientCapabilities": {
            "fs": { "readTextFile": false, "writeTextFile": false },
            "terminal": false,
        },
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServer {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

pub fn new_session_params(cwd: &Path, mcp_servers: &[McpServer]) -> Value {
    json!({
        "cwd": cwd.to_string_lossy(),
        "mcpServers": mcp_servers
            .iter()
            .map(|server| json!({
                "name": server.name,
                "command": server.command,
                "args": server.args,
                "env": [],
            }))
            .collect::<Vec<_>>(),
    })
}

pub fn prompt_params(session_id: &str, text: &str) -> Value {
    json!({ "sessionId": session_id, "prompt": [{ "type": "text", "text": text }] })
}

pub fn cancel_params(session_id: &str) -> Value {
    json!({ "sessionId": session_id })
}

pub fn permission_selected(option_id: &str) -> Value {
    json!({ "outcome": { "outcome": "selected", "optionId": option_id } })
}

pub fn permission_cancelled() -> Value {
    json!({ "outcome": { "outcome": "cancelled" } })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValue {
    pub value: String,
    pub name: String,
    pub description: Option<String>,
}

/// `session/new`'s `configOptions` entry (the model selector lives here in current ACP).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigOption {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
    pub kind: String,
    pub current_value: Option<String>,
    pub options: Vec<ConfigValue>,
}

/// The older unstable `models` field (`availableModels` + `currentModelId`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionModels {
    pub available: Vec<ConfigValue>,
    pub current: Option<String>,
}

/// What an agent exposed about configuration for the live session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionConfig {
    pub config_options: Vec<ConfigOption>,
    pub models: Option<SessionModels>,
}

fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

pub fn parse_config_options(value: Option<&Value>) -> Vec<ConfigOption> {
    let Some(items) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|candidate| {
            let id = string(candidate, "id")?;
            let name = string(candidate, "name")?;
            let kind = string(candidate, "type")?;
            let current_value = match candidate.get("currentValue")? {
                Value::String(value) => Some(value.clone()),
                Value::Bool(_) => None,
                _ => return None,
            };
            let options = candidate
                .get("options")
                .and_then(Value::as_array)
                .map(|options| options.iter().filter_map(parse_config_value).collect())
                .unwrap_or_default();
            Some(ConfigOption {
                id,
                name,
                category: string(candidate, "category"),
                kind,
                current_value,
                options,
            })
        })
        .collect()
}

fn parse_config_value(option: &Value) -> Option<ConfigValue> {
    let value = string(option, "value")?;
    Some(ConfigValue {
        name: string(option, "name").unwrap_or_else(|| value.clone()),
        description: string(option, "description"),
        value,
    })
}

pub fn parse_session_models(value: Option<&Value>) -> Option<SessionModels> {
    let value = value?;
    let available = value
        .get("availableModels")?
        .as_array()?
        .iter()
        .filter_map(|model| {
            let id = string(model, "modelId")?;
            Some(ConfigValue {
                name: string(model, "name").unwrap_or_else(|| id.clone()),
                description: string(model, "description"),
                value: id,
            })
        })
        .collect();
    Some(SessionModels {
        available,
        current: string(value, "currentModelId"),
    })
}

impl SessionConfig {
    pub fn from_result(result: &Value) -> Self {
        Self {
            config_options: parse_config_options(result.get("configOptions")),
            models: parse_session_models(result.get("models")),
        }
    }

    fn model_option(&self) -> Option<&ConfigOption> {
        self.config_options
            .iter()
            .find(|option| option.category.as_deref() == Some("model"))
            .or_else(|| {
                self.config_options
                    .iter()
                    .find(|option| option.id == "model")
            })
    }

    /// The request that changes the model, or `None` when `value` is not live.
    pub fn model_request(&self, session_id: &str, value: &str) -> Option<(&'static str, Value)> {
        if !self
            .model_state(true)
            .options
            .iter()
            .any(|option| option.value == value)
        {
            return None;
        }
        if let Some(option) = self.model_option().filter(|option| option.kind == "select") {
            return Some((
                "session/set_config_option",
                json!({ "sessionId": session_id, "configId": option.id, "value": value }),
            ));
        }
        self.models.as_ref().map(|_| {
            (
                "session/set_model",
                json!({ "sessionId": session_id, "modelId": value }),
            )
        })
    }

    /// Records the answer to a model change request.
    pub fn apply_model_result(&mut self, method: &str, value: &str, result: &Value) {
        if method == "session/set_config_option" {
            self.config_options = parse_config_options(result.get("configOptions"));
        } else if let Some(models) = self.models.as_mut() {
            models.current = Some(value.to_owned());
        }
    }

    pub fn model_state(&self, has_session: bool) -> ModelState {
        if !has_session {
            return ModelState::not_started();
        }
        let (values, current) = match self.model_option() {
            Some(option) if option.kind == "select" => {
                (option.options.clone(), option.current_value.clone())
            }
            Some(_) => (Vec::new(), None),
            None => match self.models.as_ref() {
                Some(models) => (models.available.clone(), models.current.clone()),
                None => {
                    return ModelState {
                        options: Vec::new(),
                        current_value: None,
                        stale: false,
                        unavailable_reason: Some(
                            "The current provider does not expose model selection".into(),
                        ),
                    };
                }
            },
        };
        let options = values
            .into_iter()
            .map(|value| ModelOption {
                group: model_group(&value.value),
                value: value.value,
                name: value.name,
                description: value.description,
            })
            .collect::<Vec<_>>();
        let current_value =
            current.filter(|current| options.iter().any(|option| &option.value == current));
        let unavailable_reason = options
            .is_empty()
            .then(|| "No models are live in this session".to_owned());
        ModelState {
            options,
            current_value,
            stale: false,
            unavailable_reason,
        }
    }
}

/// Electron groups OpenCode's models by subscription; other prefixes keep their own name.
pub fn model_group(value: &str) -> Option<String> {
    let (prefix, _) = value.split_once('/')?;
    Some(match prefix {
        "openai" => "ChatGPT subscription".into(),
        "opencode" => "OpenCode Zen".into(),
        "opencode-go" => "OpenCode Go".into(),
        other => other.to_owned(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextChannel {
    Message,
    Thinking,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionEvent {
    ConfigOptions(Vec<ConfigOption>),
    Tool(ToolUpdate),
    Text { channel: TextChannel, text: String },
    Ignored,
}

fn text_from_content(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Object(object)) => object
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|block| block.get("text").and_then(Value::as_str))
            .collect(),
        _ => String::new(),
    }
}

fn pretty(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    }
}

/// Classifies `session/update` params. `fallback_tool_id` names tool calls without an id.
pub fn classify_session_update(params: &Value, fallback_tool_id: &str) -> SessionEvent {
    let update = params.get("update").unwrap_or(params);
    let Some(object) = update.as_object() else {
        return SessionEvent::Ignored;
    };
    let kind = object
        .get("sessionUpdate")
        .or_else(|| object.get("type"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    match kind {
        "config_option_update" => {
            SessionEvent::ConfigOptions(parse_config_options(object.get("configOptions")))
        }
        "tool_call" | "tool_call_update" => {
            SessionEvent::Tool(tool_update(kind, object, fallback_tool_id))
        }
        "agent_thought_chunk" => text_event(
            TextChannel::Thinking,
            text_from_content(object.get("content")),
        ),
        "agent_message_chunk" | "message" => {
            let text = match object.get("content") {
                Some(content) => text_from_content(Some(content)),
                None => object
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            };
            text_event(TextChannel::Message, text)
        }
        "user_message_chunk" | "plan" | "available_commands_update" | "current_mode_update" => {
            SessionEvent::Ignored
        }
        _ => match object.get("text").and_then(Value::as_str) {
            Some(text) => text_event(TextChannel::Message, text.to_owned()),
            None => SessionEvent::Ignored,
        },
    }
}

fn text_event(channel: TextChannel, text: String) -> SessionEvent {
    if text.is_empty() {
        SessionEvent::Ignored
    } else {
        SessionEvent::Text { channel, text }
    }
}

fn tool_update(kind: &str, object: &Map<String, Value>, fallback_id: &str) -> ToolUpdate {
    let text = |key: &str| {
        object
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let tool_call_id = text("toolCallId")
        .or_else(|| text("id"))
        .unwrap_or_else(|| fallback_id.to_owned());
    let name = text("title").or_else(|| {
        (kind == "tool_call")
            .then(|| text("kind").or_else(|| text("name")))
            .flatten()
            .or_else(|| (kind == "tool_call").then(|| "tool".to_owned()))
    });
    let status = text("status").unwrap_or_default();
    let state = match status.as_str() {
        "completed" | "success" => ToolState::Completed,
        "failed" | "error" => ToolState::Error,
        "cancelled" => ToolState::Cancelled,
        "" if kind == "tool_call" => ToolState::Pending,
        _ => ToolState::Running,
    };
    let input = object.get("rawInput").map(pretty);
    let output = object
        .get("rawOutput")
        .or_else(|| object.get("content"))
        .map(pretty);
    ToolUpdate {
        tool_call_id,
        name,
        state,
        input,
        output,
    }
}

pub fn parse_permission_request(params: &Value, request_key: String) -> PermissionRequest {
    let tool_call = params.get("toolCall");
    let title = tool_call
        .and_then(|tool| tool.get("title"))
        .and_then(Value::as_str)
        .filter(|title| !title.is_empty())
        .unwrap_or("The agent wants to run a tool")
        .to_owned();
    let detail = tool_call
        .and_then(|tool| tool.get("rawInput"))
        .map(pretty)
        .filter(|detail| !detail.is_empty() && detail != "{}");
    let options = params
        .get("options")
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| {
                    let option_id = string(option, "optionId")?;
                    Some(PermissionOption {
                        name: string(option, "name").unwrap_or_else(|| option_id.clone()),
                        kind: PermissionKind::from_wire(
                            option
                                .get("kind")
                                .and_then(Value::as_str)
                                .unwrap_or_default(),
                        ),
                        option_id,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    PermissionRequest {
        request_key,
        title,
        detail,
        options,
    }
}

/// A stable string key for a JSON-RPC id so it can cross into UI state.
pub fn id_key(id: &Value) -> String {
    id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_config(current: &str) -> Value {
        json!([{
            "id": "model",
            "name": "Model",
            "category": "model",
            "type": "select",
            "currentValue": current,
            "options": [
                { "value": "openai/gpt-5.4", "name": "GPT-5.4" },
                { "value": "opencode/claude-sonnet-4-5", "name": "Claude Sonnet 4.5" },
                { "value": "opencode-go/kimi-k2.5", "name": "Kimi K2.5" },
            ],
        }])
    }

    #[test]
    fn requests_and_notifications_encode_as_single_json_lines() {
        let request = encode_request(7, "session/new", json!({ "cwd": "/tmp" }));
        assert!(request.ends_with('\n'));
        assert_eq!(request.matches('\n').count(), 1);
        let value: Value = serde_json::from_str(&request).unwrap();
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["id"], 7);
        assert_eq!(value["method"], "session/new");

        let notification: Value =
            serde_json::from_str(&encode_notification("session/cancel", cancel_params("s1")))
                .unwrap();
        assert!(notification.get("id").is_none());
        assert_eq!(notification["params"]["sessionId"], "s1");
    }

    #[test]
    fn initialize_declares_a_read_only_client() {
        let params = initialize_params("1.2.3");
        assert_eq!(params["protocolVersion"], 1);
        assert_eq!(params["clientInfo"]["version"], "1.2.3");
        assert_eq!(params["clientCapabilities"]["fs"]["readTextFile"], false);
        assert_eq!(params["clientCapabilities"]["fs"]["writeTextFile"], false);
        assert_eq!(params["clientCapabilities"]["terminal"], false);
    }

    #[test]
    fn session_setup_passes_mcp_servers_through() {
        let params = new_session_params(
            Path::new("/tmp/docs"),
            &[McpServer {
                name: "fff".into(),
                command: "/opt/homebrew/bin/fff-mcp".into(),
                args: Vec::new(),
            }],
        );
        assert_eq!(
            params["mcpServers"],
            json!([{ "name": "fff", "command": "/opt/homebrew/bin/fff-mcp", "args": [], "env": [] }])
        );
    }

    #[test]
    fn parses_responses_errors_requests_and_notifications() {
        assert_eq!(
            parse_line(r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#),
            Ok(Incoming::Response {
                id: json!(1),
                result: Ok(json!({ "ok": true })),
            })
        );
        let Ok(Incoming::Response {
            result: Err(error), ..
        }) = parse_line(
            r#"{"jsonrpc":"2.0","id":"a","error":{"code":-32000,"message":"Authentication required"}}"#,
        )
        else {
            panic!("error response should parse");
        };
        assert!(error.is_auth_required());
        assert!(matches!(
            parse_line(r#"{"jsonrpc":"2.0","id":9,"method":"fs/write_text_file","params":{}}"#),
            Ok(Incoming::Request { method, .. }) if method == "fs/write_text_file"
        ));
        assert!(matches!(
            parse_line(r#"{"jsonrpc":"2.0","method":"session/update","params":{}}"#),
            Ok(Incoming::Notification { .. })
        ));
        assert_eq!(parse_line("not json"), Err(MalformedMessage));
        assert_eq!(parse_line("[1,2]"), Err(MalformedMessage));
    }

    #[test]
    fn exposes_the_live_model_configuration() {
        let config = SessionConfig::from_result(&json!({
            "sessionId": "s",
            "configOptions": model_config("opencode/claude-sonnet-4-5"),
        }));
        let state = config.model_state(true);
        assert_eq!(state.options.len(), 3);
        assert_eq!(
            state.current_value.as_deref(),
            Some("opencode/claude-sonnet-4-5")
        );
        assert_eq!(
            state.options[0].group.as_deref(),
            Some("ChatGPT subscription")
        );
        assert_eq!(state.options[2].group.as_deref(), Some("OpenCode Go"));
        assert!(!state.stale);

        let (method, params) = config.model_request("s", "openai/gpt-5.4").unwrap();
        assert_eq!(method, "session/set_config_option");
        assert_eq!(params["configId"], "model");
        assert_eq!(params["value"], "openai/gpt-5.4");
        assert!(
            config
                .model_request("s", "anthropic/claude-opus-4")
                .is_none()
        );
        assert!(config.model_state(false).stale);
    }

    #[test]
    fn falls_back_to_the_legacy_models_field() {
        let mut config = SessionConfig::from_result(&json!({
            "sessionId": "s",
            "models": {
                "availableModels": [
                    { "modelId": "default", "name": "Default" },
                    { "modelId": "opus", "name": "Opus", "description": "Most capable" },
                ],
                "currentModelId": "default",
            },
        }));
        let state = config.model_state(true);
        assert_eq!(state.current_name(), Some("Default"));
        assert_eq!(
            state.options[1].description.as_deref(),
            Some("Most capable")
        );
        let (method, params) = config.model_request("s", "opus").unwrap();
        assert_eq!(method, "session/set_model");
        assert_eq!(params["modelId"], "opus");
        config.apply_model_result(method, "opus", &json!({}));
        assert_eq!(
            config.model_state(true).current_value.as_deref(),
            Some("opus")
        );
    }

    #[test]
    fn providers_without_model_selection_explain_why() {
        let state = SessionConfig::default().model_state(true);
        assert!(state.options.is_empty());
        assert!(!state.is_selectable());
        assert!(
            state
                .unavailable_reason
                .unwrap()
                .contains("does not expose")
        );
    }

    #[test]
    fn classifies_streamed_session_updates() {
        let thought = json!({ "sessionId": "s", "update": {
            "sessionUpdate": "agent_thought_chunk",
            "content": { "type": "text", "text": "Thinking…" },
        }});
        assert_eq!(
            classify_session_update(&thought, "t"),
            SessionEvent::Text {
                channel: TextChannel::Thinking,
                text: "Thinking…".into()
            }
        );
        let chunk = json!({ "update": {
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "Hello" },
        }});
        assert_eq!(
            classify_session_update(&chunk, "t"),
            SessionEvent::Text {
                channel: TextChannel::Message,
                text: "Hello".into()
            }
        );
        let tool = json!({ "update": {
            "sessionUpdate": "tool_call",
            "toolCallId": "tool_1",
            "title": "read",
            "status": "completed",
            "rawInput": { "path": "a.md" },
            "rawOutput": { "result": "ok" },
        }});
        assert_eq!(
            classify_session_update(&tool, "t"),
            SessionEvent::Tool(ToolUpdate {
                tool_call_id: "tool_1".into(),
                name: Some("read".into()),
                state: ToolState::Completed,
                input: Some("{\n  \"path\": \"a.md\"\n}".into()),
                output: Some("{\n  \"result\": \"ok\"\n}".into()),
            })
        );
        let progress = json!({ "update": {
            "sessionUpdate": "tool_call_update",
            "toolCallId": "tool_1",
            "status": "in_progress",
        }});
        let SessionEvent::Tool(progress) = classify_session_update(&progress, "t") else {
            panic!("tool progress should classify");
        };
        assert_eq!(progress.name, None);
        assert_eq!(progress.state, ToolState::Running);
        assert_eq!(
            classify_session_update(&json!({ "update": { "sessionUpdate": "plan" } }), "t"),
            SessionEvent::Ignored
        );
        assert!(matches!(
            classify_session_update(
                &json!({ "update": { "sessionUpdate": "config_option_update", "configOptions": model_config("openai/gpt-5.4") } }),
                "t"
            ),
            SessionEvent::ConfigOptions(options) if options.len() == 1
        ));
    }

    #[test]
    fn permission_requests_map_to_allow_and_reject_options() {
        let request = parse_permission_request(
            &json!({
                "sessionId": "s",
                "toolCall": { "toolCallId": "c1", "title": "Edit notes.md", "rawInput": { "path": "notes.md" } },
                "options": [
                    { "optionId": "always", "name": "Always allow", "kind": "allow_always" },
                    { "optionId": "once", "name": "Allow", "kind": "allow_once" },
                    { "optionId": "no", "name": "Reject", "kind": "reject_once" },
                ],
            }),
            "7".into(),
        );
        assert_eq!(request.title, "Edit notes.md");
        assert!(request.detail.as_deref().unwrap().contains("notes.md"));
        assert_eq!(request.allow_option().unwrap().option_id, "once");
        assert_eq!(request.reject_option().unwrap().option_id, "no");
        assert_eq!(
            permission_selected("once"),
            json!({ "outcome": { "outcome": "selected", "optionId": "once" } })
        );
        assert_eq!(permission_cancelled()["outcome"]["outcome"], "cancelled");
    }

    #[test]
    fn error_replies_carry_the_request_id() {
        let reply: Value =
            serde_json::from_str(&encode_error(&json!(99), SERVER_ERROR, "Refused")).unwrap();
        assert_eq!(reply["id"], 99);
        assert_eq!(reply["error"]["message"], "Refused");
        assert_eq!(id_key(&json!(99)), "99");
        assert_eq!(id_key(&json!("x")), "\"x\"");
    }
}
