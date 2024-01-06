mod api;
mod git_management;

use actix_web::{web, App, HttpServer};

pub use crate::api::config_simple_text;
pub use crate::git_management::{initialise_repo, commit_buffer, Mono};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
        .app_data(web::Data::new(initialise_repo()))
        .configure(config_simple_text)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}