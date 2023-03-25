mod handler;
mod service;

fn init_logs() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "manteau=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_ansi(cfg!(debug_assertions)))
        .try_init();
}

fn address() -> std::net::SocketAddr {
    let host = std::env::var("HOST")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| std::net::IpAddr::from(std::net::Ipv4Addr::new(127, 0, 0, 1)));
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3000);
    std::net::SocketAddr::from((host, port))
}

#[tokio::main]
async fn main() {
    init_logs();

    let app = axum::Router::new()
        .route(
            "/api/torznab",
            axum::routing::get(handler::api::torznab::handler),
        )
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let addr = address();
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("signal received, starting graceful shutdown");
}
