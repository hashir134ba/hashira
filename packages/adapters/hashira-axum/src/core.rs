use axum::{response::IntoResponse, routing::get_service, Extension, Router};
use hashira::{
    app::AppService,
    web::{Body, Request, Response},
};
use hyper::{body::to_bytes, StatusCode};
use tower_http::services::ServeDir;

// Returns a router for a `Axum` application.
pub fn router(app_service: AppService) -> Router {
    let static_dir = hashira::env::get_static_dir();
    let serve_dir = get_current_dir().join("public");

    Router::new()
        .nest_service(&static_dir, get_service(ServeDir::new(serve_dir)))
        .fallback(handle_request)
        .layer(Extension(app_service))
}

/// Handle a request.
pub async fn handle_request(
    Extension(service): Extension<AppService>,
    axum_request: Request<axum::body::Body>,
) -> impl IntoResponse {
    match map_request(axum_request).await {
        Ok(req) => {
            let res = service.handle(req).await;
            map_response(res)
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

async fn map_request(mut req: Request<axum::body::Body>) -> Result<Request, axum::Error> {
    let mut builder = Request::builder()
        .version(req.version())
        .method(req.method())
        .uri(req.uri());

    if let Some(headers) = builder.headers_mut() {
        *headers = std::mem::take(req.headers_mut());
    }

    if let Some(ext) = builder.extensions_mut() {
        *ext = std::mem::take(req.extensions_mut());
    }

    let axum_body = req.into_body();
    let bytes = to_bytes(axum_body).await.map_err(axum::Error::new)?;
    let ret = builder.body(Body::from(bytes)).map_err(axum::Error::new)?;
    Ok(ret)
}

fn map_response(mut res: Response) -> axum::response::Response {
    let mut builder = axum::response::Response::builder()
        .version(res.version())
        .status(res.status());

    if let Some(headers) = builder.headers_mut() {
        *headers = std::mem::take(res.headers_mut());
    }

    if let Some(ext) = builder.extensions_mut() {
        *ext = std::mem::take(res.extensions_mut());
    }

    let body = match res.into_body().into_inner() {
        hashira::web::BodyInner::Bytes(bytes) => axum::body::Body::from(bytes),
        hashira::web::BodyInner::Stream(stream) => axum::body::Body::wrap_stream(stream),
    };

    builder.body(axum::body::boxed(body)).unwrap()
}

fn get_current_dir() -> std::path::PathBuf {
    let mut current_dir = std::env::current_exe().expect("failed to get current directory");
    current_dir.pop();
    current_dir
}