use sea_orm::{
    entity::prelude::*,
    ActiveValue::{NotSet, Set},
    DatabaseTransaction,
};

use crate::error::Error;

pub type ActiveCapabilities = ActiveModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "capabilities")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub network: bool,
    pub websocket: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveCapabilities {
    pub async fn from_model(
        txn: &DatabaseTransaction,
        capabilities: borderless_pkg::Capabilities,
    ) -> Result<u64, Error> {
        let capabilities = ActiveCapabilities {
            id: NotSet,
            network: Set(capabilities.network),
            websocket: Set(capabilities.websocket),
        };

        let capabilities_result = ActiveCapabilities::insert(capabilities, txn).await?;
        Ok(capabilities_result.id)
    }
}
