use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::{Arc};
use tokio::sync::Mutex;

use tokio::net::{TcpListener, TcpStream};
use tokio::time::{interval, Duration};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    id: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[derive(Clone)]
struct ClientHandle {
    addr: SocketAddr,
    write: Arc<Mutex<
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<TcpStream>,
            Message,
        >
    >>,
}

#[derive(Clone)]
struct Api {
    counter: Arc<Mutex<u64>>,
    clients: Arc<Mutex<Vec<ClientHandle>>>,
}

impl Api {
    fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
            clients: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn handle_client_method(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, JsonRpcError> {
        match method {
            "add" => {
                let nums: Vec<f64> = serde_json::from_value(
                    params.unwrap_or(Value::Array(vec![])),
                )
                    .map_err(|_| JsonRpcError {
                        code: -32602,
                        message: "Invalid params".into(),
                    })?;
                Ok(serde_json::to_value(nums.iter().sum::<f64>()).unwrap())
            }
            "multiply" => {
                let nums: Vec<f64> = serde_json::from_value(
                    params.unwrap_or(Value::Array(vec![])),
                )
                    .map_err(|_| JsonRpcError {
                        code: -32602,
                        message: "Invalid params".into(),
                    })?;
                Ok(serde_json::to_value(nums.iter().product::<f64>()).unwrap())
            }
            "get_counter" => {
                let counter = *self.counter.lock().await;
                Ok(serde_json::to_value(counter).unwrap())
            }
            "increment_counter" => {
                let mut counter = self.counter.lock().await;
                *counter += 1;
                let value = *counter;
                drop(counter);

                self.notify_all_clients(
                    "counter_updated",
                    serde_json::to_value(value).unwrap(),
                )
                    .await;

                Ok(serde_json::to_value(value).unwrap())
            }
            "echo" => Ok(params.unwrap_or(Value::Null)),
            _ => Err(JsonRpcError {
                code: -32601,
                message: "Method not found".into(),
            }),
        }
    }

    async fn notify_all_clients(&self, method: &str, params: Value) {
        let clients = self.clients.lock().await.clone();

        let msg = Message::Text(
            serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        })
                .to_string().into(),
        );

        for client in clients {
            let mut write = client.write.lock().await;
            let _ = write.send(msg.clone()).await;
        }
    }


    async fn server_ping_all(&self) {
        self.notify_all_clients(
            "server_ping",
            serde_json::json!({ "timestamp": chrono::Utc::now().to_rfc3339() }),
        )
            .await;
    }

    async fn server_status_broadcast(&self) {
        let counter = *self.counter.lock().await;
        let clients = self.clients.lock().await.len();

        self.notify_all_clients(
            "server_status",
            serde_json::json!({ "counter": counter, "clients": clients }),
        )
            .await;
    }
}


async fn handle_connection(stream: TcpStream, addr: SocketAddr, api: Api) {
    let ws = accept_async(stream).await.unwrap();
    let (write, mut read) = ws.split();

    let client = ClientHandle {
        addr,
        write: Arc::new(Mutex::new(write)),
    };

    api.clients.lock().await.push(client.clone());
    api.server_status_broadcast().await;

    while let Some(msg) = read.next().await {
        let msg = match msg {
            Ok(Message::Text(t)) => t,
            Ok(Message::Close(_)) | Err(_) => break,
            _ => continue,
        };

        let req: JsonRpcRequest = match serde_json::from_str(&msg) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if let Some(id) = req.id {
            let result = api
                .handle_client_method(&req.method, req.params)
                .await;

            let response = if let Ok(ok) = result {
                JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: Some(ok),
                    error: None,
                    id,
                };
            } else {
                JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: result.err(),
                    id,
                };
            };

            let mut write = client.write.lock().await;
            let _ = write
                .send(Message::Text(
                    serde_json::to_string(&response).unwrap().into(),
                ))
                .await;
        }
    }

    api.clients.lock().await.retain(|c| c.addr != addr);
    api.server_status_broadcast().await;
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let api = Api::new();

    let api_clone = api.clone();
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(10));
        loop {
            tick.tick().await;
            api_clone.server_ping_all().await;
        }
    });

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr, api.clone()));
    }
}
