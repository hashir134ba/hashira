pub mod adapter;
pub mod app;
pub mod components;
pub mod env;
pub mod error;
pub mod hooks;
pub mod routing;
pub mod server;
pub mod web;

// Allow public?
pub(crate) mod context;

#[cfg(target_arch = "wasm32")]
pub mod client;

#[cfg(feature = "hooks")]
pub mod events;

pub(crate) mod types;

// Macros
pub use hashira_macros::*;

mod reexports {
    pub use async_trait::async_trait;
}

pub use reexports::*;

/// Constants.
pub mod consts {
    /// A constants indicating whether if the application is running on the server.
    #[cfg(not(target_arch = "wasm32"))]
    pub const IS_SERVER: bool = true;

    /// A constants indicating whether if the application is running on the server.
    #[cfg(target_arch = "wasm32")]
    pub const IS_SERVER: bool = false;
}
