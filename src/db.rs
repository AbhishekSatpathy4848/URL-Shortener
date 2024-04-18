use dotenv::dotenv;
use sqlx::{postgres::PgQueryResult, Error, PgPool};
use std::env;
use sqlx::Row;

pub async fn establish_connection() -> PgPool{
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = sqlx::postgres::PgPool::connect(&database_url).await.unwrap();
    return pool;
}

pub async fn add_url_entry(unique_id: i64, original_url: &str, short_url: &str, pool: &PgPool) -> Result<PgQueryResult, Error>{
    let row: Result<PgQueryResult, Error> = sqlx::query("INSERT INTO url_table (unique_id, original_url, short_url) VALUES ($1, $2, $3)")
        .bind(unique_id)
        .bind(original_url)
        .bind(short_url)
        .execute(pool).await;  
    row
}

pub async fn get_original_url(short_url: &str, pool: &PgPool) -> Result<String, Error>{
    dbg!("Fetching original url for short url: {}", short_url);
    let row = sqlx::query("SELECT original_url FROM url_table WHERE short_url = $1")
        .bind(short_url)
        .fetch_all(pool).await;
    
    let rows = row.unwrap();

    if rows.len() == 0 {
        return Err(Error::RowNotFound);
    }

    let original_url = rows[0].get(0);

    dbg!("Original url for short url: {} is {}", &short_url, &original_url);

    return Ok(original_url);
}


pub async fn increment_url_visit(short_url: &str, pool: &PgPool) -> Result<PgQueryResult, Error> {
    let result = sqlx::query("UPDATE url_table SET clicks = clicks + 1 WHERE short_url = $1")
    .bind(short_url)
    .execute(pool).await;

    return result;
}

pub async fn get_url_visit(short_url: &str, pool: &PgPool) -> Result<i32, Error> {
    let result = sqlx::query("SELECT clicks FROM url_table WHERE short_url = $1")
    .bind(short_url)
    .fetch_all(pool).await;

    let rows = result.unwrap();
    if rows.len() == 0 {
        return Err(Error::RowNotFound);
    }

    return Ok(rows[0].get(0));
}
