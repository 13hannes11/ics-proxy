use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{error, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder, Result};
use sqlx::{Pool, Sqlite, SqlitePool};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tera::Tera;
use tokio::time;
use url::Url;
use uuid::Uuid;
extern crate dotenv;
use actix_web::middleware::Logger;
use dotenv::dotenv;
mod model;
use chrono::DateTime;

use model::Link;

use chrono::Utc;
const REDIRECT_TIMEOUT_S: i32 = 2;

#[derive(Clone)]
struct Config {
    root: String,
}

async fn make_ics_request(req: HttpRequest, db_pool: web::Data<Pool<Sqlite>>) -> impl Responder {
    let id = req.match_info().get("id").unwrap_or("");
    let now = <SystemTime as Into<DateTime<Utc>>>::into(SystemTime::now()).to_rfc3339();
    println!("{now} serving ics request");
    match Uuid::parse_str(id) {
        Ok(uuid) => match Link::find_by_uuid(uuid.to_string(), db_pool).await {
            Ok(link) => match reqwest::get(link.destination).await {
                Ok(r) => match r.text().await {
                    Ok(res) => HttpResponse::Ok().content_type("text/calendar").body(res),
                    Err(err) => {
                        HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).body(err.to_string())
                    }
                },
                Err(err) => {
                    HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).body(err.to_string())
                }
            },
            Err(_) => HttpResponse::build(StatusCode::NOT_FOUND).finish(),
        },
        Err(_) => HttpResponse::build(StatusCode::BAD_REQUEST).finish(),
    }
}

fn error_page(
    tmpl: web::Data<tera::Tera>,
    msg: String,
    status_code: StatusCode,
) -> Result<HttpResponse, Error> {
    let mut ctx = tera::Context::new();
    ctx.insert("message", &msg);
    let s = tmpl
        .render("error.html", &ctx)
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;

    Ok(HttpResponse::Ok()
        .status(status_code)
        .content_type("text/html")
        .body(s))
}

// This is the new edit page:
async fn edit_page(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
    db_pool: web::Data<Pool<Sqlite>>,
    conf: web::Data<Config>,
) -> Result<HttpResponse, Error> {
    let now = <SystemTime as Into<DateTime<Utc>>>::into(SystemTime::now()).to_rfc3339();
    println!("{now} serving edit page");
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
                Err(err) => error_page(
                    tmpl,
                    format!("db error: {}", err.to_string()),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ),
            },
            Err(err) => error_page(
                tmpl,
                format!("uuid parsing error: {}", err.to_string()),
                StatusCode::BAD_REQUEST,
            ),
        }
    } else {
        error_page(
            tmpl,
            "uuid parameter missing".to_string(),
            StatusCode::BAD_REQUEST,
        )
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

fn redirect_to_index_page(
    tmpl: web::Data<tera::Tera>,
    message: String,
    time_s: i32,
) -> Result<HttpResponse, Error> {
    let link = "/".to_string();
    redirect_to_page(tmpl, message, link, time_s)
}

async fn delete_process(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
    db_pool: web::Data<Pool<Sqlite>>,
) -> Result<HttpResponse, Error> {
    if let Some(uuid_str) = query.get("uuid") {
        match Uuid::parse_str(uuid_str) {
            Ok(uuid) => match Link::delete(uuid.to_string(), db_pool).await {
                Ok(_) => redirect_to_index_page(
                    tmpl,
                    "Delete was successful".to_string(),
                    REDIRECT_TIMEOUT_S,
                ),
                Err(err) => error_page(
                    tmpl,
                    format!("db error: {}", err.to_string()),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ),
            },
            Err(err) => error_page(
                tmpl,
                format!("uuid parsing error: {}", err.to_string()),
                StatusCode::BAD_REQUEST,
            ),
        }
    } else {
        error_page(
            tmpl,
            "uuid parameter missing".to_string(),
            StatusCode::BAD_REQUEST,
        )
    }
}

async fn edit_process(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
    db_pool: web::Data<Pool<Sqlite>>,
    config: web::Data<Config>,
) -> Result<HttpResponse, Error> {
    // TODO: implement handling
    if let Some(uuid_str) = query.get("uuid") {
        if let Some(destination) = query.get("link") {
            if destination.starts_with(&config.root) {
                return error_page(
                    tmpl,
                    "url cannot contain url of ics-proxy".to_string(),
                    StatusCode::BAD_REQUEST,
                );
            };

            if Url::parse(destination).is_err() {
                return error_page(
                    tmpl,
                    "could not parse url".to_string(),
                    StatusCode::BAD_REQUEST,
                );
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
                        Err(err) => error_page(
                            tmpl,
                            format!("db error: {}", err.to_string()),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        ),
                    }
                }
                Err(err) => error_page(
                    tmpl,
                    format!("uuid parsing error: {}", err.to_string()),
                    StatusCode::BAD_REQUEST,
                ),
            }
        } else {
            error_page(
                tmpl,
                "link parameter missing".to_string(),
                StatusCode::BAD_REQUEST,
            )
        }
    } else {
        error_page(
            tmpl,
            "uuid parameter missing".to_string(),
            StatusCode::BAD_REQUEST,
        )
    }
}

