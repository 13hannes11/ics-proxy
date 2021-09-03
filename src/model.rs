use actix_web::web;
use sqlx::sqlite::SqliteRow;
use sqlx::{Pool, Row, Sqlite};
use url::Url;
use uuid::Uuid;

// Change to strings if to much headache
pub struct Link {
    pub uuid: String,
    pub destination: String,
}

impl Link {
    pub async fn create(link: Link, pool: web::Data<Pool<Sqlite>>) -> Result<Link, sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query("INSERT INTO links (uuid, destination) VALUES ($1, $2);")
            .bind(&link.uuid)
            .bind(&link.destination)
            .execute(&mut tx)
            .await?;

        tx.commit().await?;
        Ok(link)
    }
}