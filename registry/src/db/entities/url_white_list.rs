use sea_orm::entity::prelude::*;

pub type UrlWhitelist = Model;
pub type ActiveUrlWhitelist = ActiveModel;
pub type EntityUrlWhitelist = Entity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "url_whitelist")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub capability_id: i32,
    pub url: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::capabilities::Entity",
        from = "Column::CapabilityId",
        to = "super::capabilities::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Capabilities,
}

impl Related<super::capabilities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Capabilities.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
