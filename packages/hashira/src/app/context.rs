use super::{client_router::ClientRouter, error_router::ClientErrorRouter, RenderLayout};
use crate::error::Error;
pub use crate::error::ResponseError;
use crate::web::ResponseExt;
use crate::{
    server::{Metadata, PageLinks, PageScripts},
    web::{Request, Response},
};
use http::StatusCode;
use route_recognizer::Params;
use serde::Serialize;
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};
use yew::{html::ChildrenProps, BaseComponent};

struct AppContextInner {
    // The `<meta>` tags of the page to render
    metadata: Metadata,

    // the <link> tags of the page to render
    links: PageLinks,

    // the <script> tags of the page to render
    scripts: PageScripts,
}

#[allow(dead_code)] // TODO: Ignore server only data
pub struct AppContext<C> {
    client_router: ClientRouter,
    client_error_router: Arc<ClientErrorRouter>,
    request: Option<Arc<Request>>,
    path: String,
    params: Params,
    layout: Option<RenderLayout<C>>,
    inner: Arc<Mutex<AppContextInner>>,
}

#[allow(dead_code)] // TODO: Ignore server only data
impl<C> AppContext<C> {
    pub fn new(
        request: Option<Arc<Request>>,
        client_router: ClientRouter,
        client_error_router: Arc<ClientErrorRouter>,
        path: String,
        layout: RenderLayout<C>,
        params: Params,
    ) -> Self {
        let inner = AppContextInner {
            metadata: Metadata::default(),
            links: PageLinks::default(),
            scripts: PageScripts::default(),
        };

        AppContext {
            path,
            params,
            request,
            client_router,
            layout: Some(layout),
            client_error_router,
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

impl<C> AppContext<C>
where
    C: BaseComponent<Properties = ChildrenProps>,
{
    pub fn add_metadata(&mut self, metadata: Metadata) {
        self.inner.lock().unwrap().metadata.extend(metadata);
    }

    pub fn add_links(&mut self, links: PageLinks) {
        self.inner.lock().unwrap().links.extend(links);
    }

    pub fn add_scripts(&mut self, scripts: PageScripts) {
        self.inner.lock().unwrap().scripts.extend(scripts);
    }

    pub fn request(&self) -> &Request {
        self.request
            .as_ref()
            .expect("no request is being processed")
    }

    pub fn params(&self) -> &Params {
        &self.params
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn render<COMP>(self, error: Option<ResponseError>) -> String
    where
        COMP: BaseComponent,
        COMP::Properties: Serialize + Default + Send + Clone,
    {
        let props = COMP::Properties::default();
        self.render_with_props::<COMP>(props, error).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn render_with_props<COMP>(
        self,
        props: COMP::Properties,
        error: Option<ResponseError>,
    ) -> String
    where
        COMP: BaseComponent,
        COMP::Properties: Serialize + Send + Clone,
    {
        use crate::server::{render_page_to_html, render_to_static_html, RenderPageOptions};

        let Self {
            layout,
            request,
            inner,
            params,
            client_router,
            client_error_router,
            path,
        } = self;

        let render_layout = layout.unwrap();

        let ctx = AppContext {
            params,
            request,
            path: path.clone(),
            layout: None,
            client_router: client_router.clone(),
            client_error_router: client_error_router.clone(),
            inner: inner.clone(),
        };

        let layout_node = render_layout(ctx).await;
        let layout = render_to_static_html(move || layout_node).await;

        let inner = inner.lock().unwrap();
        let links = inner.links.clone();
        let metadata = inner.metadata.clone();
        let scripts = inner.scripts.clone();

        let options = RenderPageOptions {
            path,
            error,
            layout,
            metadata,
            links,
            scripts,
            client_router,
            client_error_router,
        };

        let result_html = render_page_to_html::<COMP, C>(props, options)
            .await
            .unwrap();
        result_html
    }
}

pub struct RenderContext<COMP, C> {
    context: AppContext<C>,
    _marker: PhantomData<COMP>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<COMP, C> RenderContext<COMP, C> {
    pub(crate) fn new(context: AppContext<C>) -> Self {
        RenderContext {
            context,
            _marker: PhantomData,
        }
    }
}
impl<COMP, C> RenderContext<COMP, C>
where
    C: BaseComponent<Properties = ChildrenProps>,
{
    pub fn add_metadata(&mut self, metadata: Metadata) {
        self.context.add_metadata(metadata);
    }

    pub fn add_links(&mut self, links: PageLinks) {
        self.context.add_links(links);
    }

    pub fn add_scripts(&mut self, scripts: PageScripts) {
        self.context.add_scripts(scripts);
    }

    pub fn request(&self) -> &Request {
        self.context.request()
    }

    pub fn params(&self) -> &Params {
        self.context.params()
    }
}

impl<COMP, C> RenderContext<COMP, C>
where
    C: BaseComponent<Properties = ChildrenProps>,
    COMP: BaseComponent,
    COMP::Properties: Serialize + Send + Clone,
{
    /// Render the page and returns the `text/html` response.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn render(self) -> Response
    where
        COMP::Properties: Default,
    {
        let html = self.context.render::<COMP>(None).await;
        Response::html(html)
    }

    /// Render the page with the given props and returns the `text/html` response.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn render_with_props(self, props: COMP::Properties) -> Response {
        let html = self.context.render_with_props::<COMP>(props, None).await;
        Response::html(html)
    }

    /// Render the page and returns the `text/html` response.
    #[cfg(target_arch = "wasm32")]
    pub async fn render(self) -> Response
    where
        COMP::Properties: Default,
    {
        unreachable!("this is a server-only function")
    }

    /// Render the page with the given props and returns the `text/html` response.
    #[cfg(target_arch = "wasm32")]
    pub async fn render_with_props(self, _: COMP::Properties) -> Response {
        unreachable!("this is a server-only function")
    }

    /// Returns a `404` error.
    pub fn not_found(self) -> Result<Response, Error> {
        Err(ResponseError::from_status(StatusCode::NOT_FOUND).into())
    }
}
