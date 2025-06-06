use sea_orm::{Database, DbErr};

async fn setup_database(db_url: &str, db_name: &str) -> Result<Database, DbErr> {
    let db = Database::connect(db_url).await?;
    todo!()
}
