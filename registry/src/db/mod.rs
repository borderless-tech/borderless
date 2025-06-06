mod entities;

use sea_orm::{Database, DatabaseConnection, DbErr};
use sea_orm_migration::prelude::*;

use crate::migrator::Migrator;

async fn setup_database(db_url: &str, db_name: &str) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect(db_url).await?;
    Ok(db)
}

async fn apply_migrations(conn: &DatabaseConnection) -> Result<(), DbErr> {
    let schema_manager = SchemaManager::new(conn);
    Migrator::up(conn, None).await?;

    assert!(schema_manager.has_table("authors").await?);
    assert!(schema_manager.has_table("registries").await?);
    assert!(schema_manager.has_table("git_info").await?);
    assert!(schema_manager.has_table("capabilities").await?);
    assert!(schema_manager.has_table("url_white_list").await?);
    assert!(schema_manager.has_table("sources").await?);
    assert!(schema_manager.has_table("packages").await?);
    assert!(schema_manager.has_table("package_authors").await?);

    Ok(())
}
