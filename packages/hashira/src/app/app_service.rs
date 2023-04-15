use super::{
    error_router::{ErrorRouter, ServerErrorRouter},
    router::PageRouterWrapper,
    Params, RenderLayout, RequestContext, Route,
};
use crate::{
    error::ResponseError,
    web::{Body, IntoResponse, Request, Response, ResponseExt},
};
use http::StatusCode;
use matchit::Router;
use std::sync::Arc;

pub(crate) struct AppServiceInner {
    pub(crate) layout: RenderLayout,
    pub(crate) server_router: Router<Route>,
    pub(crate) client_router: PageRouterWrapper,
    pub(crate) server_error_router: ServerErrorRouter,
    pub(crate) client_error_router: Arc<ErrorRouter>,

    #[cfg(feature = "hooks")]
    pub(crate) hooks: Arc<crate::events::Hooks>,
}

enum ErrorSource {
    Response(Response),
    Error(ResponseError),
}

/// The root service used for handling the `hashira` application.
pub struct AppService(Arc<AppServiceInner>);

impl AppService {
    pub(crate) fn new(inner: Arc<AppServiceInner>) -> Self {
        Self(inner)
    }

    /// Create a context to be used in the request.
    pub fn create_context(
        &self,
        path: String,
        request: Arc<Request>,
        params: Params,
        error: Option<ResponseError>,
    ) -> RequestContext {
        let render_layout = self.0.layout.clone();
        let client_router = self.0.client_router.clone();
        let error_router = self.0.client_error_router.clone();

        RequestContext::new(
            request,
            client_router,
            error_router,
            error,
            render_layout,
            path,
            params,
        )
    }

    /// Returns the server router.
    pub fn server_router(&self) -> &Router<Route> {
        &self.0.server_router
    }

    /// Returns the page router.
    pub fn page_router(&self) -> &PageRouterWrapper {
        &self.0.client_router
    }

    /// Returns the router for handling error pages on the client.
    pub fn error_router(&self) -> &Arc<ErrorRouter> {
        &self.0.client_error_router
    }

    // TODO: Remove the path, we could take that value from the request
    /// Process the incoming request and return the response.
    pub async fn handle(&self, req: Request, path: &str) -> Response {
        let req = Arc::new(req);

        // Handle the request normally
        #[cfg(not(feature = "hooks"))]
        {
            self.handle_request(req, &path).await
        }

        #[cfg(feature = "hooks")]
        {
            use crate::{app::BoxFuture, events::Next};

            let hooks = &self.0.hooks.on_handle_hooks;

            if !hooks.is_empty() {
                return self.handle_request(req, &path).await;
            }

            let this = self.clone();
            let path = path.to_owned();
            let next = Box::new(move |req| {
                Box::pin(async move {
                    let fut = this.handle_request(req, &path);
                    let res = fut.await;
                    res
                }) as BoxFuture<Response>
            }) as Next;

            let handler = hooks.iter().fold(next, move |cur, next_handler| {
                let next_handler = next_handler.clone();
                Box::new(move |req| {
                    Box::pin(async move {
                        let fut = next_handler.on_handle(req, cur);
                        let res = fut.await;
                        res
                    })
                })
            }) as Next;

            // Handle the request
            handler(req).await
        }
    }

    async fn handle_request(&self, req: Arc<Request>, mut path: &str) -> Response {
        // We remove the trailing slash from the path,
        // when adding a path we ensure it cannot end with a slash
        // and should start with a slash

        //let mut path = path.as_str();

        path = path.trim();

        // FIXME: Ensure the path always starts with `/`
        debug_assert!(path.starts_with('/'));

        if path.len() > 1 && path.ends_with('/') {
            path = path.trim_end_matches('/');
        }

        match self.0.server_router.at(path) {
            Ok(mtch) => {
                let route = mtch.value;
                let method = req.method().into();

                if !route.method().matches(&method) {
                    return Response::with_status(StatusCode::METHOD_NOT_ALLOWED, Body::default());
                }

                let params = Params::from_iter(mtch.params.iter());
                let ctx = self.create_context(path.to_owned(), req.clone(), params, None);

                let res = route.handler().call(ctx).await;
                let status = res.status();
                if status.is_client_error() || status.is_server_error() {
                    return self
                        .handle_error(path, req, ErrorSource::Response(res))
                        .await;
                }

                res
            }
            Err(_) => {
                let src = ErrorSource::Error(ResponseError::from_status(StatusCode::NOT_FOUND));
                self.handle_error(path, req, src).await
            }
        }
    }

    async fn handle_error(&self, path: &str, req: Arc<Request>, src: ErrorSource) -> Response {
        let err = match src {
            ErrorSource::Response(res) => {
                let status = res.status();

                // We get the message from the error which may be attached to the response
                let message = res
                    .extensions()
                    .get::<ResponseError>()
                    .and_then(|e| e.message())
                    .map(|s| s.to_owned());
                ResponseError::from((status, message))
            }
            ErrorSource::Error(res) => res,
        };

        let status = err.status();
        match self.0.server_error_router.recognize_error(&status) {
            Some(error_handler) => {
                let params = Params::default();
                let ctx = self.create_context(path.to_owned(), req, params, Some(err));

                match error_handler.call(ctx, status).await {
                    Ok(res) => res,
                    Err(err) => match err.downcast::<ResponseError>() {
                        Ok(err) => (*err).into_response(),
                        Err(err) => {
                            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
                        }
                    },
                }
            }
            None => err.into_response(),
        }
    }
}

impl Clone for AppService {
    fn clone(&self) -> Self {
        AppService(self.0.clone())
    }
}
