#[cfg(not(target_arch = "wasm32"))]
mod server;

// Starts the server
#[cfg(not(target_arch = "wasm32"))]
#[actix_web::main]
async fn main() -> Result<(), hashira::error::Error> {
    crate::server::start_server().await
}

// Starts client
#[cfg(target_arch = "wasm32")]
fn main() {
    // Here any additional logic to run on the client
}
