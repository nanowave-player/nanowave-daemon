use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Value,
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
struct Api {
    counter: Arc<Mutex<u64>>,
}

impl Api {
    fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
        }
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, JsonRpcError> {
        match method {
            "add" => {
                let nums: Vec<f64> = serde_json::from_value(params.unwrap_or(Value::Array(vec![])))
                    .map_err(|_| JsonRpcError { code: -32602, message: "Invalid params".to_string() })?;
                Ok(serde_json::to_value(nums.iter().sum::<f64>()).unwrap())
            }
            "multiply" => {
                let nums: Vec<f64> = serde_json::from_value(params.unwrap_or(Value::Array(vec![])))
                    .map_err(|_| JsonRpcError { code: -32602, message: "Invalid params".to_string() })?;
                Ok(serde_json::to_value(nums.iter().product::<f64>()).unwrap())
            }
            "get_counter" => {
                let counter = *self.counter.lock().unwrap();
                Ok(serde_json::to_value(counter).unwrap())
            }
            "increment_counter" => {
                let mut counter = self.counter.lock().unwrap();
                *counter += 1;
                Ok(serde_json::to_value(*counter).unwrap())
            }
            "echo" => {
                Ok(params.unwrap_or(Value::String("".to_string())))
            }
            _ => Err(JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
            }),
        }
    }
}

async fn handle_connection(stream: TcpStream, addr: SocketAddr, api: Api) {
    println!("New connection from {}", addr);

    if let Ok(mut ws_stream) = accept_async(stream).await {
        println!("Established JSON-RPC WebSocket from {}", addr);

        let (mut write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&text) {
                        if request.jsonrpc != "2.0" {
                            let error = JsonRpcError { code: -32600, message: "Invalid Request".to_string() };
                            let response = JsonRpcResponse { jsonrpc: "2.0".to_string(), result: None, error: Some(error), id: request.id };
                            let _ = write.send(Message::Text(serde_json::to_string(&response).unwrap().into())).await;
                            continue;
                        }

                        let result = api.handle_method(&request.method, request.params).await;
                        let response = JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: result.ok().map(|v| v.clone()),
                            // error: result.err(),
                            error: None,
                            id: request.id,
                        };

                        // println!("{} RPC {} -> {:?}", addr, request.method, response.result.as_ref().or_else(|| response.error.as_ref().map(|e| e.message)));

                        if let Err(e) = write.send(Message::Text(serde_json::to_string(&response).unwrap().into())).await {
                            eprintln!("Failed to send response: {}", e);
                            break;
                        }
                    } else {
                        let error = JsonRpcError { code: -32700, message: "Parse error".to_string() };
                        let response = JsonRpcResponse { jsonrpc: "2.0".to_string(), result: None, error: Some(error), id: Value::Null };
                        let _ = write.send(Message::Text(serde_json::to_string(&response).unwrap().into())).await;
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("{} sent close", addr);
                    break;
                }
                Err(e) => {
                    eprintln!("WebSocket error from {}: {}", addr, e);
                    break;
                }
                _ => {}
            }
        }
    }

    println!("{} disconnected", addr);
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.unwrap();
    let api = Api::new();

    println!("JSON-RPC 2.0 WebSocket server listening on ws://{addr}");
    println!("Available methods: add, multiply, get_counter, increment_counter, echo");

    while let Ok((stream, addr)) = listener.accept().await {
        let api = api.clone();
        tokio::spawn(handle_connection(stream, addr, api));
    }
}
