use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct CreateRegistriesTable;

#[async_trait::async_trait]
impl MigrationTrait for CreateRegistriesTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Registries::Table)
                    .col(
                        ColumnDef::new(Registries::Id)
                            .big_unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Registries::RegistryType).string())
                    .col(ColumnDef::new(Registries::Hostname).string().not_null())
                    .col(ColumnDef::new(Registries::Namespace).string().not_null())
                    .index(
                        Index::create()
                            .name("idx_registries_unique")
                            .col(Registries::Hostname)
                            .col(Registries::Namespace)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Registries::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Registries {
    Table,
    Id,
    RegistryType,
    Hostname,
    Namespace,
}