async fn index_process(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
    db_pool: web::Data<Pool<Sqlite>>,
    config: web::Data<Config>,
) -> Result<HttpResponse, Error> {
    if query.get("create").is_some() {
        let uuid = Uuid::new_v4();
        // TODO: add actuall logic and use proper uuid
        match query.get("link") {
            // TODO: actually parse link to url to make sure its valid
            Some(destination) => {
                if destination.starts_with(&config.root) {
                    return error_page(
                        tmpl,
                        "url cannot contain url of ics-proxy".to_string(),
                        StatusCode::BAD_REQUEST,
                    );
                };

                if Url::parse(destination).is_err() {
                    return error_page(
                        tmpl,
                        "could not parse url".to_string(),
                        StatusCode::BAD_REQUEST,
                    );
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
                        Err(e) => error_page(
                            tmpl,
                            format!("uuid parsing error {}", e.to_string()),
                            StatusCode::BAD_REQUEST,
                        ),
                    },
                    // TODO: actually redirect to index page to try again
                    Err(e) => error_page(
                        tmpl,
                        format!("db error: {}", e.to_string()),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    ),
                }
            }
            None => {
                // TODO: actually redirect back to index page
                error_page(
                    tmpl,
                    "link attribute not set please enter a link".to_string(),
                    StatusCode::BAD_REQUEST,
                )
            }
        }
    } else if query.get("edit").is_some() {
        match query.get("link") {
            Some(link) => {
                // Splitting string and getting uuid, alternatively pretend whole string is uuid
                let vec: Vec<&str> = link.split('/').collect();

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
                    Err(e) => error_page(
                        tmpl,
                        format!("could not parse uuid: {}", e.to_string()),
                        StatusCode::BAD_REQUEST,
                    ),
                }
            }
            None => {
                // TODO: actually redirect back to index page
                error_page(
                    tmpl,
                    "link attribute not set please enter a link".to_string(),
                    StatusCode::BAD_REQUEST,
                )
            }
        }
    } else {
        error_page(
            tmpl,
            "missing create or edit form submission!".to_string(),
            StatusCode::BAD_REQUEST,
        )
    }
}

// store tera template in application state
async fn index(tmpl: web::Data<tera::Tera>) -> Result<HttpResponse, Error> {
    // TODO: add option to prefill link with parameter
    let now = <SystemTime as Into<DateTime<Utc>>>::into(SystemTime::now()).to_rfc3339();
    println!("{now} serving index page");
    let s = tmpl
        .render("index.html", &tera::Context::new())
        .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

fn attach_templates(cfg: &mut web::ServiceConfig) {
    let tera = Tera::new("templates/**/*.html").unwrap();
    cfg.app_data(Data::new(tera));
}

fn attach_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/events.ics", web::get().to(make_ics_request))
        .service(web::resource("/").route(web::get().to(index)))
        .service(web::resource("/edit").route(web::get().to(edit_page)))
        .service(web::resource("/index_process").route(web::get().to(index_process)))
        .service(web::resource("/edit_process").route(web::get().to(edit_process)))
        .service(web::resource("/delete_process").route(web::get().to(delete_process)));
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

    let host = match std::env::var("HOST") {
        Ok(host) => host,
        Err(_e) => "0.0.0.0:8080".to_string(),
    };

    let conf = Config {
        root: format!("{}://{}", protocol, base_url),
    };

    let db_pool = SqlitePool::connect(&database_url)
        .await
        .expect("could not create db pool");

    sqlx::migrate!("./migrations").run(&db_pool).await.unwrap();

    // Spawn a background task for periodic cleanup
    let cleanup_pool = db_pool.clone();
    tokio::spawn(async move {
        run_periodic_cleanup(cleanup_pool).await;
    });

    println!(
        "Listening on: {}://{}, open browser and visit have a try!",
        protocol, base_url
    );
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::new("%a %{User-Agent}i"))
            .app_data(Data::new(db_pool.clone())) // pass database pool to application so we can access it inside handlers
            .app_data(Data::new(conf.clone()))
            .configure(attach_templates)
            .configure(attach_routes)
    })
    .bind(host)?
    .run()
    .await
}

