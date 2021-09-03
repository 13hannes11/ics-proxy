use std::collections::HashMap;
use url::Url;

use actix_web::http::StatusCode;
use actix_web::{error, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder, Result};
use sqlx::{Pool, Sqlite, SqlitePool};
use tera::Tera;
use uuid::Uuid;

extern crate dotenv;
use dotenv::dotenv;
use std::env;

mod model;
use model::Link;

const REDIRECT_TIMEOUT_S: i32 = 2;

#[derive(Clone)]
struct CONFIG {
    root: String,
}

async fn make_ics_request(req: HttpRequest, db_pool: web::Data<Pool<Sqlite>>) -> impl Responder {
    let id = req.match_info().get("id").unwrap_or("");

    match Uuid::parse_str(id) {
        Ok(uuid) => match Link::find_by_uuid(uuid.to_string(), db_pool).await {
            Ok(link) => match reqwest::blocking::get(link.destination) {
                Ok(r) => match r.text() {
                    Ok(res) => HttpResponse::Ok().content_type("text/calendar").body(res),
                    Err(_) => HttpResponse::Ok()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .finish(),
                },
                Err(_) => HttpResponse::Ok()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .finish(),
            },
            Err(_) => HttpResponse::Ok().status(StatusCode::NOT_FOUND).finish(),
        },
        Err(_) => HttpResponse::Ok().status(StatusCode::BAD_REQUEST).finish(),
    }
}

