use chrono::Utc;
use sea_orm::entity::prelude::*;


#[sea_orm::model]
#[derive(DeriveEntityModel, Clone, Debug, PartialEq)]
#[sea_orm(table_name = "items_progress_history")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    // Foreign key to items.id
    pub item_id: i32,

    pub session_key: String,

    // playback position in seconds, percentage, etc.
    pub position: Time,

    pub date_modified: chrono::DateTime<Utc>,

    #[sea_orm(belongs_to, from = "item_id", to = "id")]
    pub item: HasOne<super::item::Entity>,

}

impl ActiveModelBehavior for ActiveModel {}
