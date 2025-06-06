use sea_orm::entity::prelude::*;

type Capabilities = Model;
type ActiveCapabilities = ActiveModel;
type EntityCapabilities = Entity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "capabilities")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub network: bool,
    pub websocket: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
