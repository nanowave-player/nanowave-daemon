use chrono::Utc;
use sea_orm::entity::prelude::*;

// TagField enum stored as INTEGER
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum JsonTagField {
    #[sea_orm(num_value = 0)]
    Chapters,
    #[sea_orm(num_value = 1)]
    Lyrics,
}

#[sea_orm::model]
#[derive(DeriveEntityModel, Clone, Debug, PartialEq)]
#[sea_orm(table_name = "items_json_metadata")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    // Foreign key to items.id
    pub item_id: i32,

    pub tag_field: JsonTagField,

    pub value: String,

    pub date_modified: chrono::DateTime<Utc>,

    #[sea_orm(belongs_to, from = "item_id", to = "id")]
    pub item: HasOne<super::item::Entity>,

}

impl ActiveModelBehavior for ActiveModel {}
