use async_trait::async_trait;
use chrono::{DateTime, Local, Utc};
use image::imageops::FilterType;
use image::{load_from_memory, GenericImageView};
use lofty::error::LoftyError;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::MimeType;
use lofty::probe::Probe;
use lofty::tag::TagType::Mp4Ilst;
use lofty::tag::{Accessor, Tag};
use media_source::media_source_chapter::MediaSourceChapter;
use media_source::media_source_image_codec::MediaSourceImageCodec;
use media_source::media_source_item::MediaSourceItem;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration
};
use file_id::get_file_id;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use walkdir::WalkDir;

use crate::entity::item::{ActiveModel, ActiveModelEx};
use crate::entity::items_json_metadata::JsonTagField::Chapters;
use crate::entity::items_metadata::TagField::*;
use crate::entity::{item, items_json_metadata, items_metadata};
use crate::media_source::media_source::MediaSource;
use crate::media_source::media_source_command::MediaSourceCommand;
use crate::media_source::media_source_event::MediaSourceEvent;
use mp4ameta::FreeformIdent;
use sea_orm::prelude::HasMany;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, HasManyModel, QueryFilter};
use xxhash_rust::xxh3::xxh3_64;

use media_source::media_source_metadata::MediaSourceMetadata;
use media_source::media_source_picture::MediaSourcePicture;
use media_source::media_type::MediaType;

#[derive(Clone)]
pub struct FileMediaSource {
    pub db: DatabaseConnection,
    pub base_path: String,
    state: Arc<Mutex<FileMediaSourceState>>,
}

struct FileMediaSourceState {
    pub base_path: String,
}

impl FileMediaSource {
    pub fn new(db: DatabaseConnection, base_path: String) -> Self {
        Self {
            db,
            base_path: base_path.clone(),
            state: Arc::new(Mutex::new(FileMediaSourceState {
                base_path
            })),
        }
    }

    pub fn empty_metadata(&self) -> MediaSourceMetadata {
        MediaSourceMetadata {
            artist: None,
            title: None,
            album: None,
            genre: None,
            composer: None,
            series: None,
            part: None,
            cover: None,
            chapters: vec![],
        }
    }
    pub fn map_db_model_to_media_item(&self, i: &item::ModelEx, metadata: &HasMany<items_metadata::Entity>, json: &HasMany<items_json_metadata::Entity>) -> MediaSourceItem {
        let mut title : String = String::from("");
        let mut genre : Option<String> = None;
        let mut artist : Option<String> = None;
        let mut album : Option<String> = None;
        let mut composer : Option<String> = None;
        let mut series : Option<String> = None;
        let mut part : Option<String> = None;
        let cover = Some(MediaSourcePicture {
            cache_dir: self.cache_path(),
            hash: i.cover_hash.clone(),
            codec: MediaSourceImageCodec::Jpeg
        });
        let filename = i.location.split('/').last();
        if filename.is_some() {
            let filename_no_ext = filename.unwrap().split('.').next();
            if filename_no_ext.is_some() {
                title = filename_no_ext.unwrap().to_string();
            }
        }

        for tag in metadata {
            match tag.tag_field {
                Title => title = tag.value.clone(),
                Genre => genre = Some(tag.value.clone()),
                Artist => artist = Some(tag.value.clone()),
                Album => album = Some(tag.value.clone()),
                Composer => composer = Some(tag.value.clone()),
                Series => series = Some(tag.value.clone()),
                Part => part = Some(tag.value.clone()),
            };
        }

        let mut chapters: Vec<MediaSourceChapter> = Vec::new();

        for json_tag in json {
            match json_tag.tag_field {
                Chapters => {
                    if let Ok(chaps) = serde_json::from_str(&json_tag.value) {
                        chapters = chaps;
                    }
                },
                _ => {}
            }
        }

        MediaSourceItem {
            id: i.id.to_string(),
            location: format!("{}/{}", self.base_path.clone().trim_end_matches('/'), i.location.trim_start_matches('/').to_string()),
            title: title.clone(),
            media_type: MediaType::Unspecified,
            metadata: MediaSourceMetadata {
                title: Some(title.clone()),
                artist,
                album,
                genre,
                composer,
                series,
                part,
                cover,
                chapters
            },
        }
    }


