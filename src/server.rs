//! Axum server
use std::convert::Infallible;

use crate::app::App;
use axum::response::Response as AxumResponse;
use axum::{body::Body, http::Request, response::IntoResponse};
use axum::{routing::post, Router};
use get_port::tcp::TcpPort;
use get_port::{Ops, Range};
use leptos::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use std::net::SocketAddr;
use tower_http::services::ServeDir;

/// Axum server main entry point
pub async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // TODO: --verbose should use DEBUG
        .init();
    let conf = get_configuration(None).await.unwrap();
    tracing::debug!("Firing up Leptos app with config: {:?}", conf);
    let routes = generate_route_list(|cx| view! { cx, <App/> }).await;
    let client_dist = ServeDir::new(conf.leptos_options.site_root.clone());
    let leptos_options = conf.leptos_options.clone(); // A copy to move to the closure below.
    let not_found_service =
        tower::service_fn(move |req| not_found_handler(leptos_options.to_owned(), req));
    let app = Router::new()
        // server functions API routes
        .route("/api/*fn_name", post(leptos_axum::handle_server_fns))
        // application routes
        .leptos_routes(&conf.leptos_options, routes, |cx| view! { cx, <App/> })
        // static files are served as fallback (but *before* falling back to
        // error handler)
        .fallback_service(client_dist.clone().not_found_service(not_found_service))
        .with_state(conf.leptos_options.clone());
    let server = axum::Server::bind(&resolve_port_zero(conf.leptos_options.site_addr, 3000))
        .serve(app.into_make_service());
    tracing::info!("App is running at http://{}", server.local_addr());
    server.await.unwrap()
}

/// Resolve port 0 to an available port (using the given defaut + 1000 range).
fn resolve_port_zero(address: SocketAddr, default: u16) -> SocketAddr {
    let mut site_address = address.clone();
    if site_address.port() == u16::min_value() {
        let tcp_port = TcpPort::in_range(
            &address.ip().to_string(),
            Range {
                min: default,
                max: default + 1000,
            },
        )
        .unwrap();
        // TODO: This should be debug
        tracing::info!("Resolved port 0 to {}", tcp_port);
        site_address.set_port(tcp_port);
    }
    site_address
}

/// Handler for missing routes
///
/// On missing routes, just delegate to the leptos app, which has a route
/// fallback rendering 404 response.
async fn not_found_handler(
    options: LeptosOptions,
    req: Request<Body>,
) -> Result<AxumResponse, Infallible> {
    let handler =
        leptos_axum::render_app_to_stream(options.to_owned(), move |cx| view! {cx, <App/>});
    Ok(handler(req).await.into_response())
}
