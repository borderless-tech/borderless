use sea_orm::{
    entity::prelude::*,
    ActiveValue::{NotSet, Set},
    DatabaseTransaction,
};

use super::capabilities::ActiveCapabilities;
use super::meta::ActiveMeta;
use super::source::ActiveSource;

use crate::error::Error;

pub type ActivePackage = ActiveModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "packages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub name: String,
    pub app_name: Option<String>,
    pub app_module: Option<String>,
    pub pkg_type: String,
    pub meta_id: u64,
    pub source_id: u64,
    pub capabilities_id: Option<u64>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::source::Entity",
        from = "Column::SourceId",
        to = "super::source::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Sources,

    #[sea_orm(
        belongs_to = "super::capabilities::Entity",
        from = "Column::CapabilitiesId",
        to = "super::capabilities::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    Capabilities,

    #[sea_orm(
        belongs_to = "super::meta::Entity",
        from = "Column::MetaId",
        to = "super::meta::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Meta,
}

impl Related<super::source::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sources.def()
    }
}

impl Related<super::capabilities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Capabilities.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl ActivePackage {
    pub async fn from_model(
        txn: &DatabaseTransaction,
        model: borderless_pkg::WasmPkg,
    ) -> Result<(), Error> {
        let meta_id = ActiveMeta::from_model(txn, model.meta).await?;
        let source_id = ActiveSource::from_model(txn, model.source).await?;

        let capabilities_id = if let Some(capa) = model.capabilities {
            let id = ActiveCapabilities::from_model(txn, capa).await?;
            Some(id)
        } else {
            None
        };

        let package = ActivePackage {
            id: NotSet,
            name: Set(model.name),
            app_name: Set(model.app_name),
            app_module: Set(model.app_module),
            pkg_type: Set(model.pkg_type.to_string()),
            meta_id: Set(meta_id),
            source_id: Set(meta_id),
            capabilities_id: Set(capabilities_id),
            ..Default::default()
        };

        ActivePackage::insert(package, txn).await?;
        Ok(())
    }
}