    async fn upsert_item(&self, id: i32, file_id: String, media_type: item::MediaType, location: String, meta: &MediaSourceMetadata) -> ActiveModelEx {
        // todo: improve this
        // see https://www.sea-ql.org/blog/2025-11-25-sea-orm-2.0/
        let db = self.db.clone();
        let now = Utc::now();
        let cover = meta.cover.clone();

        let cover_hash = if cover.is_some() {
            cover.unwrap().hash
        } else {
            String::from("")
        };



        // let file_id_item = self.find_file_id()


        // if id == 0 insert, otherwise update
        let builder = if id == 0 {
            ActiveModel::builder()
                .set_file_id(file_id)
                .set_media_type(media_type)
                .set_location(location.trim_start_matches('/'))
                .set_cover_hash(cover_hash)
                .set_last_scan_random_key("")
                .set_date_modified(now)
                //.add_metadatum(metadata_items)

        } else {
            ActiveModel::builder()
                .set_id(id)
                .set_file_id(file_id)
                .set_media_type(media_type)
                .set_location(location.trim_start_matches('/'))
                .set_cover_hash(cover_hash)
                .set_last_scan_random_key("")
                .set_date_modified(now)

        };


        let mut result = builder
            // .add_metadatum()
            // .add_picture()
            // .add_progress_history()
            .save(&db)
            .await
            .expect("todo");


        // now sync the metadata
        // todo: handle multi persons with comma separated values
        self.add_metadata(&mut result.metadata, Genre, meta.genre.clone(), now);
        self.add_metadata(&mut result.metadata, Artist, meta.artist.clone(), now);
        self.add_metadata(&mut result.metadata, Title, meta.title.clone(), now);
        self.add_metadata(&mut result.metadata, Album, meta.album.clone(), now);
        self.add_metadata(&mut result.metadata, Composer, meta.composer.clone(), now);
        self.add_metadata(&mut result.metadata, Series, meta.series.clone(), now);
        self.add_metadata(&mut result.metadata, Part, meta.part.clone(), now);

        if !meta.chapters.is_empty() {
            let chapters_json_result = serde_json::to_string(&meta.chapters);
            if let Ok(chapters_json) = chapters_json_result {
                let chapters_model = items_json_metadata::ActiveModel::builder()
                    .set_tag_field(Chapters)
                    .set_value(chapters_json)
                    .set_date_modified(now);
                result.json.push(chapters_model);
            }

        }

        let res = result.save(&db).await;

        res.unwrap()
    }

    fn add_metadata(&self, metadata: &mut HasManyModel<items_metadata::Entity>, tag_field: items_metadata::TagField, value: Option<String>, date_modified: DateTime<Utc>) {
        if value.is_some() {
            metadata.push(items_metadata::ActiveModel::builder()
                .set_tag_field(tag_field)
                .set_value(value.unwrap())
                .set_date_modified(date_modified));
        }
    }


    fn cache_path(&self) -> String {
        let inner = self.state.lock().unwrap();
        let cache_path = format!("{}/{}", inner.base_path.trim_end_matches('/').to_string(), self.rel_cache_path());
        drop(inner);
        cache_path
    }

    fn rel_cache_path(&self) -> String {
        String::from("cache/")
    }

