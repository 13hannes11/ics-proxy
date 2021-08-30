use std::collections::HashMap;

use actix_http::{body::Body, Response};
use actix_web::dev::ServiceResponse;
use actix_web::http::StatusCode;
use actix_web::middleware::errhandlers::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::{error, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder, Result};
use tera::Tera;

async fn make_ics_request(req: HttpRequest) -> impl Responder {
    let id = req.match_info().get("id").unwrap_or("World");

    let body = match id {
        "1" => {
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

// store tera template in application state
async fn index(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, Error> {
    if query.get("create").is_some() {
        // create new link
    } else if query.get("edit").is_some() {
        // edit existing link
    } else if query.get("replace").is_some() {
        // replace link
    }
    let s = if let Some(link) = query.get("link") {
        // submitted form
        let proxy_link = &"Insert link here".to_owned();

        let mut ctx = tera::Context::new();
        ctx.insert("link", &link.to_owned());
        ctx.insert("proxy_link", proxy_link);
        tmpl.render("edit.html", &ctx)
            .map_err(|_| error::ErrorInternalServerError("Template error"))?
    } else {
        tmpl.render("index.html", &tera::Context::new())
            .map_err(|_| error::ErrorInternalServerError("Template error"))?
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");

    println!("Listening on: 127.0.0.1:8080, open browser and visit have a try!");
    HttpServer::new(|| {
        let tera = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).unwrap();

        App::new()
            .data(tera)
            .route("/{id}/events.ics", web::get().to(make_ics_request))
            .service(web::resource("/").route(web::get().to(index)))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await

    //.route("/{id}/events.ics", web::get().to(make_ics_request))
}