fn error_page(tmpl: web::Data<tera::Tera>, msg: String) -> Result<HttpResponse, Error> {
    let mut ctx = tera::Context::new();
    ctx.insert("message", &msg);
    let s = tmpl
        .render("error.html", &ctx)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

// This is the new edit page:
async fn edit_page(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
    db_pool: web::Data<Pool<Sqlite>>,
    conf: web::Data<CONFIG>,
) -> Result<HttpResponse, Error> {
    // one uuid: 9228c1a4-8956-4f1c-8b5f-53cc575bd78
    if let Some(uuid_str) = query.get("uuid") {
        match Uuid::parse_str(uuid_str) {
            Ok(uuid) => match Link::find_by_uuid(uuid.to_string(), db_pool).await {
                Ok(link) => {
                    let mut ctx = tera::Context::new();
                    ctx.insert("link", &link.destination);
                    ctx.insert("uuid", &link.uuid);
                    ctx.insert("root", &conf.root);
                    let s = tmpl
                        .render("edit.html", &ctx)
                        .map_err(|_| error::ErrorInternalServerError("Template error"))?;

                    Ok(HttpResponse::Ok().content_type("text/html").body(s))
                }
                Err(err) => error_page(tmpl, format!("db error: {}", err.to_string())),
            },
            Err(err) => error_page(tmpl, format!("uuid parsing error: {}", err.to_string())),
        }
    } else {
        error_page(tmpl, "uuid parameter missing".to_string())
    }
}

fn redirect_to_page(
    tmpl: web::Data<tera::Tera>,
    message: String,
    link: String,
    time_s: i32,
) -> Result<HttpResponse, Error> {
    let mut ctx = tera::Context::new();
    ctx.insert("message", &message);
    ctx.insert("link", &link);
    ctx.insert("time", &time_s);
    let s = tmpl
        .render("redirect.html", &ctx)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

fn redirect_to_edit_page(
    tmpl: web::Data<tera::Tera>,
    message: String,
    uuid: Uuid,
    time_s: i32,
) -> Result<HttpResponse, Error> {
    let mut ctx = tera::Context::new();
    ctx.insert("message", &message);
    let link = format!("/edit?uuid={}", uuid.to_string());
    ctx.insert("time", &time_s);
    redirect_to_page(tmpl, message, link, time_s)
}

/*
fn redirect_to_index_page(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
    message: String,
    uuid: Uuid,
    time_s: i32,
) -> Result<HttpResponse, Error> {
    // TODO: add option to prefill link
}
*/

async fn edit_process(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
    db_pool: web::Data<Pool<Sqlite>>,
    config: web::Data<CONFIG>,
) -> Result<HttpResponse, Error> {
    // TODO: implement handling
    if let Some(uuid_str) = query.get("uuid") {
        if let Some(destination) = query.get("link") {
            if destination.starts_with(&config.root) {
                return error_page(tmpl, "url cannot contain url of ics-proxy".to_string());
            };

            if let Err(_) = Url::parse(destination) {
                return error_page(tmpl, "could not parse url".to_string());
            }

            match Uuid::parse_str(uuid_str) {
                Ok(uuid) => {
                    let link = Link {
                        uuid: uuid.to_string(),
                        destination: destination.to_string(),
                    };
                    match Link::update(link, db_pool).await {
                        Ok(_) => redirect_to_edit_page(
                            tmpl,
                            "Edit successful!".to_string(),
                            uuid,
                            REDIRECT_TIMEOUT_S,
                        ),
                        Err(err) => error_page(tmpl, format!("db error: {}", err.to_string())),
                    }
                }
                Err(err) => error_page(tmpl, format!("uuid parsing error: {}", err.to_string())),
            }
        } else {
            error_page(tmpl, "link parameter missing".to_string())
        }
    } else {
        error_page(tmpl, "uuid parameter missing".to_string())
    }
}

async fn index_process(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
    db_pool: web::Data<Pool<Sqlite>>,
    config: web::Data<CONFIG>,
) -> Result<HttpResponse, Error> {
    if query.get("create").is_some() {
        let uuid = Uuid::new_v4();
        // TODO: add actuall logic and use proper uuid
        match query.get("link") {
            // TODO: actually parse link to url to make sure its valid
            Some(destination) => {
                if destination.starts_with(&config.root) {
                    return error_page(tmpl, "url cannot contain url of ics-proxy".to_string());
                };

                if let Err(_) = Url::parse(destination) {
                    return error_page(tmpl, "could not parse url".to_string());
                }

                let insert_link = Link {
                    uuid: uuid.to_string(),
                    destination: destination.to_string(),
                };

                match Link::create(insert_link, db_pool).await {
                    Ok(link) => match Uuid::parse_str(&link.uuid) {
                        Ok(uuid) => redirect_to_edit_page(
                            tmpl,
                            "Create was successful".to_string(),
                            uuid,
                            REDIRECT_TIMEOUT_S,
                        ),
                        Err(e) => error_page(tmpl, format!("uuid parsing error {}", e.to_string())),
                    },
                    // TODO: actually redirect to index page to try again
                    Err(e) => error_page(tmpl, format!("db error: {}", e.to_string())),
                }
            }
            None => {
                // TODO: actually redirect back to index page
                error_page(
                    tmpl,
                    "link attribute not set please enter a link".to_string(),
                )
            }
        }
    } else if query.get("edit").is_some() {
        match query.get("link") {
            Some(link) => {
                // Splitting string and getting uuid, alternatively pretend whole string is uuid
                let vec: Vec<&str> = link.split("/").collect();

                let mut uuid_str = link.to_string();
                if vec.len() > 1 {
                    uuid_str = match vec.get(vec.len() - 2) {
                        Some(s) => s.to_string(),
                        None => link.to_string(),
                    };
                }

                match Uuid::parse_str(&uuid_str) {
                    Ok(uuid) => redirect_to_edit_page(
                        tmpl,
                        "Got uuid from submission!".to_string(),
                        uuid,
                        REDIRECT_TIMEOUT_S,
                    ),
                    // TODO: actually redirect back to index page
                    Err(e) => error_page(tmpl, format!("could not parse uuid: {}", e.to_string())),
                }
            }
            None => {
                // TODO: actually redirect back to index page
                error_page(
                    tmpl,
                    "link attribute not set please enter a link".to_string(),
                )
            }
        }
    } else {
        error_page(tmpl, "missing create or edit form submission!".to_string())
    }
}

// store tera template in application state
async fn index(tmpl: web::Data<tera::Tera>) -> Result<HttpResponse, Error> {
    // TODO: add option to prefill link with parameter

    let s = tmpl
        .render("index.html", &tera::Context::new())
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");

    dotenv().ok();

    let database_url = match std::env::var("DATABASE_URL") {
        Ok(var) => var,
        Err(e) => panic!("{}", e.to_string()),
    };
    let protocol =
        std::env::var("PROTOCOL").expect("PROTOCOL environemt variable error, make sure it is set");
    let base_url =
        std::env::var("BASE_URL").expect("BASE_URL environemt variable error, make sure it is set");

    let conf = CONFIG {
        root: format!("{}://{}", protocol, base_url),
    };

    let db_pool = SqlitePool::connect(&database_url)
        .await
        .expect("could not create db pool");

    println!("Listening on: 127.0.0.1:8080, open browser and visit have a try!");
    HttpServer::new(move || {
        let tera = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).unwrap();

        App::new()
            .data(db_pool.clone()) // pass database pool to application so we can access it inside handlers
            .data(tera)
            .data(conf.clone())
            .route("/{id}/events.ics", web::get().to(make_ics_request))
            .service(web::resource("/").route(web::get().to(index)))
            .service(web::resource("/edit").route(web::get().to(edit_page)))
            .service(web::resource("/index_process").route(web::get().to(index_process)))
            .service(web::resource("/edit_process").route(web::get().to(edit_process)))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await

    //.route("/{id}/events.ics", web::get().to(make_ics_request))
}