async fn run_periodic_cleanup(pool: Pool<Sqlite>) {
    let mut interval = time::interval(Duration::from_secs(3600));

    loop {
        interval.tick().await;

        match model::delete_old_entries(&pool).await {
            Ok(rows) => {
                println!("Cleanup job: successfully deleted {} old entries", rows);
            }
            Err(err) => {
                eprintln!("Error in cleanup job: {}", err);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web};
    use sqlx::SqlitePool;

    async fn setup_test_db() -> Pool<Sqlite> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        pool
    }

    #[actix_web::test]
    async fn test_make_ics_request_destination_does_not_exist() {
        let pool = setup_test_db().await;

        let test_uuid = Uuid::new_v4().to_string();
        let test_link = Link {
            uuid: test_uuid.clone(),
            destination: "http://calendar.example/calendar.ics".to_string(),
        };

        Link::create(test_link, web::Data::new(pool.clone()))
            .await
            .expect("Failed to create test link");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .configure(attach_routes),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/{}/events.ics", test_uuid))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_server_error());
    }

    #[actix_web::test]
    async fn test_edit_page() {
        let pool = setup_test_db().await;
        let test_uuid = Uuid::new_v4();
        let destination = "http://calendar.example/".to_string();
        let test_link = Link {
            uuid: test_uuid.to_string(),
            destination: destination.clone(),
        };

        Link::create(test_link, web::Data::new(pool.clone()))
            .await
            .expect("Failed to create test link");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .configure(attach_templates)
                .configure(attach_routes)
                .app_data(web::Data::new(Config {
                    root: "http://localhost:8080".to_string(),
                })),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/edit?uuid={}", test_uuid))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let body_content = String::from_utf8(body.to_vec()).unwrap();
        let encoded_destination_url = html_escape::encode_safe(destination.as_str());
        assert!(body_content.contains(encoded_destination_url.to_string().as_str()));
    }

    #[actix_web::test]
    async fn test_index_process_create() {
        let pool = setup_test_db().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .configure(attach_routes)
                .configure(attach_templates)
                .app_data(web::Data::new(Config {
                    root: "http://localhost:8080".to_string(),
                })),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/index_process?create=1&link=https://example.com/calendar.ics")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_edit_process() {
        let pool = setup_test_db().await;
        let test_uuid = Uuid::new_v4();
        let test_link = Link {
            uuid: test_uuid.to_string(),
            destination: "https://example.com/calendar.ics".to_string(),
        };

        Link::create(test_link, web::Data::new(pool.clone()))
            .await
            .expect("Failed to create test link");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .configure(attach_routes)
                .configure(attach_templates)
                .app_data(web::Data::new(Config {
                    root: "http://localhost:8080".to_string(),
                })),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!(
                "/edit_process?uuid={}&link=https://example.com/new.ics",
                test_uuid
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_edit_missing_uuid() {
        let pool = setup_test_db().await;
        let test_uuid = Uuid::new_v4();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .configure(attach_routes)
                .configure(attach_templates)
                .app_data(web::Data::new(Config {
                    root: "http://localhost:8080".to_string(),
                })),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/edit?uuid={}", test_uuid))
            .to_request();

        let resp = test::call_service(&app, req).await;
        // TODO: it would be better to return a 404 but for now this is fine
        assert!(resp.status().is_server_error());
    }

    #[actix_web::test]
    async fn test_delete_link() {
        let pool = setup_test_db().await;
        let test_uuid = Uuid::new_v4();
        let test_link = Link {
            uuid: test_uuid.to_string(),
            destination: "https://example.com/calendar.ics".to_string(),
        };

        Link::create(test_link, web::Data::new(pool.clone()))
            .await
            .expect("Failed to create test link");

        let result = Link::delete(test_uuid.to_string(), web::Data::new(pool.clone()))
            .await
            .expect("Failed to delete link");

        assert_eq!(result, 1);

        let link = Link::find_by_uuid(test_uuid.to_string(), web::Data::new(pool.clone())).await;
        assert!(link.is_err());
    }

    #[actix_web::test]
    async fn test_delete_process() {
        let pool = setup_test_db().await;
        let test_uuid = Uuid::new_v4();
        let test_link = Link {
            uuid: test_uuid.to_string(),
            destination: "https://example.com/calendar.ics".to_string(),
        };

        Link::create(test_link, web::Data::new(pool.clone()))
            .await
            .expect("Failed to create test link");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .configure(attach_routes)
                .configure(attach_templates)
                .app_data(web::Data::new(Config {
                    root: "http://localhost:8080".to_string(),
                })),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/delete_process?uuid={}", test_uuid))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let link = Link::find_by_uuid(test_uuid.to_string(), web::Data::new(pool.clone())).await;
        assert!(link.is_err());
    }

    #[actix_web::test]
    async fn test_proxy_request() {
        let pool = setup_test_db().await;

        // Create a mock server
        let mut mock_server = mockito::Server::new_async().await;
        let calendar_data = "BEGIN:VCALENDAR\nEND:VCALENDAR";
        let mock = mock_server
            .mock("GET", "/calendar.ics")
            .with_status(200)
            .with_header("content-type", "text/calendar")
            .with_body(calendar_data)
            .create();

        let test_uuid = Uuid::new_v4().to_string();
        let test_link = Link {
            uuid: test_uuid.clone(),
            destination: format!("{}/calendar.ics", mock_server.url()),
        };

        Link::create(test_link, web::Data::new(pool.clone()))
            .await
            .expect("Failed to create test link");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .route("/{id}/events.ics", web::get().to(make_ics_request)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/{}/events.ics", test_uuid))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body = test::read_body(resp).await;
        assert_eq!(body, calendar_data);
        mock.assert();
    }
}
