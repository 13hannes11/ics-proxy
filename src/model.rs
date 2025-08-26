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
    pub created_at: Option<String>,
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
            created_at: rec.created_at,
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
        let now = <SystemTime as Into<DateTime<Utc>>>::into(SystemTime::now()).to_rfc3339();
        sqlx::query(
            "INSERT INTO links (uuid, destination, created_at) VALUES ($1, $2, $3);",
        )
        .bind(&link.uuid)
        .bind(&link.destination)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        println!("{} create uuid {}", now, link.uuid);
        tx.commit().await?;
        Ok(Link {
            created_at: Some(now.to_string()),
            ..link
        })
    }

    pub async fn delete(uuid: String, pool: web::Data<Pool<Sqlite>>) -> Result<u64, sqlx::Error> {
        let mut tx = pool.begin().await?;
        let result = sqlx::query("DELETE FROM links WHERE uuid = $1;")
            .bind(&uuid)
            .execute(&mut *tx)
            .await?;
        let now = <SystemTime as Into<DateTime<Utc>>>::into(SystemTime::now()).to_rfc3339();
        println!("{} delete uuid {}", now, uuid);
        tx.commit().await?;
        Ok(result.rows_affected())
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

pub async fn set_created_at_for_old_entries(pool: &Pool<Sqlite>) -> Result<u64, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;
    println!("{} setting created_at for old entries", now);

    let result = sqlx::query!(
        r#"
            UPDATE links
            SET created_at = $1
            WHERE created_at IS NULL
        "#,
        now
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(result.rows_affected())
}
