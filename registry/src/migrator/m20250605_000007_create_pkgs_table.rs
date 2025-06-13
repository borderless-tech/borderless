use super::{
    m20250605_000004_ceate_capabilities_table::Capabilities,
    m20250605_000006_create_sources_table::Sources, m20250605_000009_ceate_meta_table::Meta,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct CreatePackageTable;

#[async_trait::async_trait]
impl MigrationTrait for CreatePackageTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Erst die Tabelle erstellen
        manager
            .create_table(
                Table::create()
                    .table(Packages::Table)
                    .col(
                        ColumnDef::new(Packages::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Packages::Name).string().not_null())
                    .col(ColumnDef::new(Packages::AppName).string())
                    .col(ColumnDef::new(Packages::AppModule).string())
                    .col(ColumnDef::new(Packages::PkgType).string().not_null())
                    .col(ColumnDef::new(Packages::SourceId).integer().not_null())
                    .col(ColumnDef::new(Packages::MetaId).integer().not_null())
                    .col(ColumnDef::new(Packages::CapabilitiesId).integer())
                    .col(ColumnDef::new(Packages::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Packages::UpdatedAt).timestamp().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_packages_source")
                            .from(Packages::Table, Packages::SourceId)
                            .to(Sources::Table, Sources::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_packages_capabilities")
                            .from(Packages::Table, Packages::CapabilitiesId)
                            .to(Capabilities::Table, Capabilities::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_packages_meta")
                            .from(Packages::Table, Packages::MetaId)
                            .to(Meta::Table, Meta::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Dann die Indizes separat erstellen (besser für SQLite)
        manager
            .create_index(
                Index::create()
                    .name("idx_packages_name")
                    .table(Packages::Table)
                    .col(Packages::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_packages_app")
                    .table(Packages::Table)
                    .col(Packages::AppName)
                    .col(Packages::AppModule)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Indizes löschen
        manager
            .drop_index(
                Index::drop()
                    .name("idx_packages_app")
                    .table(Packages::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_packages_name")
                    .table(Packages::Table)
                    .to_owned(),
            )
            .await?;

        // Tabelle löschen
        manager
            .drop_table(Table::drop().table(Packages::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Packages {
    Table,
    Id,
    Name,
    AppName,
    AppModule,
    PkgType,
    MetaId,
    SourceId,
    CapabilitiesId,
    CreatedAt,
    UpdatedAt,
}
