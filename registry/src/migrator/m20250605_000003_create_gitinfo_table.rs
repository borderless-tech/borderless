use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct CreateGitInfoTable;

#[async_trait::async_trait]
impl MigrationTrait for CreateGitInfoTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GitInfo::Table)
                    .col(
                        ColumnDef::new(GitInfo::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GitInfo::CommitHashShort).not_null())
                    .col(ColumnDef::new(GitInfo::CommitsPastTag).string())
                    .col(ColumnDef::new(GitInfo::Tag).string())
                    .col(ColumnDef::new(GitInfo::Dirty).not_null().default(false))
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GitInfo::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum GitInfo {
    Table,
    Id,
    CommitHashShort,
    CommitsPastTag,
    Tag,
    Dirty,
}
