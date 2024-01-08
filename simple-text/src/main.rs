mod api;
mod git_management;

use actix_web::{App, HttpServer};

pub use crate::api::config_simple_text;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().configure(config_simple_text))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
