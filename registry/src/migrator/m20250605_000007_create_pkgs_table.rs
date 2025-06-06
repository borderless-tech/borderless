use sea_orm_migration::prelude::*;

use super::{
    m20250605_000004_ceate_capabilities_table::Capabilities,
    m20250605_000006_create_sources_table::Sources,
};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m_20250605_000001_create_pkg_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
                    .col(ColumnDef::new(Packages::PkgType).string().not_null()) // 'contract' or 'agent'
                    .col(ColumnDef::new(Packages::Description).text())
                    .col(ColumnDef::new(Packages::Documentation).string())
                    .col(ColumnDef::new(Packages::License).string())
                    .col(ColumnDef::new(Packages::Repository).string())
                    .col(ColumnDef::new(Packages::SourceId).integer().not_null())
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
                    .index(
                        Index::create()
                            .name("idx_packages_name")
                            .col(Packages::Name),
                    )
                    .index(
                        Index::create()
                            .name("idx_packages_app")
                            .col(Packages::AppName)
                            .col(Packages::AppModule),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
    Description,
    Documentation,
    License,
    Repository,
    SourceId,
    CapabilitiesId,
    CreatedAt,
    UpdatedAt,
}
