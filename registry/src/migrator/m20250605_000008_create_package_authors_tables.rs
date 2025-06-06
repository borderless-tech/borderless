use sea_orm_migration::prelude::*;

use super::{
    m20250605_000001_create_authors_table::Authors, m20250605_000007_create_pkgs_table::Packages,
};

#[derive(DeriveMigrationName)]
pub struct CreatePackageAuthorsTable;

#[async_trait::async_trait]
impl MigrationTrait for CreatePackageAuthorsTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PackageAuthors::Table)
                    .col(
                        ColumnDef::new(PackageAuthors::PackageId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PackageAuthors::AuthorId)
                            .integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_package_authors")
                            .col(PackageAuthors::PackageId)
                            .col(PackageAuthors::AuthorId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_package_authors_package")
                            .from(PackageAuthors::Table, PackageAuthors::PackageId)
                            .to(Packages::Table, Packages::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_package_authors_author")
                            .from(PackageAuthors::Table, PackageAuthors::AuthorId)
                            .to(Authors::Table, Authors::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PackageAuthors::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum PackageAuthors {
    Table,
    PackageId,
    AuthorId,
}
