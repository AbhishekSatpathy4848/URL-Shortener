use redis::{aio::MultiplexedConnection, AsyncCommands, RedisError};
use dotenv::dotenv;
use std::env;


pub async fn establish_connection() -> MultiplexedConnection{
    dotenv().ok();
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    let client = redis::Client::open(redis_url).unwrap();
    return client.get_multiplexed_tokio_connection().await.unwrap();
}

pub async fn write_to_cache(short_url: &str, original_url: &str, connection: &mut rocket::futures::lock::MutexGuard<'_, MultiplexedConnection>) -> Result<(), redis::RedisError> {
    connection.set(short_url, original_url).await?;
    dbg!("Added {},{} to cache", &short_url, &original_url);
    Ok(())
}

pub async fn read_from_cache(short_url: &str, connection: &mut rocket::futures::lock::MutexGuard<'_, MultiplexedConnection>) -> Result<String, RedisError> {
    let original_url = connection.get(short_url).await;
    if let Err(redis_error) = original_url {
        dbg!("Cache miss for {}", short_url);
        return Err(redis_error);
    }
    dbg!("Read {},{} from cache", &short_url, &original_url);
    Ok(original_url.unwrap())
}