    pub async fn scan_media(&self) {
        let audio_extensions = vec!("mp3", "m4b");
        let inner = self.state.lock().unwrap();
        let base_path = inner.base_path.clone();
        let db = self.db.clone();
        drop(inner);



        let audio_files = WalkDir::new(base_path.clone())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let e_clone = e.clone();
                let metadata = e_clone.metadata().unwrap();
                if !metadata.is_file() {
                    false;
                }
                let path = e_clone.into_path();
                return match path.extension() {
                    Some(ext) => {
                        audio_extensions.contains(&ext.to_str().unwrap())
                    }
                    None => false,
                }

            });

        for audio_file in audio_files {
            let full_path = audio_file.path().to_str().unwrap().to_string();
            let start_index = base_path.len();
            let rel_path = full_path[start_index..].to_string();
            let media_type = if rel_path.starts_with("/music/") {
                item::MediaType::Music
            } else if rel_path.starts_with("/audiobooks/") {
                item::MediaType::Audiobook
            } else {
                item::MediaType::Unspecified
            };

            let file_id = get_file_id(full_path.clone()).unwrap();
            let file_id_str = format!("{:?}", file_id);
            let file_date_modified = audio_file.path().metadata().unwrap().modified().unwrap();
            let file_date_mod_compare: DateTime<Local> = DateTime::from(file_date_modified);

            println!("check entity");
            let item_result = item::Entity::find()
                .filter(item::Column::FileId.eq(file_id_str.clone()))
                .one(&db)
                .await;
            println!("item result");
            if item_result.is_err() {
                println!("result err {:?}", item_result);
                continue;
            }
            println!("result ok");
            let item_option = item_result.unwrap();

            let (item_is_modified, id) = if let Some(item) = item_option {
                (item.date_modified < file_date_mod_compare, item.id)
            } else {
                (true, 0)
            };

            if item_is_modified {
                // file_name_without_ext
                let mut item_meta_result = self.extract_metadata(full_path.clone()).await;
                if item_meta_result.is_err() {
                    item_meta_result = Ok(self.empty_metadata());
                }
                // println!("item is modified");
                let item_meta = if let Ok(meta) = item_meta_result {
                    meta
                } else {
                    self.empty_metadata()
                };

                let _ = self.upsert_item(id, file_id_str.clone(), media_type.clone(), rel_path.clone(), &item_meta).await;
            } else {
                // item has not been modified
            }
        }
    }

    async fn extract_metadata(&self, path: String) -> Result<MediaSourceMetadata, LoftyError> {

        let tagged_file = Probe::open(path.clone())?.guess_file_type()?.read()?;
        let tag_result = match tagged_file.primary_tag() {
            Some(primary_tag) => Some(primary_tag),
            // If the "primary" tag doesn't exist, we just grab the
            // first tag we can find. Realistically, a tag reader would likely
            // iterate through the tags to find a suitable one.
            None => tagged_file.first_tag(),
        };

        let properties = tagged_file.properties();
        let duration = properties.duration();

        if tag_result.is_none() {
            return Ok(self.empty_metadata());
        }
        let tag = tag_result.unwrap();
        let mut media_source_metadata = MediaSourceMetadata::new(
            tag.artist().map(|s| s.to_string()),
            tag.title().map(|s| s.to_string()),
            tag.album().map(|s| s.to_string()),
            None, // composer
            None, // series
            None, // part
            None, // genre
            None, // cover
            vec![], // chapters
        );
        let pictures = self.extract_pictures(tag).await?;
        if pictures.len() > 0 {
            media_source_metadata.cover = Some(pictures[0].clone());
        }

        if tag.tag_type() == Mp4Ilst {
            self.extract_mp4_metadata(&mut media_source_metadata, path.clone(), duration);
        }

        Ok(media_source_metadata)
    }

    fn extract_mp4_metadata(&self, meta: &mut MediaSourceMetadata, path: String, duration: Duration) {
        let mut chapters: Vec<MediaSourceChapter> = Vec::new();
        let mp4tag = mp4ameta::Tag::read_from_path(path.clone()).unwrap();
        let tmp_chaps = mp4tag.chapters().iter().rev();
        let mut end = duration;
        for tmp_chap in tmp_chaps {
            let duration = end - tmp_chap.start;
            chapters.push(MediaSourceChapter::new(tmp_chap.title.clone(), tmp_chap.start, duration));
            end -= duration;
        }
        chapters.reverse();
        meta.chapters = chapters;

        let movement = mp4tag.movement();
        let movement_index = mp4tag.movement_index();
        meta.composer = mp4tag.composer().map(|s| s.to_string());

        // mp4tag.artist_sort_order()
        let series_indent = FreeformIdent::new_static("com.pilabor.tone", "SERIES");
        let series = mp4tag.strings_of(&series_indent).next();
        let part_indent = FreeformIdent::new_static("com.pilabor.tone", "PART");
        let part = mp4tag.strings_of(&part_indent).next();
        meta.genre = mp4tag.genre().map(String::from);
        // let series_part = format!("{} {}", series, part);

        if series.is_some() {
            meta.series = series.map(|s| s.to_string());
        } else if movement.is_some() {
            meta.series = movement.map(|s| s.to_string());
        }

        if part.is_some() {
            meta.part = part.map(|s| s.to_string());
        } else if movement_index.is_some() {
            meta.part = movement_index.map(|s| s.to_string());
        }
    }

    async fn extract_pictures(&self, tag: &Tag) -> Result<Vec<MediaSourcePicture>, LoftyError> {
        let mut pics: Vec<MediaSourcePicture> = Vec::new();

        for pic in tag.pictures() {
            let hash_u64 = xxh3_64(&pic.data());
            let hash = format!("{:016x}", hash_u64); // 16 chars, lowercase, zero-padded

            let media_source_picture = MediaSourcePicture {
                cache_dir: self.cache_path(),
                hash,
                codec: self.map_encoding(pic.mime_type())
            };

            // we will always use webp for thumbnails and images
            let pic_ext = String::from("jpg");
            let pic_path_str = media_source_picture.path();
            let pic_full_path = PathBuf::from(media_source_picture.pic_full_path( pic_ext.clone()));
            let tb_full_path = PathBuf::from(media_source_picture.tb_full_path( pic_ext.clone()));
            fs::create_dir_all(pic_path_str.clone())?;

            let pic_full_path_exists = pic_full_path.exists();
            if !pic_full_path_exists {
                let _ = resize_image_bytes_to_file(&pic.data(), &pic_full_path, 368, 368);
            }

            if !tb_full_path.exists() {
                let _ = resize_image_bytes_to_file(&pic.data(), &tb_full_path, 192, 192);
            }

            pics.push(media_source_picture);
        }

        Ok(pics)
    }


    fn map_encoding(&self, p0: Option<&MimeType>) -> MediaSourceImageCodec {
        if p0.is_some() && let Some(mime_type) = p0 {
            return match mime_type {
                MimeType::Png => MediaSourceImageCodec::Png,
                MimeType::Jpeg => MediaSourceImageCodec::Jpeg,
                MimeType::Tiff => MediaSourceImageCodec::Tiff,
                MimeType::Bmp => MediaSourceImageCodec::Bmp,
                MimeType::Gif => MediaSourceImageCodec::Gif,
                _ => MediaSourceImageCodec::Unknown
            }
        }
        MediaSourceImageCodec::Unknown
    }
}

