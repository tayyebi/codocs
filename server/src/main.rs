mod auth;
mod config;
mod error;
mod federation;
mod models;
mod routes;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;

use config::Config;
use routes::{
    api::{admin, auth as api_auth, notes},
    ap::{actor, inbox, outbox, webfinger},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("codocs=info".parse().unwrap()),
        )
        .init();

    let cfg = Config::from_env();
    let port = cfg.port;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await
        .expect("failed to connect to database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    tracing::info!("Codocs server starting on port {port}");
    tracing::info!("Instance: {}", cfg.instance_url);

    let http_client = reqwest::Client::builder()
        .user_agent("Codocs/0.1 ActivityPub")
        .build()
        .expect("failed to build http client");

    let cfg_data = web::Data::new(cfg.clone());
    let pool_data = web::Data::new(pool);
    let http_data = web::Data::new(http_client);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(cfg_data.clone())
            .app_data(pool_data.clone())
            .app_data(http_data.clone())
            .app_data(web::JsonConfig::default().error_handler(|err, _| {
                let msg = err.to_string();
                actix_web::error::InternalError::from_response(
                    err,
                    actix_web::HttpResponse::BadRequest()
                        .json(serde_json::json!({"error": msg})),
                )
                .into()
            }))
            // ── ActivityPub ─────────────────────────────────────────────
            .route("/.well-known/webfinger", web::get().to(webfinger::webfinger))
            .route("/users/{username}", web::get().to(actor::get_actor))
            .route("/users/{username}/inbox", web::post().to(inbox::post_inbox))
            .route("/users/{username}/outbox", web::get().to(outbox::get_outbox))
            // ── REST API ─────────────────────────────────────────────────
            .service(
                web::scope("/api")
                    .route("/auth/register", web::post().to(api_auth::register))
                    .route("/auth/login", web::post().to(api_auth::login))
                    .route("/notes", web::get().to(notes::list_notes))
                    .route("/notes", web::post().to(notes::create_note))
                    .route("/notes/{id}", web::delete().to(notes::delete_note))
                    .route("/admin/users", web::get().to(admin::list_users))
                    .route("/admin/users/{id}", web::delete().to(admin::delete_user))
                    .route("/admin/users/{id}/promote", web::post().to(admin::promote_user)),
            )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
