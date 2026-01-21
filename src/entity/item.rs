use sea_orm::entity::prelude::*;

use chrono::{DateTime, Utc};
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum MediaType {
    #[sea_orm(num_value = 0)]
    Unspecified,
    #[sea_orm(num_value = 2)]
    Audiobook,
    #[sea_orm(num_value = 4)]
    Music,
}

#[sea_orm::model]
#[derive(DeriveEntityModel, Clone, Debug, PartialEq)]
#[sea_orm(table_name = "items")]   // plural table name
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub file_id: String,

    pub media_type: MediaType,

    pub cover_hash: String,
    
    pub location: String,

    // this key is randomly generated on every "full scan" and each item gets updated
    // all items that do not have this updated key get
    pub last_scan_random_key: String,
    
    pub date_modified: DateTime<Utc>,
    
    #[sea_orm(has_many)]
    pub metadata: HasMany<super::items_metadata::Entity>,

    #[sea_orm(has_many)]
    pub json: HasMany<super::items_json_metadata::Entity>,



    #[sea_orm(has_many)]
    pub progress_history: HasMany<super::items_progress_history::Entity>,

    /*
    // properties needed for listing
    pub cover: String, // empty for no cover, rel_path for cover

    pub genre: String,

    pub album: String,

    pub title: String, // title

    pub sort_title: String,

    pub artist: String, // artist or author

    pub composer: String, // composer or narrator

    pub series: String, // series

    pub part: String,

    pub release_date: NaiveDate,

    pub duration: NaiveTime,

    pub progress: NaiveTime,
    */

}

impl ActiveModelBehavior for ActiveModel {}
