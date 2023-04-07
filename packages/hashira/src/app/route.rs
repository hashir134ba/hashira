use http::Method;

use super::PageHandler;
use crate::components::AnyComponent;

// Represents a client-side page route, containing a component and a path pattern.
pub struct ClientPageRoute {
    pub(crate) component: AnyComponent<serde_json::Value>, // The component for this page route.
    pub(crate) path: String,                               // The path pattern for this page route.
}

impl ClientPageRoute {
    // Renders the component for this page route with the given props.
    pub fn render(&self, props: serde_json::Value) -> yew::Html {
        self.component.render_with_props(props)
    }

    // Returns a reference to the path pattern for this page route.
    pub fn path(&self) -> &str {
        self.path.as_str()
    }
}

/// Represents an HTTP method as a bit field. This is a compact representation
/// of the HTTP method that allows for efficient matching of multiple methods
/// at once.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct HttpMethod(u8);

impl HttpMethod {
    /// The HTTP GET method.
    pub const GET: HttpMethod =     HttpMethod(0b0001);

    /// The HTTP POST method.
    pub const POST: HttpMethod =    HttpMethod(0b0010);

    /// The HTTP PUT method.
    pub const PUT: HttpMethod =     HttpMethod(0b0100);

    /// The HTTP PATCH method.
    pub const PATCH: HttpMethod =   HttpMethod(0b1000);

    /// The HTTP DELETE method.
    pub const DELETE: HttpMethod =  HttpMethod(0b0001_0000);

    /// The HTTP HEAD method.
    pub const HEAD: HttpMethod =    HttpMethod(0b0010_0000);

    /// The HTTP OPTIONS method.
    pub const OPTIONS: HttpMethod = HttpMethod(0b0100_0000);

    /// The HTTP TRACE method.
    pub const TRACE: HttpMethod =   HttpMethod(0b1000_0000);

    /// Returns true if this `HttpMethod` matches the given `HttpMethod`.
    ///
    /// Matching is done by bitwise ANDing the bit fields of the two `HttpMethod`s.
    /// If the result is non-zero, the two methods match.
    pub fn matches(&self, other: &HttpMethod) -> bool {
        (self.0 & other.0) != 0
    }
}

impl std::ops::BitOr for HttpMethod {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        HttpMethod(self.0 | other.0)
    }
}

impl From<&Method> for HttpMethod {
    fn from(value: &Method) -> Self {
        match *value {
            Method::GET => HttpMethod::GET,
            Method::POST => HttpMethod::POST,
            Method::PUT => HttpMethod::PUT,
            Method::DELETE => HttpMethod::DELETE,
            Method::HEAD => HttpMethod::HEAD,
            Method::OPTIONS => HttpMethod::OPTIONS,
            Method::PATCH => HttpMethod::PATCH,
            Method::TRACE => HttpMethod::TRACE,
            _ => panic!("unsupported http method: {value}"),
        }
    }
}

impl From<Method> for HttpMethod {
    fn from(value: Method) -> Self {
        HttpMethod::from(&value)
    }
}

/// Represents a route for a web server request, including the path, HTTP method,
/// and handler function for the request.
pub struct Route {
    /// The path that the route matches, e.g. "/users/:id" or "/login".
    path: String,
    /// The HTTP method that the route matches, e.g. HttpMethod::GET or HttpMethod::POST.
    method: HttpMethod,
    /// The handler function that should be called when this route matches a request.
    handler: PageHandler,
}

impl Route {
    /// Creates a new `ServerPageRoute` with the given path, HTTP method, and handler function.
    pub fn new(path: &str, method: HttpMethod, handler: PageHandler) -> Self {
        assert!(path.starts_with("/"), "page path must start with `/`");

        Route {
            path: path.to_owned(),
            method,
            handler,
        }
    }

    /// Creates a new `Route` with the HTTP method set to POST.
    pub fn post(path: &str, handler: PageHandler) -> Self {
        Self::new(path, HttpMethod::POST, handler)
    }

    /// Creates a new `Route` with the HTTP method set to GET.
    pub fn get(path: &str, handler: PageHandler) -> Self {
        Self::new(path, HttpMethod::GET, handler)
    }

    /// Creates a new `Route` with the HTTP method set to HEAD.
    pub fn head(path: &str, handler: PageHandler) -> Self {
        Self::new(path, HttpMethod::HEAD, handler)
    }

    /// Creates a new `Route` with the HTTP method set to PUT.
    pub fn put(path: &str, handler: PageHandler) -> Self {
        Self::new(path, HttpMethod::PUT, handler)
    }

    /// Creates a new `Route` with the HTTP method set to DELETE.
    pub fn delete(path: &str, handler: PageHandler) -> Self {
        Self::new(path, HttpMethod::DELETE, handler)
    }

    /// Creates a new `Route` with the HTTP method set to OPTIONS.
    pub fn options(path: &str, handler: PageHandler) -> Self {
        Self::new(path, HttpMethod::OPTIONS, handler)
    }

    /// Creates a new `Route` with the HTTP method set to PATCH.
    pub fn patch(path: &str, handler: PageHandler) -> Self {
        Self::new(path, HttpMethod::PATCH, handler)
    }

    /// Returns a reference to the path for this `Route`.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the HTTP method for this `Route`.
    pub fn method(&self) -> HttpMethod {
        self.method
    }

    /// Returns a reference to the handler function for this `Route`.
    pub fn handler(&self) -> &PageHandler {
        &self.handler
    }
}