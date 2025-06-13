use sea_orm::{
    entity::prelude::*,
    ActiveValue::{NotSet, Set},
    DatabaseTransaction,
};

use crate::error::Error;

pub type ActiveRegistry = ActiveModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "registries")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub registry_type: Option<String>,
    pub hostname: String,
    pub namespace: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveRegistry {
    pub async fn from_model(
        txn: &DatabaseTransaction,
        registry: borderless_pkg::Registry,
    ) -> Result<u64, Error> {
        let registry = ActiveRegistry {
            id: NotSet,
            registry_type: Set(registry.registry_type),
            hostname: Set(registry.registry_hostname),
            namespace: Set(registry.namespace),
        };

        let registry_result = ActiveRegistry::insert(registry, txn).await?;
        Ok(registry_result.id)
    }
}
