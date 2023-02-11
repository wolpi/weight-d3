mod parse;

#[macro_use]
extern crate actix_web;

extern crate serde_json;

extern crate url;

use actix_http::{body::Body, Response};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Result};
use actix_web::dev::ServiceResponse;
use actix_web::http::StatusCode;
use actix_web::middleware;
use actix_web::middleware::errhandlers::{ErrorHandlerResponse, ErrorHandlers};
use actix_web_static_files;
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::fs::File;
use std::path::Path;
use std::io;
use std::env;
use std::ops::Deref;

use webbrowser;

lazy_static! {
    static ref DATA: Mutex<Vec<parse::Entry>> = Mutex::new(vec![]);
}

#[get("/data.json")]
async fn data_json(req: HttpRequest) -> HttpResponse {
    println!("data.json request from {}", req.connection_info().remote_addr().unwrap());
    HttpResponse::Ok().json(&DATA.lock().unwrap().deref())
}

#[get("/data-apex.json")]
async fn data_apex_json(req: HttpRequest) -> HttpResponse {
    println!("data-apex.json request from {}", req.connection_info().remote_addr().unwrap());
    let data = DATA.lock().unwrap();
    let data_apex :Vec<(&u64, &f32)> = data.deref().into_iter().map(
        |entry| (&entry.raw_timestamp, &entry.weight)
    ).collect();
    HttpResponse::Ok().json(data_apex)
}

fn error_handlers() -> ErrorHandlers<Body> {
    ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found)
}

fn not_found<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    let response = get_error_response(&res, "Page not found");
    Ok(ErrorHandlerResponse::Response(
        res.into_response(response.into_body()),
    ))
}

fn get_error_response<B>(res: &ServiceResponse<B>, error: &'static str) -> Response<Body> {
    Response::build(res.status())
        .content_type("text/plain")
        .body(error)
}

fn init_data(file: &File) {
    let mut data: Vec<parse::Entry> = parse::parse_file(&file);
    data.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    let mut guard = DATA.lock().unwrap();
    for entry in data {
        guard.push(entry);
    }
}

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_web::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return io::Result::Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "which data file to serve ???"));
    }
    let data_path = Path::new(&args[1]);
    let file_result = File::open(&data_path);
    if file_result.is_err() {
        println!("could not open file: '{}'!!!", data_path.to_str().unwrap());
        let err = file_result.unwrap_err();
        println!("{}", err);
        return io::Result::Err(err);
    }
    init_data(&file_result.unwrap());

    let port = "8080";
    let mut browser_url = "http://127.0.0.1:".to_string();
    browser_url.push_str(port);
    println!("opening browser at {} ...", browser_url);
    let browser_result = webbrowser::open(browser_url.as_str());
    if browser_result.is_err() {
        println!("... failed");
        println!("{}", browser_result.err().unwrap());
    }

    let mut bind = "0.0.0.0:".to_string();
    bind.push_str(port);
    HttpServer::new(move || {
        let generated = generate();
        App::new()
            .wrap(error_handlers())
            .wrap(middleware::DefaultHeaders::new().header("Cache-Control", "max-age=0"))
            // register data_json before static files on /
            .service(data_json)
            .service(data_apex_json)
            .service(actix_web_static_files::ResourceFiles::new("/", generated,))
    })
    .bind(bind.as_str())?
    .run()
    .await
}

