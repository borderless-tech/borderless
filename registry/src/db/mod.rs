pub mod entities;

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use sea_orm_migration::prelude::*;

use crate::migrator::Migrator;

use tracing::{debug, error, info, instrument};

#[instrument(err)]
pub async fn setup_database(db_url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new("sqlite::memory:");

    // Turn this on for detailed database loging
    opt.sqlx_logging(false);
    let db = Database::connect(opt).await?;

    let migrations = Migrator::migrations();
    for (index, _migration) in migrations.iter().enumerate() {
        let step = (index + 1) as u32;
        match Migrator::up(&db, Some(step)).await {
            Ok(_) => {}
            Err(e) => {
                error!("❌ FAILED at migration step {}", step);
                debug!("Error: {}", e);

                if let Some(failed_migration) = migrations.get(index) {
                    error!("Failed migration name: {}", failed_migration.name());
                }

                return Err(e.into());
            }
        }
    }

    info!("Check database schema!");
    let schema_manager = SchemaManager::new(&db);
    assert!(schema_manager.has_table("authors").await?);
    assert!(schema_manager.has_table("registries").await?);
    assert!(schema_manager.has_table("git_info").await?);
    assert!(schema_manager.has_table("capabilities").await?);
    assert!(schema_manager.has_table("url_whitelist").await?);
    assert!(schema_manager.has_table("sources").await?);
    assert!(schema_manager.has_table("packages").await?);
    assert!(schema_manager.has_table("meta").await?);
    assert!(schema_manager.has_table("package_authors").await?);

    Ok(db)
}
