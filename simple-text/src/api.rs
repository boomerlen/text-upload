use actix_web::{web, HttpResponse, Responder};

use crate::git_management::{Mono, modify_buffer, commit_buffer};

fn split_post_body(req_body: String) -> (String, String) {
    (String::from("buffname"), String::from("bufftext"))
}

async fn upload_text(req_body: String, data: web::Data<Mono>) -> impl Responder {
    let response_text: String = format!("Message received! Post text: {}", req_body);

    // Use magic function to extract data from body
    let (buf_name, buf_text) = split_post_body(req_body);

    modify_buffer(&buf_name, &buf_text);
    commit_buffer(&buf_name, &data);

    // Note: no semicolon after this line because it is an expression
    // and we want to return its result
    HttpResponse::Ok().body(response_text)
}

async fn basic_get() -> impl Responder {
    HttpResponse::Ok().body("GET Received!")
}

pub fn config_simple_text(cfg: &mut web::ServiceConfig) {
    // Contrast to line 8 - this function does not return
    // anything so we make the following line into a statement with the semicolon
    cfg.service(
        web::scope("/api")
            .route("/simple-text", web::post().to(upload_text))
            .route("/simple-text", web::get().to(basic_get))
    );
}
