mod handler;
mod hooks;

pub use handler::*;
pub use hooks::*;

use crate::{app::RequestContext, routing::RouteMethod, types::BoxFuture, web::IntoJsonResponse};

/// An action that can be execute on the server.
pub trait Action: 'static {
    /// The type of the body of the action response.
    type Response: IntoJsonResponse + 'static;

    /// The path of the route.
    fn route() -> &'static str;

    /// Returns the methods this action can be called:
    fn method() -> RouteMethod {
        RouteMethod::GET
            | RouteMethod::POST
            | RouteMethod::PUT
            | RouteMethod::PATCH
            | RouteMethod::DELETE
    }

    /// Call this action and returns a response.
    fn call(ctx: RequestContext) -> BoxFuture<crate::Result<Self::Response>>;
}
