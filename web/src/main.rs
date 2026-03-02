//! Server entry-point – Axum + Leptos SSR.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use tower_http::services::ServeDir;

    use gaia_gmn_web::app::App;

    // ── Tracing ──────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gaia_gmn_web=info,tower_http=info".into()),
        )
        .init();

    // ── Configuration ────────────────────────────────────────────────
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options.clone();
    let addr = leptos_options.site_addr;
    let site_root = leptos_options.site_root.clone();

    let data_dir = std::env::var("GAIA_DATA_DIR").unwrap_or_else(|_| "/data".into());
    tracing::info!("Data directory: {data_dir}");

    // ── Detection database ───────────────────────────────────────────
    let db_path = std::path::PathBuf::from(
        std::env::var("GAIA_DB_PATH").unwrap_or_else(|_| format!("{data_dir}/detections.db")),
    );
    if let Err(e) = gaia_gmn_web::server::db::ensure_schema(&db_path) {
        tracing::error!("Cannot initialise detection database: {e}");
        std::process::exit(1);
    }
    tracing::info!("Detection DB ready at {}", db_path.display());

    // Store data dir in Leptos context so server functions can access it.
    let data_path = std::path::PathBuf::from(&data_dir);
    provide_context(data_path);

    // ── Routes ───────────────────────────────────────────────────────
    let routes = generate_route_list(App);

    let app = Router::new()
        // Serve the live.jpg from the capture data directory.
        .nest_service("/data", ServeDir::new(&data_dir))
        .leptos_routes(&leptos_options, routes, App)
        .nest_service(
            "/pkg",
            ServeDir::new(format!("{}/pkg", site_root.to_string())),
        )
        .with_state(leptos_options);

    // ── Start server ─────────────────────────────────────────────────
    tracing::info!("Gaia GMN Web listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // Required for the wasm lib target – unused.
}
