use actix_web::{web, HttpResponse, Responder};
use git2::Repository;
use serde::Deserialize;

use crate::git_management::{add_buffer, commit_buffer, modify_buffer, open_repo, push_to_repo};

#[derive(Deserialize)]
struct PostBody {
    buffer: String,
    text: String,
}

async fn upload_text(req_body: web::Json<PostBody>) -> impl Responder {
    // TODO: error handling along the way
    // Should not reutn Ok if it's not Ok.
    let response_text: String = format!("Message received! Post text: {}", req_body.text);

    // Use magic function to extract data from body
    let buf_name = &req_body.buffer;
    let buf_text = &req_body.text;

    let repo: Repository = open_repo();

    modify_buffer(buf_name, buf_text);
    add_buffer(buf_name, &repo);
    commit_buffer(&repo);
    push_to_repo(&repo);

    HttpResponse::Ok().body(response_text)
}

async fn basic_get() -> impl Responder {
    HttpResponse::Ok().body("GET Received!")
}

pub fn config_simple_text(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/simple-text", web::post().to(upload_text))
            .route("/simple-text", web::get().to(basic_get)),
    );
}
