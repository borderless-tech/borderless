use sea_orm::{entity::prelude::*, ActiveValue::NotSet, DatabaseTransaction, Set};

use crate::{db::entities::git_info::ActiveGitInfo, error::Error};

use super::registry::ActiveRegistry;

pub type ActiveSource = ActiveModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sources")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub version: String,
    pub digest: String,
    pub wasm_blob: Option<Vec<u8>>,
    pub registry_id: Option<u64>,
    pub git_info_id: Option<u64>,
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

impl ActiveSource {
    pub async fn from_model(
        txn: &DatabaseTransaction,
        source: borderless_pkg::Source,
    ) -> Result<u64, Error> {
        let (wasm, git_info, registry) = match source.code {
            borderless_pkg::SourceType::Wasm { wasm, git_info } => (Some(wasm), git_info, None),
            borderless_pkg::SourceType::Registry { registry } => (None, None, Some(registry)),
        };

        let git_id = if let Some(git) = git_info {
            let id = ActiveGitInfo::from_model(txn, git).await?;
            Some(id)
        } else {
            None
        };

        let registry_id = if let Some(registry) = registry {
            let id = ActiveRegistry::from_model(txn, registry).await?;
            Some(id)
        } else {
            None
        };

        let src = ActiveSource {
            id: NotSet,
            version: Set(source.version.to_string()),
            digest: Set(source.digest.to_string()),
            wasm_blob: Set(wasm),
            git_info_id: Set(git_id),
            registry_id: Set(registry_id),
        };

        let src_result = ActiveSource::insert(src, txn).await?;
        Ok(src_result.id)
    }
}
