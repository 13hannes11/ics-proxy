use std::collections::HashMap;

use actix_web::{error, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder, Result};
use sqlx::{Pool, Sqlite, SqlitePool};
use tera::Tera;
use uuid::Uuid;

mod model;
use model::Link;

const REDIRECT_TIMEOUT_S: i32 = 2;

async fn make_ics_request(req: HttpRequest) -> impl Responder {
    let id = req.match_info().get("id").unwrap_or("");

    let body = match id {
        "1" => {
            // TODO: load url based on id from database and make request
            let res = match reqwest::blocking::get("https://cloud.timeedit.net/uu/web/schema/ri6QX6089X8061QQ88Z4758Z08y37424838828461554904Y684XX09894Q8721784ZnX6503.ics") {
                Ok(r) => match r.text() {
                    Ok(res) => res,
                    Err(_) => "".to_string(),
                },
                Err(_) => "".to_string(),
            };

            res
        }
        _ => "".to_string(),
    };
    HttpResponse::Ok().content_type("text/calendar").body(body)
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
) -> Result<HttpResponse, Error> {
    // one uuid: 9228c1a4-8956-4f1c-8b5f-53cc575bd78
    if let Some(uuid_str) = query.get("uuid") {
        // TODO: based on uuid get link from database
        let link = "this is the link from the db".to_string();
        match Uuid::parse_str(uuid_str) {
            Ok(uuid) => {
                let mut ctx = tera::Context::new();
                ctx.insert("link", &link);
                ctx.insert("uuid", &uuid.to_string());
                let s = tmpl
                    .render("edit.html", &ctx)
                    .map_err(|_| error::ErrorInternalServerError("Template error"))?;

                Ok(HttpResponse::Ok().content_type("text/html").body(s))
            }
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
) -> Result<HttpResponse, Error> {
    // TODO: implement handling
    if let Some(uuid_str) = query.get("uuid") {
        if let Some(link) = query.get("link") {
            match Uuid::parse_str(uuid_str) {
                Ok(uuid) => {
                    // TODO: actually save entry in database
                    redirect_to_edit_page(
                        tmpl,
                        "Edit successful!".to_string(),
                        uuid,
                        REDIRECT_TIMEOUT_S,
                    )
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
) -> Result<HttpResponse, Error> {
    if query.get("create").is_some() {
        let uuid = Uuid::new_v4();
        // TODO: add actuall logic and use proper uuid
        match query.get("link") {
            // TODO: actually parse link to url to make sure its valid
            Some(destination) => {
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
        let uuid = Uuid::nil();
        // TODO: add actuall logic and use proper uuid
        redirect_to_edit_page(
            tmpl,
            "Entry found in database!".to_string(),
            uuid,
            REDIRECT_TIMEOUT_S,
        )
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

    let database_url = "sqlite://db/db.db"; //env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let db_pool = SqlitePool::connect(&database_url)
        .await
        .expect("could not create db pool");

    println!("Listening on: 127.0.0.1:8080, open browser and visit have a try!");
    HttpServer::new(move || {
        let tera = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).unwrap();

        App::new()
            .data(db_pool.clone()) // pass database pool to application so we can access it inside handlers
            .data(tera)
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
