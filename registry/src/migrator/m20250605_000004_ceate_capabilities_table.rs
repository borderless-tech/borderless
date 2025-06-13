use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct CreateCapabilitiesTable;

#[async_trait::async_trait]
impl MigrationTrait for CreateCapabilitiesTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Capabilities::Table)
                    .col(
                        ColumnDef::new(Capabilities::Id)
                            .big_unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Capabilities::Network)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Capabilities::Websocket)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Capabilities::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Capabilities {
    Table,
    Id,
    Network,
    Websocket,
}
