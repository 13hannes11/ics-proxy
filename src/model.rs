use actix_web::web;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use std::time::SystemTime;

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
        let now = <SystemTime as Into<DateTime<Utc>>>::into(SystemTime::now()).to_rfc3339();
        let mut tx = pool.begin().await?;
        println!("{now} find by uuid {uuid}");
        let rec = sqlx::query!(
            r#"
                    SELECT * FROM links WHERE uuid = $1
                "#,
            uuid
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            r#" UPDATE links SET last_used = $1 WHERE uuid = $2"#,
            now,
            uuid,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

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
            .execute(&mut *tx)
            .await?;
        let now = <SystemTime as Into<DateTime<Utc>>>::into(SystemTime::now()).to_rfc3339();
        println!("{} update uuid {}", now, link.uuid);
        tx.commit().await?;
        Ok(link)
    }
    pub async fn create(link: Link, pool: web::Data<Pool<Sqlite>>) -> Result<Link, sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query("INSERT INTO links (uuid, destination) VALUES ($1, $2);")
            .bind(&link.uuid)
            .bind(&link.destination)
            .execute(&mut *tx)
            .await?;
        let now = <SystemTime as Into<DateTime<Utc>>>::into(SystemTime::now()).to_rfc3339();
        println!("{} create uuid {}", now, link.uuid);
        tx.commit().await?;
        Ok(link)
    }
}

pub async fn delete_old_entries(pool: &Pool<Sqlite>) -> Result<u64, sqlx::Error> {
    let ninety_days_ago = Utc::now() - Duration::days(90);
    let ninety_days_ago_str = ninety_days_ago.to_rfc3339();

    let mut tx = pool.begin().await?;
    println!(
        "{} deleting entries older than {}",
        Utc::now().to_rfc3339(),
        ninety_days_ago_str
    );

    let result = sqlx::query!(
        r#"
            DELETE FROM links
            WHERE last_used < $1
        "#,
        ninety_days_ago_str
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(result.rows_affected())
}
