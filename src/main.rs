mod db;
mod cache;

use dotenv::dotenv;
use redis::aio::MultiplexedConnection;
use rocket::futures::lock::Mutex;
use rocket::http::Status;
use serde_json::json;
use std::env;
use std::sync::Arc;
use base62::DecodeError;
use rocket::{get, launch, post, routes, State};
use rocket::serde::json::{Json, Value};
use serde::Deserialize;
use snowflake::SnowflakeIdGenerator;
use sqlx::PgPool;
use rocket::response::Redirect;

#[derive(Deserialize)]
struct ShortenUrlBody {
    original_url: String,
}

#[derive(Deserialize)]
struct ShortUrlClicksBody {
    short_url: String,
}

fn encode_to_base_62(unique_id: u64) -> String {
    base62::encode(unique_id)
}

fn decode_from_base_62(short_url: &str) -> Result<u128,DecodeError> {
    base62::decode(short_url)
}

fn get_unique_id() -> i64 {
   let mut id_generator = SnowflakeIdGenerator::new(1, 1);
   id_generator.real_time_generate()
}

fn get_domain_name() -> String{
    dotenv().ok();
    let domain_name = env::var("DOMAIN_NAME").expect("DOMAIN_NAME must be set");
    return domain_name;
}

fn append_domain_name_to(short_url: &str) -> String{
    let domain_name = get_domain_name();
    return format!("{}/{}", domain_name, short_url);
}

fn get_port_number() -> u16 {
    dotenv().ok();
    let port = env::var("PORT").expect("PORT must be set");
    return port.parse::<u16>().unwrap();
}

fn get_ip_address() -> String {
    dotenv().ok();
    let ip_address = env::var("IP_ADDRESS").expect("IP_ADDRESS must be set");
    return ip_address;
}

fn parse_url(url:&str) -> Result<url::Url, url::ParseError> {
    return url::Url::parse(url);
}

fn check_valid_url(url: &str) -> bool {
    match parse_url(url) {
        Ok(parsed_url) => {
            if parsed_url.scheme() == "https" || parsed_url.scheme() == "http" {
               return true;
            }
            return false;
        }
        Err(_) => {
            return false;
        }
    }
}

fn attach_base_if_not_present(url: String) -> String {
    let mut final_url = url;
    if !(final_url.starts_with("http://") || final_url.starts_with("https://")){
        final_url = format!("https://{}", &final_url);
    }
    
    final_url
}

#[post("/create", format="json", data="<shorten_url_body>")]
async fn generate_short_url(db_connection: &State<PgPool>, shorten_url_body: Json<ShortenUrlBody>) -> Result<Value, Status> {
    let mut final_original_url:String = shorten_url_body.0.original_url.clone();

    final_original_url = attach_base_if_not_present(final_original_url);

    if !check_valid_url(&final_original_url) {
        return Err(Status::BadRequest);
    }

    let mut short_url = db::get_short_url_if_exists(&final_original_url, db_connection).await;
    if let Err(_) = short_url {

        let unique_id = get_unique_id();
        let generated_short_url =  encode_to_base_62(unique_id as u64);
        
        db::add_url_entry(unique_id, &final_original_url, &generated_short_url, db_connection).await.unwrap();

        short_url = Ok(generated_short_url);
    }
    return Ok(json!({"short_url": format!("{}",append_domain_name_to(&short_url.unwrap())), "long_url" : final_original_url}));
}

#[get("/<short_url_hash>")]
async fn redirect_to_original_url(connection_pool: &State<PgPool>, cache_connection: &State<Arc<Mutex<MultiplexedConnection>>>, short_url_hash: &str) -> Result<Redirect, Status> {
    let mut cache_connection_mutex: rocket::futures::lock::MutexGuard<'_, MultiplexedConnection> = cache_connection.lock().await;
    let original_url_cache = cache::read_from_cache(short_url_hash, &mut cache_connection_mutex).await;

    if let Ok(url) = original_url_cache {
        let _ = db::increment_url_visit(short_url_hash, connection_pool).await;

        return Ok(Redirect::found(url));
    }

    let original_url_db = db::get_original_url(short_url_hash, connection_pool).await;

    if original_url_db.is_err() {
        return Err(Status::NotFound);
    }

    let url = original_url_db.unwrap();
    cache::write_to_cache(short_url_hash, &url, &mut cache_connection_mutex).await.unwrap();
    let _ = db::increment_url_visit(short_url_hash, connection_pool).await;

    return Ok(Redirect::found(url));
}

#[post("/visits", format="json", data="<short_url_clicks_body>")]
async fn get_number_of_visits(short_url_clicks_body: Json<ShortUrlClicksBody>, pool: &State<PgPool>) -> Result<Value, Status>{
    
    let mut updated_short_url = short_url_clicks_body.0.short_url.clone();
    updated_short_url = attach_base_if_not_present(updated_short_url);

    let result = parse_url(&updated_short_url);
    if let Err(_) = result {
        return Err(Status::BadRequest);
    }

    let short_url_hash = short_url_clicks_body.0.short_url.split("/").last();
    if let None = short_url_hash {
        return Err(Status::BadRequest);
    }

    let visits = db::get_url_visit(short_url_hash.unwrap(), pool).await;
    if let Err(_) = visits {
        return Err(Status::NotFound);
    }
    return Ok(json!({"visits": visits.unwrap()}));
}

#[launch]
pub async fn rocket() -> _ {
    let db_connection: PgPool = db::establish_connection().await;
    let cache_connection: redis::aio::MultiplexedConnection = cache::establish_connection().await;
    let cache_connection_arc = std::sync::Arc::new(Mutex::new(cache_connection));
    let port = get_port_number();
    let ip_address = get_ip_address();
    rocket::build()
    .configure(rocket::Config::figment().merge(("port", port)).merge(("address", ip_address)))
    .manage(db_connection)
    .manage(cache_connection_arc)
    .mount("/",routes![generate_short_url, redirect_to_original_url, get_number_of_visits])
}