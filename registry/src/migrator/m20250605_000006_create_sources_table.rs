use sea_orm_migration::prelude::*;

use super::{
    m20250605_000002_create_registries_table::Registries,
    m20250605_000003_create_gitinfo_table::GitInfo,
};

#[derive(DeriveMigrationName)]
pub struct CreateSourcesTable;

#[async_trait::async_trait]
impl MigrationTrait for CreateSourcesTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Sources::Table)
                    .col(
                        ColumnDef::new(Sources::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Sources::Version).string().not_null())
                    .col(ColumnDef::new(Sources::Digest).string().not_null())
                    .col(ColumnDef::new(Sources::SourceType).string().not_null())
                    .col(ColumnDef::new(Sources::RegistryId).integer())
                    .col(ColumnDef::new(Sources::WasmBlob).binary())
                    .col(ColumnDef::new(Sources::GitInfoId).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_sources_registry")
                            .from(Sources::Table, Sources::RegistryId)
                            .to(Registries::Table, Registries::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_sources_git_info")
                            .from(Sources::Table, Sources::GitInfoId)
                            .to(GitInfo::Table, GitInfo::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .index(
                        Index::create()
                            .name("idx_sources_digest")
                            .col(Sources::Digest)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Sources::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Sources {
    Table,
    Id,
    Version,
    Digest,
    SourceType,
    RegistryId,
    WasmBlob,
    GitInfoId,
}
