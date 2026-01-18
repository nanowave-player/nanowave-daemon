use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};

async fn handle_connection(stream: TcpStream, addr: SocketAddr) {
    println!("New connection from {}", addr);

    if let Ok(ws_stream) = accept_async(stream).await {
        println!("Established WebSocket connection from {}", addr);

        let (mut write, mut read) = ws_stream.split();

        // Echo incoming messages back to client
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    println!("Received from {}: {}", addr, text);
                    if let Err(e) = write.send(Message::Text(text)).await {
                        eprintln!("Failed to echo: {}", e);
                        break;
                    }
                }
                Ok(Message::Binary(bin)) => {
                    println!("Received {} bytes from {}", bin.len(), addr);
                    if let Err(e) = write.send(Message::Binary(bin)).await {
                        eprintln!("Failed to echo binary: {}", e);
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("{} sent close message", addr);
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

    println!("Connection from {} closed", addr);
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("WebSocket server listening on ws://{addr}");

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr));
    }
}
