use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct CreateAuthorsTable;

#[async_trait::async_trait]
impl MigrationTrait for CreateAuthorsTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Authors::Table)
                    .col(
                        ColumnDef::new(Authors::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Authors::Name).string().not_null())
                    .col(ColumnDef::new(Authors::Email).string())
                    .index(
                        Index::create()
                            .name("idx_authors_email")
                            .col(Authors::Email)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Authors::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Authors {
    Table,
    Id,
    Name,
    Email,
}
