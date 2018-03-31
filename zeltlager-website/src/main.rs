extern crate actix;
extern crate actix_web;
extern crate bytes;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate lettre;
extern crate lettre_email;
extern crate mime;
extern crate pulldown_cmark;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate t4rust_derive;
extern crate toml;

use std::fs::File;
use std::io::Read;

use actix_web::*;
use futures::Future;

mod basic;
mod db;
mod email;
mod signup;

type Result<T> = std::result::Result<T, failure::Error>;
type BoxFuture<T> = Box<futures::Future<Item = T, Error = failure::Error>>;

/*macro_rules! tryf {
    ($e:expr) => {
        match $e {
            Ok(e) => e,
            Err(error) => return Box::new(future::err(error.into())),
        }
    };
}*/

#[derive(Deserialize, Debug, Clone)]
struct Config {
    email_username: String,
    email_password: String,
    /// Address to bind to.
    ///
    /// # Default
    ///
    /// 127.0.0.0:8080
    bind_address: Option<String>,
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

fn startpage(req: HttpRequest<AppState>) -> Result<HttpResponse> {
    let name: String = "startseite".to_string();
    let site = req.state().basics.get_site(&name)?;
    let content = format!("{}", site);

    Ok(httpcodes::HttpNotFound.build()
       .content_type("text/html; charset=utf-8")
       .body(content)?)
}

fn signup_send(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
    // Get the body of the request
    req.urlencoded()
       .limit(1024 * 5) // 5 kiB
       .from_err()
       .and_then(|body| {
           let form = signup::Form::from_hashmap(body)?;
           println!("Get form {:?}", form);
           Ok(HttpResponse::Ok().into())
       }).responder()
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
    let config: Config = toml::from_str(&content).expect("Failed to parse config.toml");

    let address = config.bind_address.as_ref().map(|s| s.as_str())
        .unwrap_or("127.0.0.1:8080").to_string();
    let state = AppState { basics, config };

    HttpServer::new(move || {
        Application::with_state(state.clone())
            .middleware(middleware::Logger::default())
            .handler("/static", fs::StaticFiles::new("static", false)
                .default_handler(not_found))
            .resource("/mail", |r| r.f(send_confirmation_mail))
            .resource("/signup-send", |r| r.method(Method::POST).f(signup_send))
            .resource("/{name}", |r| r.f(index))
            .resource("", |r| r.f(startpage))
            .default_resource(|r| r.f(not_found))
    }).bind(address).unwrap()
        .run();
}
