use sea_orm::{
    entity::prelude::*,
    ActiveValue::{NotSet, Set},
    DatabaseTransaction,
};

use crate::error::Error;

pub type ActiveUrlWhitelist = ActiveModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "url_whitelist")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub capability_id: u64,
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

impl ActiveUrlWhitelist {
    pub async fn from_id_and_url(
        txn: &DatabaseTransaction,
        capability_id: u64,
        url: String,
    ) -> Result<u64, Error> {
        let url_whitelist = ActiveUrlWhitelist {
            id: NotSet,
            capability_id: Set(capability_id),
            url: Set(url),
        };

        let url_result = ActiveUrlWhitelist::insert(url_whitelist, txn).await?;
        Ok(url_result.id)
    }
}
