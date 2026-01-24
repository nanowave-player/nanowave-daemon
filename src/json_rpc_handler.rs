use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use crate::api_command::{ApiCommand, ApiEvent};

#[derive(Deserialize)]
pub struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Value,
}

#[derive(Serialize)]
pub struct JsonRpcResponse {
    jsonrpc: &'static str,
    result: Option<Value>,
    error: Option<Value>,
    id: Value,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            result: Some(result),
            error: None,
            id,
        }
    }

    fn error(id: Value, message: &str) -> Self {
        Self {
            jsonrpc: "2.0",
            result: None,
            error: Some(json!({ "message": message })),
            id,
        }
    }
}


#[derive(Deserialize)]
struct PlayMediaParams {
    id: String,
}

#[derive(Deserialize)]
struct SeekParams {
    pos: u64, // seconds
}

#[derive(Deserialize)]
struct MediaSourceFilterParams {
    query: String,
}

#[derive(Deserialize)]
struct MediaSourceFindParams {
    id: String,
}

#[derive(Deserialize)]
struct PreferencesUpdateParams {
    key: String,
    value: String,
}

/* =======================
   Helpers
   ======================= */

fn parse_params<T: for<'de> Deserialize<'de>>(params: Option<Value>) -> T {
    serde_json::from_value(params.unwrap_or(Value::Null))
        .expect("Invalid params")
}

#[derive(Clone)]
pub struct JsonRpcHandler {
    cmd_tx: UnboundedSender<ApiCommand>,
}

impl JsonRpcHandler {
    pub fn new(cmd_tx: UnboundedSender<ApiCommand>) -> Self {
        Self { cmd_tx }
    }

    pub async fn handle_request(
        &self,
        req: JsonRpcRequest,
    ) -> JsonRpcResponse {
        let (reply_tx, reply_rx) = oneshot::channel();

        let send = match req.method.as_str() {
            "player_play_media" => {
                let p: PlayMediaParams = parse_params(req.params);
                self.cmd_tx.send(ApiCommand::PlayerPlayMedia {
                    id: p.id,
                    reply: reply_tx,
                })
            }

            "player_play" => {
                self.cmd_tx.send(ApiCommand::PlayerPlay {
                    reply: reply_tx,
                })
            }

            "player_pause" => {
                self.cmd_tx.send(ApiCommand::PlayerPause {
                    reply: reply_tx,
                })
            }

            "player_seek" => {
                let p: SeekParams = parse_params(req.params);
                self.cmd_tx.send(ApiCommand::PlayerSeek {
                    pos: p.pos,
                    reply: reply_tx,
                })
            }

            "file_media_source_filter" => {
                let p: MediaSourceFilterParams = parse_params(req.params);
                self.cmd_tx.send(ApiCommand::MediaSourceFilter {
                    query: p.query,
                    reply: reply_tx,
                })
            }

            "file_media_source_find" => {
                let p: MediaSourceFindParams = parse_params(req.params);
                self.cmd_tx.send(ApiCommand::MediaSourceFind {
                    id: p.id,
                    reply: reply_tx,
                })
            }

            _ => {
                return JsonRpcResponse::error(req.id, "Unknown method");
            }
        };

        if send.is_err() {
            return JsonRpcResponse::error(req.id, "API unavailable");
        }

        match reply_rx.await {
            Ok(ApiEvent::Ok(val)) => JsonRpcResponse::success(req.id, val),
            Ok(ApiEvent::Error(err)) => JsonRpcResponse::error(req.id, &err),
            _ => JsonRpcResponse::error(req.id, "API failure"),
        }
    }
}
