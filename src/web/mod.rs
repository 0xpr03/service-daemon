pub mod api;

pub mod models;
pub mod websocket;

use actix_files as fs;
use actix_identity::*;
use actix_web::cookie::SameSite;
use actix_web::dev::Server;
use actix_web::middleware::Logger;

use actix_web::{web, App, HttpServer};

pub fn start(domain: String, max_age_secs: i64) -> std::io::Result<Server> {
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 32])
                    .name("my-auth-cookie")
                    .same_site(SameSite::Strict)
                    // #[cfg(not(debug_assertions))]
                    // .domain(domain.as_str())
                    .max_age(max_age_secs) // 1 day
                    // .http_only(true) already set by CookieIdentityPolicy
                    .secure(false),
            ))
            .service(
                web::scope("/api")
                    .data(web::JsonConfig::default().limit(4096))
                    .service(web::resource("/logout").route(web::post().to_async(api::logout)))
                    .service(web::resource("/login").route(web::post().to_async(api::login)))
                    .service(web::resource("/checklogin").route(web::get().to_async(api::checklogin)))
                    .service(web::resource("/totp").route(web::post().to_async(api::totp)))
                    .service(
                        web::resource("/service/{service}/state")
                            .route(web::get().to_async(api::state)),
                    )
                    .service(
                        web::resource("/service/{service}/output")
                            .route(web::get().to_async(api::output)),
                    )
                    .service(
                        web::resource("/service/{service}/input")
                            .route(web::post().to_async(api::input)),
                    )
                    .service(
                        web::resource("/service/{service}/stop")
                            .route(web::post().to_async(api::stop)),
                    )
                    .service(
                        web::resource("/service/{service}/start")
                            .route(web::post().to_async(api::start)),
                    )
                    .service(web::resource("/services").route(web::get().to_async(api::services))),
            )
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
    // let ServiceController handle signals
    // .disable_signals()
    .bind("127.0.0.1:9000")?
    .start())
}
