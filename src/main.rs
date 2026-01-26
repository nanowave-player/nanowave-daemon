use sea_orm::ColumnTrait;
use sea_orm::QueryFilter;
use crate::media_source::media_source::{MediaSource, MediaSourceCommand, MediaSourceEvent};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use sea_orm::{Database, DatabaseConnection, DbErr, EntityTrait};
use sea_orm_migration::MigratorTrait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use crate::api::Api;
use crate::entity::{item, items_json_metadata, items_metadata, items_progress_history};
use crate::json_rpc_handler::{JsonRpcDispatcher, JsonRpcRequest, MediaSourceRpcHandler};
use crate::media_source::file_media_source::FileMediaSource;
use crate::migrator::Migrator;
use crate::player::player::Player;

mod entity;
mod migrator;
mod media_source;

mod player;
mod api_command;
mod json_rpc_handler;
mod api;
/*
#[derive(Clone)]
struct Api {
    player: Player,
    media_source: FileMediaSource,
    // preferences: Preferences,
}

impl Api {
    pub fn emit_event(&self, method: &str, params: serde_json::Value) {
        // This is intentionally transport-agnostic.
        // Your websocket layer can subscribe to these.
        let event = JsonRpcResponse {
            jsonrpc: "2.0",
            result: None,
            error: None,
            id: serde_json::Value::Null,
        };

        let _ = (method, params, event);
        // hook into your outbound message bus here
    }

    async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let mut player = self.player.clone();
        match req.method.as_str() {
            // Player
            "player_play_media" => {
                let params: PlayMediaParams = parse_params(req.params);
                player.play_media(params.id).await;
                JsonRpcResponse::success(req.id, json!(true))
            }
            "player_play" => {
                player.play();
                JsonRpcResponse::success(req.id, json!(true))
            }
            "player_pause" => {
                player.pause();
                JsonRpcResponse::success(req.id, json!(true))
            }
            "player_seek" => {
                let params: SeekParams = parse_params(req.params);
                player
                    .try_seek(Duration::from_secs(params.pos));

                JsonRpcResponse::success(req.id, json!(true))
            }

            // FileMediaSource
            "file_media_source_filter" => {
                let params: MediaSourceFilterParams = parse_params(req.params);
                let result = self.media_source.filter(params.query.as_str()).await;
                JsonRpcResponse::success(req.id, json!(result))
            }
            "file_media_source_find" => {
                let params: MediaSourceFindParams = parse_params(req.params);
                let result = self.media_source.find(params.id.as_str()).await;
                JsonRpcResponse::success(req.id, json!(result))
            }

            /*
            // Preferences
            "preferences_update" => {
                let params: PreferencesUpdateParams = parse_params(req.params);
                self.preferences
                    .update(params.key, params.value)
                    .await;
                JsonRpcResponse::success(req.id, json!(true))
            }

             */

            _ => JsonRpcResponse::error(req.id, "Unknown method"),
        }
    }
}
*/
/* =======================
   Params structs
   ======================= */

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

    let db_clone = db.clone();

    /*
    let str = "something".to_string();
    let item_result = item::Entity::find()
        .filter(item::Column::FileId.eq(str))
        .one(&db_clone)
        .await;

    if item_result.is_err() {
        println!("error {:?}", item_result);
    } else {
        println!("success");
    }
    tokio::time::sleep(Duration::from_secs(5));
    return;

     */
    let (source_cmd_tx, source_cmd_rx) = mpsc::unbounded_channel::<MediaSourceCommand>();
    let (source_evt_tx, source_evt_rx) = mpsc::unbounded_channel::<MediaSourceEvent>();

    let media_source = FileMediaSource::new(db.clone(), args.base_directory);
    media_source.scan_media().await;

    let media_source_clone = media_source.clone();
    tokio::spawn(async move {
        media_source_clone.run(source_cmd_rx).await;
    });


    /*

    let player = Player::new(
        media_source_clone,
        vec!["USB-C to 3.5mm Headphone Jack A", "pipewire"]
    );
    player.run_in_background();
    player.play_test().await;

    // let player_clone = player.clone();
    // let _ = player_clone.play_test().await;

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
    let (evt_tx, _evt_rx) = tokio::sync::mpsc::unbounded_channel();

    let mut api = Api::new(player, media_source, cmd_rx, evt_tx);
    tokio::spawn(async move {
        api.run().await;
    });
    */

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let mut dispatcher = JsonRpcDispatcher::new();
    let media_source_rpc = MediaSourceRpcHandler::new(source_cmd_tx);
    dispatcher.add_handler(media_source_rpc);

    // let handler = JsonRpcHandler::new(cmd_tx);

    while let Ok((stream, _)) = listener.accept().await {

        println!("new client deteced");

        let handler = dispatcher.clone();

        tokio::spawn(async move {
            let ws = accept_async(stream).await.unwrap();
            let (mut write, mut read) = ws.split();

            while let Some(Ok(msg)) = read.next().await {
                if !msg.is_text() {
                    continue;
                }

                let text = msg.to_text();
                if text.is_err() {
                    continue;
                }
                let json = serde_json::from_str(text.unwrap());
                if json.is_err() {
                    continue;
                }
                let req: JsonRpcRequest =
                    json.unwrap();

                let resp = handler.handle_request(req).await;
                let text = serde_json::to_string(&resp).unwrap();

                write.send(text.into()).await.unwrap();
            }
        });
    }

}
