use crate::media_source::media_source::MediaSource;
use std::path::Path;
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use sea_orm::{Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use crate::entity::{item, items_json_metadata, items_metadata, items_progress_history};
use crate::media_source::file_media_source::FileMediaSource;
use crate::migrator::Migrator;

mod entity;
mod migrator;
mod media_source;



#[derive(Clone)]
struct Api {
    media_source: FileMediaSource,
}


impl Api {
    async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            "media_source_filter" => {
                let params: MediaSourceFilterParams =
                    serde_json::from_value(req.params.unwrap_or(Value::Null))
                        .unwrap_or(MediaSourceFilterParams {
                            query: String::new(),
                        });

                let result = self.media_source.filter(params.query.as_str()).await;
                JsonRpcResponse::success(req.id, json!(result))
            }
            _ => JsonRpcResponse::error(req.id, "Unknown method"),
        }
    }
}

#[derive(Deserialize)]
struct MediaSourceFilterParams {
    query: String,
}

#[derive(Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Value,
}

#[derive(Serialize)]
struct JsonRpcResponse {
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



    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let api = Api {
        media_source: file_source,
    };

    println!("WebSocket server listening on ws://127.0.0.1:8080");

    while let Ok((stream, _)) = listener.accept().await {
        let api = api.clone();

        tokio::spawn(async move {
            let ws_stream = accept_async(stream).await.unwrap();
            let (mut write, mut read) = ws_stream.split();

            while let Some(Ok(msg)) = read.next().await {
                if !msg.is_text() {
                    continue;
                }

                let req: JsonRpcRequest =
                    serde_json::from_str(msg.to_text().unwrap()).unwrap();

                let resp = api.handle_request(req).await;
                let text = serde_json::to_string(&resp).unwrap();

                write.send(text.into()).await.unwrap();
            }
        });
    }
}
