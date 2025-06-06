use sea_orm::entity::prelude::*;

pub type Source = Model;
pub type ActiveSource = ActiveModel;
pub type EntitySource = Entity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sources")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub version: String,
    pub digest: String,
    pub source_type: String,
    pub registry_id: Option<i32>,
    pub wasm_blob: Option<Vec<u8>>,
    pub git_info_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::registry::Entity",
        from = "Column::RegistryId",
        to = "super::registry::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    Registries,

    #[sea_orm(
        belongs_to = "super::git_info::Entity",
        from = "Column::GitInfoId",
        to = "super::git_info::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    GitInfo,
}

impl Related<super::registry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Registries.def()
    }
}

impl Related<super::git_info::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GitInfo.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
