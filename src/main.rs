
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc};
use std::thread;
use sea_orm::{Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;
use tokio::sync::Mutex;
use clap::Parser;

use tokio::net::{TcpListener, TcpStream};
use tokio::time::{interval, Duration};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use crate::entity::{item, items_json_metadata, items_metadata, items_progress_history};
use crate::media_source::file_media_source::FileMediaSource;
use crate::migrator::Migrator;
use crate::player::player::Player;
use crate::media_source::media_source::MediaSource;


mod gpio_button_service;
mod headset;
mod player;

mod entity;
mod migrator;

mod button_handler;
mod media_source;
mod debouncer;
mod audio;
mod display;
mod time;

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

#[derive(Deserialize)]
struct PlayerPlayMediaParams {
    id: String,
}

#[derive(Deserialize)]
struct MediaSourceFilterParams {
    query: String,
}



#[derive(Clone)]
struct Api {
    counter: Arc<Mutex<u64>>,
    clients: Arc<Mutex<Vec<ClientHandle>>>,
    player: Arc<Mutex<Player>>,
    media_source: Arc<Mutex<FileMediaSource>>,
}

impl Api {
    fn new(player: Player, media_source: FileMediaSource) -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
            clients: Arc::new(Mutex::new(Vec::new())),
            player: Arc::new(Mutex::new(player)),
            media_source: Arc::new(Mutex::new(media_source)),
        }
    }

    async fn handle_client_method(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, JsonRpcError> {
        let mut player = self.player.lock().await;
        let mut media_source = self.media_source.lock().await;
        match method {
            "echo" => Ok(params.unwrap_or(Value::Null)),

            "player_play_media" => {

                let params: PlayerPlayMediaParams = serde_json::from_value(
                    params.ok_or(JsonRpcError {
                        code: -32602,
                        message: "Missing params".into(),
                    })?,
                )
                    .map_err(|_| JsonRpcError {
                        code: -32602,
                        message: "Invalid params".into(),
                    })?;

                player.play_media(params.id).await;

                Ok(serde_json::json!({ "status": "playing" }))
            },
            "media_source_filter" => {
                let params: MediaSourceFilterParams = serde_json::from_value(
                    params.ok_or(JsonRpcError {
                        code: -32602,
                        message: "Missing params".into(),
                    })?,
                )
                    .map_err(|_| JsonRpcError {
                        code: -32602,
                        message: "Invalid params".into(),
                    })?;
                let filter_results = media_source.filter(params.query.as_str()).await;

                Ok(serde_json::json!({ "results": filter_results }))
            },
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

        println!("message: {:?}", msg);

        let req: JsonRpcRequest = match serde_json::from_str(&msg) {
            Ok(r) => {
                println!("request: {:?}", r);
                r
            },
            Err(_) => continue,
        };

        if let Some(id) = req.id {
            let result = api
                .handle_client_method(&req.method, req.params)
                .await;



            let response = if let Ok(ok) = result {
                println!("response ok: {:?}", ok);

                println!("ok response: {}", ok);
                JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: Some(ok),
                    error: None,
                    id,
                };
            } else {
                println!("response error");

                JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: result.err(),
                    id,
                };
            };

            let mut write = client.write.lock().await;
            println!("sending response");
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



async fn connect_db(db_url: &str, first_run: bool) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect(db_url).await?;
    // todo: dirty hack to prevent startup failure if db exists
    // this has to be solved with migrations or at least better than this
    if first_run {
        db.get_schema_builder()
            .register(item::Entity)
            .register(items_metadata::Entity)
            .register(items_json_metadata::Entity)
            .register(items_progress_history::Entity)
            .apply(&db)
            .await?;
    }
    Migrator::up(&db, None).await?;
    Ok(db)
}
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./media/")]
    base_directory: String,
}
#[tokio::main]
async fn main() {
    /*
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .init();
    */
    let args = Args::parse();
    let base_dir = args.base_directory.clone();
    println!("base directory is: {}", base_dir.clone());
    let base_path = Path::new(&base_dir);
    if !Path::exists(base_path) {
        match std::env::current_dir() {
            Ok(cwd) => {
                println!(
                    "base directory does not exist: {:?}, current dir is: {:?}",
                    base_path, cwd
                );
            }
            Err(_) => {
                println!("base directory does not exist: {}", args.base_directory);
            }
        }

        return; /* Err(format!(
            "Base directory does not exist: {}",
            args.base_directory
        ));*/
    }

    let db_path = format!(
        "{}/{}",
        base_dir.clone().trim_end_matches("/"),
        String::from("player.db")
    );
    let first_run = !Path::new(&db_path).exists();
    let db_url = format!("sqlite://{}?mode=rwc", db_path);

    let connect_result = connect_db(&db_url, first_run).await;
    if connect_result.is_err() {
        /*
        return Err(slint::PlatformError::Other(format!(
            "Could not find, create or migrate database: {}",
            connect_result.err().unwrap()
        )));
         */
        return;
    }

    let db = connect_result.unwrap();

    let file_source = FileMediaSource::new(db.clone(), args.base_directory);
    let fs_clone1 = file_source.clone();
    fs_clone1.scan_media().await;
    // let fs_clone2 = file_source.clone();


/*
    tokio::spawn(async move {
        fs_clone1.scan_media().await;
        // fs_clone1.run(source_cmd_rx, source_evt_tx).await;
    });
*/
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let player = Player::new(
        Arc::new(fs_clone1),
        "USB-C to 3.5mm Headphone Jack A".to_string(),
        "pipewire".to_string(),
    );


    let api = Api::new(player, file_source);

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
