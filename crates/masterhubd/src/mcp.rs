//! `masterhubd mcp`: a Model Context Protocol server for AI agents (see `docs/MCP.md`).
//!
//! The agent starts this process and speaks JSON-RPC 2.0 with it over stdin/stdout, one message
//! per line. Every tool is a request to the running service over the local IPC socket, so this
//! process holds no state and never opens a module. It refuses to start, and refuses each tool
//! call, unless the config has `[app] mcp = true` (Settings › AI agents in the app).

mod tools;

use std::io::{BufRead, Write};

use anyhow::{Context, Result, bail};
use masterhub_core::ipc::Request;
use serde_json::{Value, json};

/// Protocol revisions this server speaks; the newest is offered to clients that ask for another.
const PROTOCOL_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "OpenMasterHUB drives a Cooler Master MasterHUB: a keypad of 15 keys \
(index 0..=14, 5 columns × 3 rows, row-major from the top-left), a dial with a round screen, \
knobs (0 bottom-left, 1 bottom-right, 2 top), faders (0..=4, top first) and rollers (left, \
right). The config has profiles (each with keypad pages and folders, a dial, knobs, faders and \
rollers; a profile can inherit controls it does not set from a `base` profile) and setups \
(follow: the profile follows the focused app; manual: the user steps through profiles; hybrid: \
follow until a profile is pinned). Start with get_hub and list_profiles. Every edit is validated \
like the app's and saved to the user's config at once; the hub and the app pick it up live. \
Prefer the structured tools (set_key, set_dial, put_setup, ...) over put_config.";

/// The running service, or a stand-in in tests.
pub trait Hub {
    fn call(&self, request: Request) -> Result<Value>;
}

/// The service over its local socket, one connection per request.
struct Service;

impl Hub for Service {
    fn call(&self, request: Request) -> Result<Value> {
        masterhub_core::ipc::call_once(request)
    }
}

/// Serve on stdin/stdout until the agent closes stdin.
pub fn run() -> Result<()> {
    let hub = Service;
    // Refuse to start when off, so the agent's MCP list shows the server as failed and why.
    check_enabled(&hub)?;
    let mut out = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let line = line.context("reading stdin")?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle_line(&hub, &line) {
            let mut text = serde_json::to_vec(&reply)?;
            text.push(b'\n');
            out.write_all(&text)?;
            out.flush()?;
        }
    }
    Ok(())
}

/// Whether a config's text turns the MCP server on (`[app] mcp = true`).
fn enabled(text: &str) -> bool {
    toml::from_str::<toml::Table>(text).ok().and_then(|t| t.get("app")?.get("mcp")?.as_bool()).unwrap_or(false)
}

fn check_enabled(hub: &dyn Hub) -> Result<()> {
    let text = hub.call(Request::GetConfig).context("asking the masterhubd service for its config")?;
    if !enabled(text.as_str().unwrap_or_default()) {
        bail!("the OpenMasterHUB MCP server is off; turn on Settings › AI agents › Run MCP server in the OpenMasterHUB app");
    }
    Ok(())
}

/// Answer one JSON-RPC line; `None` for notifications and responses, which get no reply.
fn handle_line(hub: &dyn Hub, line: &str) -> Option<Value> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
    };
    let method = msg.get("method").and_then(Value::as_str)?;
    let id = msg.get("id").cloned()?;
    let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
    Some(match method {
        "initialize" => reply(id, initialize(&params)),
        "ping" => reply(id, json!({})),
        "tools/list" => reply(id, json!({ "tools": tools::list() })),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
            if !tools::exists(name) {
                return Some(error(id, -32602, &format!("unknown tool {name:?}")));
            }
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            reply(id, call_tool(hub, name, &args))
        }
        "resources/list" => reply(id, json!({ "resources": [] })),
        "prompts/list" => reply(id, json!({ "prompts": [] })),
        _ => error(id, -32601, &format!("method {method:?} not found")),
    })
}

fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or_default();
    let version = PROTOCOL_VERSIONS.iter().find(|v| **v == asked).unwrap_or(&PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "openmasterhub", "title": "OpenMasterHUB", "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS,
    })
}

/// A tool's result, or its error as a result the agent reads (`isError`).
fn call_tool(hub: &dyn Hub, name: &str, args: &Value) -> Value {
    match check_enabled(hub).and_then(|()| tools::call(hub, name, args)) {
        Ok(v) => {
            let text = match v {
                Value::String(s) => s,
                v => serde_json::to_string_pretty(&v).unwrap_or_default(),
            };
            json!({ "content": [{ "type": "text", "text": text }] })
        }
        Err(e) => json!({ "content": [{ "type": "text", "text": format!("{e:#}") }], "isError": true }),
    }
}

