use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/{id}/events.ics", web::get().to(make_ics_request)))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
