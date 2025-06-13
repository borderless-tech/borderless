use sea_orm_migration::prelude::*;

use super::m20250605_000004_ceate_capabilities_table::Capabilities;

#[derive(DeriveMigrationName)]
pub struct CreateUrlWhitelistTable;

#[async_trait::async_trait]
impl MigrationTrait for CreateUrlWhitelistTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UrlWhitelist::Table)
                    .col(
                        ColumnDef::new(UrlWhitelist::Id)
                            .big_unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UrlWhitelist::CapabilityId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(UrlWhitelist::Url).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_url_whitelist_capability")
                            .from(UrlWhitelist::Table, UrlWhitelist::CapabilityId)
                            .to(Capabilities::Table, Capabilities::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UrlWhitelist::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum UrlWhitelist {
    Table,
    Id,
    CapabilityId,
    Url,
}
