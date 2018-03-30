extern crate actix_web;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use actix_web::*;

fn index(req: HttpRequest) -> &'static str {
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
