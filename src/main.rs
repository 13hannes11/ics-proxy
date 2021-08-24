use actix_web::{get, App, HttpResponse, HttpServer, Responder};

#[get("/events.ics")]
async fn hello() -> impl Responder {
    let res = match reqwest::blocking::get("https://cloud.timeedit.net/uu/web/schema/ri6QX6089X8061QQ88Z4758Z08y37424838828461554904Y684XX09894Q8721784ZnX6503.ics") {
        Ok(r) => match r.text() {
            Ok(res) => res,
            Err(_) => "".to_string(),
        },
        Err(_) => "".to_string(),
    };

    HttpResponse::Ok().content_type("text/calendar").body(res)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(hello))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
