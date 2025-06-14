use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct CreateRegistryIndexTable;

#[async_trait::async_trait]
impl MigrationTrait for CreateRegistryIndexTable {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RegistryIndex::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RegistryIndex::Id)
                            .big_unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RegistryIndex::Registry).string().null())
                    .col(ColumnDef::new(RegistryIndex::Namespace).string().not_null())
                    .col(ColumnDef::new(RegistryIndex::Repository).string())
                    .col(ColumnDef::new(RegistryIndex::Tag).string().not_null())
                    .col(ColumnDef::new(RegistryIndex::Yank).boolean().not_null())
                    .col(
                        ColumnDef::new(RegistryIndex::Deprecated)
                            .boolean()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RegistryIndex::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_unique_package_identity")
                    .table(RegistryIndex::Table)
                    .col(RegistryIndex::Registry)
                    .col(RegistryIndex::Namespace)
                    .col(RegistryIndex::Tag)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_repository_lookup")
                    .table(RegistryIndex::Table)
                    .col(RegistryIndex::Repository)
                    .col(RegistryIndex::Namespace)
                    .col(RegistryIndex::Tag)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_namespace_listing")
                    .table(RegistryIndex::Table)
                    .col(RegistryIndex::Namespace)
                    .col(RegistryIndex::Repository)
                    .col(RegistryIndex::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tag_listing")
                    .table(RegistryIndex::Table)
                    .col(RegistryIndex::Namespace)
                    .col(RegistryIndex::Repository)
                    .col(RegistryIndex::Yank)
                    .col(RegistryIndex::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_available_packages")
                    .table(RegistryIndex::Table)
                    .col(RegistryIndex::Yank)
                    .col(RegistryIndex::Deprecated)
                    .col(RegistryIndex::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_repository_search")
                    .table(RegistryIndex::Table)
                    .col(RegistryIndex::Repository)
                    .col(RegistryIndex::Yank)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_temporal_queries")
                    .table(RegistryIndex::Table)
                    .col(RegistryIndex::CreatedAt)
                    .col(RegistryIndex::Yank)
                    .col(RegistryIndex::Deprecated)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let indexes = [
            "idx_unique_package_identity",
            "idx_repository_lookup",
            "idx_namespace_listing",
            "idx_tag_listing",
            "idx_available_packages",
            "idx_repository_search",
            "idx_temporal_queries",
        ];

        for index_name in indexes {
            manager
                .drop_index(
                    Index::drop()
                        .name(index_name)
                        .table(RegistryIndex::Table)
                        .to_owned(),
                )
                .await
                .ok();
        }

        manager
            .drop_table(Table::drop().table(RegistryIndex::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum RegistryIndex {
    Table,
    Id,
    Registry,
    Namespace,
    Repository,
    Tag,
    Yank,
    Deprecated,
    CreatedAt,
}
