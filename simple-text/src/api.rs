use actix_web::{web, HttpResponse, Responder};
use git2::Repository;
use serde::Deserialize;

use crate::git_management::{add_buffer, commit_buffer, modify_buffer, open_repo, push_to_repo, get_now};


#[derive(Deserialize)]
struct PostBody {
    buffer: String,
    text: String,
}

fn resolve_buffer(buf_text: &String) -> String {
    // Specifies allowed buffers
    match buf_text.as_str() {
        "Places" => String::from("places.md"),
        "TTM" => String::from("ttm.md"),
        "Misc" => String::from("misc.md"),
        "Food" => String::from("food.md"),
        _ => {
            format!("nobuffer/{}.md", get_now())
        },
    }
}

async fn upload_text(req_body: web::Json<PostBody>) -> impl Responder {
    // Should not reutn Ok if it's not Ok.
    // Use magic function to extract data from body
    let buf_name = resolve_buffer(&req_body.buffer);
    let buf_text = &req_body.text;

    let repo: Repository = match open_repo() {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Oopen repo failed with git error code {:?}.", e.code())),
    };

    match modify_buffer(&buf_name, buf_text) {
        Ok(_) => (),
        Err(e) => return HttpResponse::InternalServerError().body(format!("Modify buffer failed with git error code {:?}.", e.code())),
    };

    match add_buffer(&buf_name, &repo) {
        Ok(_) => (),
        Err(e) => return HttpResponse::InternalServerError().body(format!("Add buffer failed with git error code {:?}.", e.code())),
    };

    match commit_buffer(&repo) {
        Ok(_) => (),
        Err(e) => return HttpResponse::InternalServerError().body(format!("Commit buffer failed with git error code {:?}", e.code())),
    };

    match push_to_repo(&repo) {
        Ok(_) => HttpResponse::Ok().body("Success!"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Push failed with git error code {:?}.", e.code())),
    }
}

async fn basic_get() -> impl Responder {
    HttpResponse::Ok().body("GET Received! Hello world!")
}

pub fn config_simple_text(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/simple-text", web::post().to(upload_text))
            .route("/simple-text", web::get().to(basic_get)),
    );
}
