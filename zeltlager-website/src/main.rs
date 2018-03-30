extern crate actix_web;
#[macro_use]
extern crate failure;
extern crate lettre;
extern crate lettre_email;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate t4rust_derive;
extern crate toml;

use std::fs::File;
use std::io::Read;

use actix_web::*;

mod basic;
mod email;

type Result<T> = std::result::Result<T, failure::Error>;

#[derive(Deserialize, Debug, Clone)]
struct Config {
    email_username: String,
    email_password: String,
}

#[derive(Clone)]
pub struct AppState {
    basics: basic::SiteDescriptions,
    config: Config,
}

fn index(req: HttpRequest<AppState>) -> Result<HttpResponse> {
    let name: String = req.match_info().query("name")?;
    let site = req.state().basics.get_site(&name)?;
    let content = format!("{}", site);

    Ok(httpcodes::HttpNotFound.build()
       .content_type("text/html; charset=utf-8")
       .body(content)?)
}

fn not_found(_: HttpRequest<AppState>) -> Result<HttpResponse> {
    let mut content = String::new();
    File::open("static/404.html")?.read_to_string(&mut content)?;
    Ok(httpcodes::HttpNotFound.build()
       .content_type("text/html; charset=utf-8")
       .body(content)?)
}

fn send_confirmation_mail(req: HttpRequest<AppState>) -> Result<HttpResponse> {
    let maildata = email::MailData {
        parent_mail: "c.eltern@flakebi.de".to_string(),
        parent_name: "Sebastian Neubauer".to_string(),
        child_first_name: "Antonia".to_string(),
        child_last_name: "Neubauer".to_string() };
    let result = email::send_mail(maildata, req.state());
    println!("{:?}", result);
    let mut content = String::new();
    File::open("static/Home.html")?.read_to_string(&mut content)?;
    Ok(httpcodes::HttpAccepted.build()
        .content_type("text/html; charset=utf-8")
        .body(content)?)
}

fn main() {
    let basics = basic::SiteDescriptions::parse().expect("Failed to parse basic.toml");
    let mut content = String::new();
    File::open("config.toml").unwrap().read_to_string(&mut content).unwrap();
    let config = toml::from_str(&content).expect("Failed to parse config.toml");

    let state = AppState { basics, config };
    HttpServer::new(move || {
        Application::with_state(state.clone())
            .middleware(middleware::Logger::default())
            .handler("/static", fs::StaticFiles::new("static", false)
                .default_handler(not_found))
            .resource("/mail", |r| r.f(send_confirmation_mail))
            .resource("/{name}", |r| r.f(index))
            .default_resource(|r| r.f(not_found))
    }).bind("127.0.0.1:8080").unwrap()
        .run();
}
