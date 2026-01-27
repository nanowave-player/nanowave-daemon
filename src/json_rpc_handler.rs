use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{mpsc, mpsc::UnboundedSender, oneshot};
use tokio::sync::oneshot::Receiver;
use crate::api_command::{ApiCommand, ApiEvent};
use crate::media_source::media_source::{MediaSourceCommand, MediaSourceEvent};

use async_trait::async_trait;





#[derive(Deserialize,Clone)]
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

/*
#[derive(Deserialize)]
struct PlayMediaParams {
    id: String,
}

#[derive(Deserialize)]
struct SeekParams {
    pos: u64, // seconds
}
*/

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




#[async_trait]
pub trait JsonRpcHandler: Send + Sync {
    /// Returns `Some(response)` if this handler handled the method,
    /// otherwise `None` so another handler can try.
    async fn handle(
        &self,
        req: &JsonRpcRequest,
    ) -> Option<JsonRpcResponse>;
}


#[derive(Clone)]
pub struct MediaSourceRpcHandler {
    cmd_tx: mpsc::UnboundedSender<MediaSourceCommand>,
}

impl MediaSourceRpcHandler {
    pub fn new(cmd_tx: mpsc::UnboundedSender<MediaSourceCommand>) -> Self {
        Self { cmd_tx }
    }
}

#[async_trait::async_trait]
impl JsonRpcHandler for MediaSourceRpcHandler {
    async fn handle(
        &self,
        req: &JsonRpcRequest,
    ) -> Option<JsonRpcResponse> {

        println!("MediaSourceRpcHandler::handle");

        let req_params_opt = req.params.clone();
        if req_params_opt.is_none() {
            return None;
        }
        let req_params = req_params_opt.unwrap();

        match req.method.as_str() {
            "media_source_filter" => {
                println!("media_source_filter");

                let params: MediaSourceFilterParams =
                    serde_json::from_value(req_params).ok()?;

                let (tx, rx) = oneshot::channel();

                self.cmd_tx.send(MediaSourceCommand::Filter {
                    query: params.query,
                    reply: tx,
                }).ok()?;

                match rx.await.ok()? {
                    MediaSourceEvent::FilterResults(items) => {
                        Some(JsonRpcResponse::success(
                            req.id.clone(),
                            json!(items),
                        ))
                    }
                    _ => Some(JsonRpcResponse::error(
                        req.id.clone(),
                        "Unexpected media source event",
                    )),
                }
            }

            "media_source_find" => {
                println!("media_source_find");
                let params: MediaSourceFindParams =
                    serde_json::from_value(req_params).ok()?;

                let (tx, rx) = oneshot::channel();

                self.cmd_tx.send(MediaSourceCommand::Find {
                    id: params.id,
                    reply: tx,
                }).ok()?;

                match rx.await.ok()? {
                    MediaSourceEvent::FindResult(item) => {
                        Some(JsonRpcResponse::success(
                            req.id.clone(),
                            json!(item),
                        ))
                    }
                    _ => Some(JsonRpcResponse::error(
                        req.id.clone(),
                        "Unexpected media source event",
                    )),
                }
            }

            _ => None,
        }
    }
}

















#[derive(Clone)]
pub struct JsonRpcDispatcher {
    handlers: Vec<std::sync::Arc<dyn JsonRpcHandler>>,
}

impl JsonRpcDispatcher {
    pub fn new() -> Self {
        Self { handlers: Vec::new() }
    }

    pub fn add_handler<H>(&mut self, handler: H)
    where
        H: JsonRpcHandler + 'static,
    {
        self.handlers.push(std::sync::Arc::new(handler));
    }

    pub async fn handle_request(
        &self,
        req: JsonRpcRequest,
    ) -> JsonRpcResponse {
        println!("JsonRpcDispatcher::handle_request");

        for handler in &self.handlers {
            if let Some(resp) = handler.handle(&req).await {
                return resp;
            }
        }

        JsonRpcResponse::error(req.id, "Unknown method")
    }
}





/*
pub trait JsonRpcRequestHandler {
    fn id(&self) -> &str;
    fn supports(&self, method: &str) -> bool;
    fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse;
}
*/
/*
pub struct MediaSourceRequestHandler {
    cmd_tx: UnboundedSender<MediaSourceCommand>,
}
/*
 */