fn reply(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use masterhub_core::config::Config;
    use masterhub_core::ipc::{HubState, ModuleState};

    use super::*;

    /// A service that keeps a config in memory and validates saves as the real one does.
    pub struct FakeHub {
        pub text: Mutex<String>,
        pub state: Mutex<HubState>,
        pub sent: Mutex<Vec<Request>>,
    }

    impl FakeHub {
        pub fn new(text: &str) -> Self {
            Self { text: Mutex::new(text.to_owned()), state: Mutex::default(), sent: Mutex::default() }
        }
        pub fn config(&self) -> Config {
            Config::parse(&self.text.lock().unwrap(), std::path::Path::new("/cfg")).unwrap()
        }
    }

    impl Hub for FakeHub {
        fn call(&self, request: Request) -> Result<Value> {
            self.sent.lock().unwrap().push(request.clone());
            Ok(match request {
                Request::GetConfig => json!(*self.text.lock().unwrap()),
                Request::GetConfigPath => json!("/cfg/config.toml"),
                Request::SetConfig { toml } => {
                    let config = Config::parse_unchecked(&toml, std::path::Path::new("/cfg"))?;
                    crate::actions::validate_config(&config)?;
                    *self.text.lock().unwrap() = toml;
                    Value::Null
                }
                Request::GetState => serde_json::to_value(&*self.state.lock().unwrap())?,
                Request::GetModules => serde_json::to_value(vec![ModuleState {
                    kind: "keypad".into(),
                    serial: "SERIAL-NOT-SHOWN".into(),
                    socket: Some("22".into()),
                    placement: None,
                    firmware: Some("1.0".into()),
                    needs_replug: false,
                }])?,
                Request::GetCapabilities => json!(["setups", "pin"]),
                Request::GetOpenApps => json!(["firefox", "obs"]),
                Request::SetSetup { name } if name == "Nope" => bail!("no setup {name:?}"),
                _ => Value::Null,
            })
        }
    }

    pub const ON: &str = "[app]\nmcp = true\n\n[[profile]]\nname = \"Desktop\"\n\n[[profile.page]]\nname = \"Main\"\n";

    fn rpc(hub: &dyn Hub, msg: Value) -> Option<Value> {
        handle_line(hub, &msg.to_string())
    }

    #[test]
    fn the_toggle_gates_every_call() {
        assert!(enabled(ON));
        assert!(!enabled(""));
        assert!(!enabled("[app]\nmcp = false\n"));
        assert!(!enabled("[app]\nmcp = \"yes\"\n"));
        let off = FakeHub::new("[[profile]]\nname = \"Desktop\"\n");
        assert!(check_enabled(&off).unwrap_err().to_string().contains("Run MCP server"));
        let r = rpc(&off, json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_profiles"}})).unwrap();
        assert_eq!(r["result"]["isError"], true);
        assert!(r["result"]["content"][0]["text"].as_str().unwrap().contains("off"));
        // Nothing but the gate's own read reached the service.
        assert!(off.sent.lock().unwrap().iter().all(|r| *r == Request::GetConfig));
    }

    #[test]
    fn protocol() {
        let hub = FakeHub::new(ON);
        let init = rpc(&hub, json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}})).unwrap();
        assert_eq!(init["id"], 0);
        assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
        assert!(init["result"]["capabilities"]["tools"].is_object());
        let newer = rpc(&hub, json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2099-01-01"}})).unwrap();
        assert_eq!(newer["result"]["protocolVersion"], PROTOCOL_VERSIONS[0]);
        assert_eq!(rpc(&hub, json!({"jsonrpc":"2.0","method":"notifications/initialized"})), None);
        assert_eq!(rpc(&hub, json!({"jsonrpc":"2.0","id":2,"method":"ping"})).unwrap()["result"], json!({}));
        assert_eq!(rpc(&hub, json!({"jsonrpc":"2.0","id":3,"method":"nope"})).unwrap()["error"]["code"], -32601);
        assert_eq!(handle_line(&hub, "{not json").unwrap()["error"]["code"], -32700);
        let unknown = rpc(&hub, json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"reboot"}})).unwrap();
        assert_eq!(unknown["error"]["code"], -32602);

        let list = rpc(&hub, json!({"jsonrpc":"2.0","id":5,"method":"tools/list"})).unwrap();
        let tools = list["result"]["tools"].as_array().unwrap();
        let mut names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(tools.iter().all(|t| t["inputSchema"]["type"] == "object" && t["description"].as_str().is_some_and(|d| !d.is_empty())));
        let n = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), n, "tool names are unique");

        let call = rpc(&hub, json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"list_profiles","arguments":{}}})).unwrap();
        assert_eq!(call["result"].get("isError"), None);
        assert!(call["result"]["content"][0]["text"].as_str().unwrap().contains("Desktop"));
    }
}