#[async_trait]
impl MediaSource for FileMediaSource {
    fn id(&self) -> String {
        let inner = self.state.lock().unwrap();
        let id = inner.base_path.clone();
        drop(inner);
        id
    }

    async fn filter(&self, query: &str) -> Vec<MediaSourceItem> {
        let db = self.db.clone();

        // let q = query.to_lowercase();
        let media_type = match query {
            "4" => item::MediaType::Music,
            "2" => item::MediaType::Audiobook,
            _ => item::MediaType::Unspecified
        };

        let items = item::Entity::load()
                .filter(item::Column::MediaType.eq(media_type))
                .with(items_metadata::Entity)
                .all(&db)
                .await;
        if items.is_err() {
            return vec![];
        }

        let items = items.unwrap();
        let result: Vec<MediaSourceItem> = items.iter().map(|i| {
            self.map_db_model_to_media_item(i, &i.metadata, &i.json)
        }).collect();

        result
    }

    async fn find(&self, id: &str) -> Option<MediaSourceItem> {
        let db = self.db.clone();
        let items = item::Entity::load()
            .filter(item::Column::Id.eq(id))
            .with(items_metadata::Entity)
            .with(items_json_metadata::Entity)
            .one(&db)
            .await;

        if items.is_err() {
            return None;
        }

        let items = items.unwrap();

        if let Some(i) = items {
            return Some(self.map_db_model_to_media_item(&i, &i.metadata, &i.json));
        }
        None
    }

    async fn run(
        mut self,
        mut cmd_rx: UnboundedReceiver<MediaSourceCommand>,
    ) {

        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                MediaSourceCommand::Filter { query, reply } => {
                    let _ = reply.send( MediaSourceEvent::FilterResults(self.filter(&query).await));
                }
                MediaSourceCommand::Find { id, reply } => {
                    let _ = reply.send( MediaSourceEvent::FindResult( self.find(&id).await));
                }
            }
        }
    }
}

fn resize_image_bytes_to_file(
    image_bytes: &[u8],
    output_path: &Path,
    max_width: u32,
    max_height: u32
) -> Result<(), Box<dyn std::error::Error>> {
    let img = load_from_memory(image_bytes)?;
    let img_format =  image::ImageFormat::Jpeg;
    let (width, height) = img.dimensions();
    if width <= max_width && height <= max_height {
        img.save_with_format(output_path, img_format)?;
        return Ok(());
    }

    let aspect = width as f32 / height as f32;
    let target_width = (max_height as f32 * aspect).min(max_width as f32) as u32;
    let target_height = (max_width as f32 / aspect).min(max_height as f32) as u32;

    let resized = img.resize(target_width, target_height, FilterType::Lanczos3);
    resized.save_with_format(output_path, img_format)?;

    Ok(())
}