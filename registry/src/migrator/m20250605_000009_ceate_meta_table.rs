use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct CreateMetaTable;

#[async_trait::async_trait]
impl MigrationTrait for CreateMetaTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Meta::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Meta::Id)
                            .big_unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Meta::Description).text().null())
                    .col(ColumnDef::new(Meta::Documentation).text().null())
                    .col(ColumnDef::new(Meta::License).text().null())
                    .col(ColumnDef::new(Meta::Repository).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Meta::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Meta {
    Table,
    Id,
    Description,
    Documentation,
    License,
    Repository,
}
