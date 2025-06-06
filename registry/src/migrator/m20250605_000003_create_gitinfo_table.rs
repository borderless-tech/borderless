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
                    .col(ColumnDef::new(GitInfo::CommitHash).string())
                    .col(ColumnDef::new(GitInfo::Branch).string())
                    .col(ColumnDef::new(GitInfo::Repository).string())
                    .col(ColumnDef::new(GitInfo::Tag).string())
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
    CommitHash,
    Branch,
    Repository,
    Tag,
}