impl MediaSourceRequestHandler {
    pub fn new(cmd_tx: UnboundedSender<MediaSourceCommand>) -> Self {
        Self {
            cmd_tx
        }
    }
}
*/
/*
impl JsonRpcRequestHandler for MediaSourceRequestHandler {
    fn id(&self) -> &str {
        "media_source"
    }
    fn supports(&self, method:&str) -> bool {
        method.starts_with(self.id())
    }
    fn handle_request(&self, req: JsonRpcRequest, reply_tx: Receiver<MediaSourceEvent>) {
        match req.method.as_str() {
            "media_source_filter" => {
                let p: MediaSourceFilterParams = parse_params(req.params);
                self.cmd_tx.send(MediaSourceCommand::Filter(p.query))
            }
        }
    }
}
*/
/*
#[derive(Clone)]
pub struct JsonRpcHandler {
    request_handlers: Vec<Arc<dyn JsonRpcRequestHandler>>,
}
*/
//
// impl JsonRpcHandler {
//     pub fn new() -> Self {
//         Self {
//             request_handlers: Vec::new(),
//         }
//     }
//
//     pub fn add_request_handler(&mut self, handler: Arc<dyn JsonRpcRequestHandler>) {
//         self.request_handlers.push(handler);
//     }
//
//     pub async fn handle_request(
//         &self,
//         req: JsonRpcRequest,
//     ) -> JsonRpcResponse {
//         let (reply_tx, reply_rx) = oneshot::channel();
//
//         let method = req.method.clone().as_str();
//         for handler in &self.request_handlers {
//             if handler.supports(method) {
//                 return handler.handle_request(req, reply_tx);
//             }
//         }
//     }
//     /*
//     pub fn new(cmd_tx: UnboundedSender<ApiCommand>) -> Self {
//         Self { cmd_tx }
//     }
//
//     pub async fn handle_request(
//         &self,
//         req: JsonRpcRequest,
//     ) -> JsonRpcResponse {
//         let (reply_tx, reply_rx) = oneshot::channel();
//
//         let send = match req.method.as_str() {
//             "player_play_media" => {
//                 let p: PlayMediaParams = parse_params(req.params);
//                 self.cmd_tx.send(ApiCommand::PlayerPlayMedia {
//                     id: p.id,
//                     reply: reply_tx,
//                 })
//             }
//
//             "player_play" => {
//                 self.cmd_tx.send(ApiCommand::PlayerPlay {
//                     reply: reply_tx,
//                 })
//             }
//
//             "player_pause" => {
//                 self.cmd_tx.send(ApiCommand::PlayerPause {
//                     reply: reply_tx,
//                 })
//             }
//
//             "player_seek" => {
//                 let p: SeekParams = parse_params(req.params);
//                 self.cmd_tx.send(ApiCommand::PlayerSeek {
//                     pos: p.pos,
//                     reply: reply_tx,
//                 })
//             }
//
//             "file_media_source_filter" => {
//                 let p: MediaSourceFilterParams = parse_params(req.params);
//                 self.cmd_tx.send(ApiCommand::MediaSourceFilter {
//                     query: p.query,
//                     reply: reply_tx,
//                 })
//             }
//
//             "file_media_source_find" => {
//                 let p: MediaSourceFindParams = parse_params(req.params);
//                 self.cmd_tx.send(ApiCommand::MediaSourceFind {
//                     id: p.id,
//                     reply: reply_tx,
//                 })
//             }
//
//             _ => {
//                 return JsonRpcResponse::error(req.id, "Unknown method");
//             }
//         };
//
//         if send.is_err() {
//             return JsonRpcResponse::error(req.id, "API unavailable");
//         }
//
//         match reply_rx.await {
//             Ok(ApiEvent::Ok(val)) => JsonRpcResponse::success(req.id, val),
//             Ok(ApiEvent::Error(err)) => JsonRpcResponse::error(req.id, &err),
//             _ => JsonRpcResponse::error(req.id, "API failure"),
//         }
//     }
//
//      */
// }
