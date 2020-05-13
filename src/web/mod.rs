pub mod api;

pub mod models;
pub mod websocket;

use crate::settings::Web;
use actix_files as fs;
use actix_identity::*;
use actix_web::cookie::SameSite;
use actix_web::dev::Server;
use actix_web::middleware::Logger;
use actix_web::{guard, web, App, HttpResponse, HttpServer};

pub fn start(config: &Web, max_age_secs: u32) -> std::io::Result<Server> {
    //TODO: add CORS
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 32])
                    .name("sc-auth")
                    .same_site(SameSite::Strict)
                    // #[cfg(not(debug_assertions))]
                    // .domain(domain.as_str())
                    .max_age(i64::from(max_age_secs)) // 1 day
                    // .http_only(true) already set by CookieIdentityPolicy
                    .secure(false),
            ))
            .service(web::scope("/api")
                .data(web::JsonConfig::default().limit(4096))
                .service(web::resource("/logout").route(web::post().to(api::logout)))
                .service(web::resource("/login").route(web::post().to(api::login)))
                .service(web::resource("/checklogin").route(web::get().to(api::checklogin)))
                .service(web::resource("/totp").route(web::post().to(api::totp)))
                // current user permissions management
                .service(web::resource("/permissions").route(web::get().to(api::session_permissions)))
                .service(web::scope("/user")
                    .service(web::resource("/list").route(web::get().to(api::user_list)))
                    .service(web::resource("/create").route(web::post().to(api::create_user)))
                    .service(web::scope("/{user}")
                        .service(web::resource("/password").route(web::post().to(api::change_password)))
                        .service(web::resource("/totp").route(web::post().to(api::change_totp)))
                        .service(web::resource("/info")
                            .route(web::get().to(api::get_user_info))
                            .route(web::post().to(api::set_user_info)))
                        .service(web::resource("/delete").route(web::post().to(api::delete_user)))
                        .service(web::resource("/services").route(web::get().to(api::all_user_services)))
                        .service(web::resource("/permissions/{service}")
                            .route(web::get().to(api::get_service_permission))
                            .route(web::post().to(api::set_service_permission)))
                    )
                )
                .service(web::scope("/service/{service}")
                    .service(web::resource("/state").route(web::get().to(api::state)))
                    .service(web::resource("/output").route(web::get().to(api::output)))
                    .service(web::resource("/input").route(web::post().to(api::input)))
                    .service(web::resource("/stop").route(web::post().to(api::stop)))
                    .service(web::resource("/start").route(web::post().to(api::start)))
                    .service(web::resource("/kill").route(web::post().to(api::kill)))
                    .service(web::resource("/log/latest/{amount}").route(web::get().to(api::log_latest)))
                    .service(web::resource("/log/console/{log_id}").route(web::get().to(api::log_console)))
                    .service(web::resource("/log/details/{log_id}").route(web::get().to(api::log_details)))
                    // Permissions of current user for service
                    .service(web::resource("/permissions").route(web::get().to(api::session_service_perm)))
                )
                .service(web::resource("/services").route(web::get().to(api::services)))
                .default_service(web::resource("")
                    .route(web::get().to(||HttpResponse::NotFound()))
                    .route(web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(||HttpResponse::MethodNotAllowed()),
                ))
            )
            .service(fs::Files::new("/", "./static").index_file("index.html"))
            .default_service(web::get().to(api::fallback))
    })
    // let ServiceController handle signals
    // .disable_signals()
    .bind(format!("{}:{}",config.bind_ip,config.bind_port))?
    .run())
}
