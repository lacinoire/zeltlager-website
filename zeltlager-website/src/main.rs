extern crate actix_web;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate t4rust_derive;

use std::fs::File;
use std::io::Read;

use actix_web::*;

mod basic;

fn index(_req: HttpRequest) -> &'static str {
    ""
}

fn not_found(_: HttpRequest) -> Result<HttpResponse> {
    let mut content = String::new();
    File::open("static/404.html")?.read_to_string(&mut content)?;
    Ok(httpcodes::HttpNotFound.build()
       .content_type("text/html; charset=utf-8")
       .body(content)?)
}

fn main() {
    HttpServer::new(
        || Application::new()
            .middleware(middleware::Logger::default())
            .handler("/static", fs::StaticFiles::new("static", false)
                .default_handler(not_found))
            .resource("/{name}", |r| r.f(index))
            .default_resource(|r| r.f(not_found)))
        .bind("127.0.0.1:8080").unwrap()
        .run();
}
