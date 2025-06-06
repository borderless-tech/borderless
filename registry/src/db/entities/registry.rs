use sea_orm::entity::prelude::*;

pub type Registry = Model;
pub type ActiveAuthor = ActiveModel;
pub type EntityAuthor = Entity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "registries")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub registry_type: Option<String>,
    pub hostname: String,
    pub namespace: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
