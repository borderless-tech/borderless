mod m20250605_000001_create_authors_table;
mod m20250605_000002_create_registries_table;
mod m20250605_000003_create_gitinfo_table;
mod m20250605_000004_ceate_capabilities_table;
mod m20250605_000005_create_url_whitelists_table;
mod m20250605_000006_create_sources_table;
mod m20250605_000007_create_pkgs_table;
mod m20250605_000008_create_package_authors_tables;
mod m20250605_000009_ceate_meta_table;
mod m20250605_000010_create_index_table;

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250605_000001_create_authors_table::CreateAuthorsTable),
            Box::new(m20250605_000002_create_registries_table::CreateRegistriesTable),
            Box::new(m20250605_000003_create_gitinfo_table::CreateGitInfoTable),
            Box::new(m20250605_000004_ceate_capabilities_table::CreateCapabilitiesTable),
            Box::new(m20250605_000005_create_url_whitelists_table::CreateUrlWhitelistTable),
            Box::new(m20250605_000006_create_sources_table::CreateSourcesTable),
            Box::new(m20250605_000007_create_pkgs_table::CreatePackageTable),
            Box::new(m20250605_000008_create_package_authors_tables::CreatePackageAuthorsTable),
            Box::new(m20250605_000009_ceate_meta_table::CreateMetaTable),
            Box::new(m20250605_000010_create_index_table::CreateRegistryIndexTable),
        ]
    }
}
