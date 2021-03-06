use actix_web::web;
use sqlx::{Pool, Sqlite};

// Change to strings if to much headache
pub struct Link {
    pub uuid: String,
    pub destination: String,
}

impl Link {
    pub async fn find_by_uuid(
        uuid: String,
        pool: web::Data<Pool<Sqlite>>,
    ) -> Result<Link, sqlx::Error> {
        let mut tx = pool.begin().await?;
        let rec = sqlx::query!(
            r#"
                    SELECT * FROM links WHERE uuid = $1
                "#,
            uuid
        )
        .fetch_one(&mut tx)
        .await?;

        Ok(Link {
            uuid: rec.UUID,
            destination: rec.DESTINATION,
        })
    }
    pub async fn update(link: Link, pool: web::Data<Pool<Sqlite>>) -> Result<Link, sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query("UPDATE links SET destination = $2 WHERE uuid = $1;")
            .bind(&link.uuid)
            .bind(&link.destination)
            .execute(&mut tx)
            .await?;

        tx.commit().await?;
        Ok(link)
    }
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
