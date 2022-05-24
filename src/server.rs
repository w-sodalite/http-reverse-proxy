use std::collections::HashSet;
use std::future::Future;
use std::net::SocketAddr;
use std::str::FromStr;

use anyhow::anyhow;
use hyper::{
    Body, Client as HttpClient, HeaderMap, Request, Response, Server as HttpServer, StatusCode,
};
use hyper::client::HttpConnector;
use hyper::http::request::Parts;
use hyper::service::{Service, service_fn};
use hyper_tls::HttpsConnector;
use log::{debug, info, LevelFilter};
use once_cell::sync::Lazy;
use route_recognizer::Router;
use tower::make::Shared;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::config::{Client, Config, Logging, Route};

static HTTP_RESPONSE_STATUS_502: u16 = 502;

static HTTP_RESPONSE_STATUS_404: u16 = 404;

static LOGGING_LEVEL_ROOT: &str = "root";

static HEADERS_REMOVED_ON_REQUEST: Lazy<HashSet<&str>> = Lazy::new(|| {
    HashSet::from([
        "connection",
        "keep-alive",
        "transfer-encoding",
        "te",
        "trailer",
        "proxy-authorization",
        "proxy-authenticate",
        "x-application-context",
        "upgrade",
        "host",
    ])
});

pub struct Server {
    port: u16,
    client: HttpClient<HttpsConnector<HttpConnector>, Body>,
    router: Router<Route>,
}

impl Server {
    pub fn with_config(config: Config) -> Server {
        let (server, client, logging, routes) = config.into_parts();
        // 日志
        init_logger(&logging);
        // 客户端
        let http_client = make_http_client(&client);
        // 路由
        let http_router = make_http_router(routes);
        Server {
            port: server.port(),
            client: http_client,
            router: http_router,
        }
    }

    pub async fn serve(self) -> anyhow::Result<()> {
        let Server {
            port,
            client,
            router,
        } = self;
        let proxy_service = make_proxy_service(router, client).map_err(|e| anyhow!(e))?;
        let proxy_service = ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .service(proxy_service);

        let address = format!("0.0.0.0:{}", port)
            .parse::<SocketAddr>()
            .map_err(|e| anyhow!(e))?;
        info!("Server bind: {}", address);
        HttpServer::bind(&address)
            .serve(Shared::new(proxy_service))
            .await
            .map_err(|e| anyhow!(e))
    }
}

fn init_logger(logging: &Logging) {
    let mut builder = env_logger::builder();
    for (module, level) in logging.level() {
        if let Ok(filter) = LevelFilter::from_str(level) {
            if module != LOGGING_LEVEL_ROOT {
                builder.filter(Some(module), filter);
            } else {
                builder.filter_level(filter);
            }
        }
    }
    builder.init();
}

fn make_http_client(client: &Client) -> HttpClient<HttpsConnector<HttpConnector>, Body> {
    let mut builder = HttpClient::builder();
    if client.pool_max_idle_per_host() > 0 {
        builder.pool_max_idle_per_host(client.pool_max_idle_per_host());
    }
    if let Some(timeout) = client.pool_idle_timeout() {
        builder.pool_idle_timeout(timeout);
    }
    builder.build(HttpsConnector::new())
}

fn make_http_router(routes: Vec<Route>) -> Router<Route> {
    let mut router = Router::new();
    for route in routes {
        let key = String::from(route.predicate());
        router.add(&key, route);
    }
    router
}

fn make_proxy_service(
    router: Router<Route>,
    client: HttpClient<HttpsConnector<HttpConnector>, Body>,
) -> anyhow::Result<
    impl Service<
        Request<Body>,
        Response=Response<Body>,
        Error=anyhow::Error,
        Future=impl Future<Output=Result<Response<Body>, anyhow::Error>> + Send + Sync + 'static,
    > + Clone,
> {
    let router = make_static_ref(router);
    let client = make_static_ref(client);
    let proxy_service = service_fn(move |request: Request<Body>| async move {
        let (parts, body) = request.into_parts();
        let Parts {
            uri,
            method,
            headers,
            ..
        } = parts;
        let path = uri.path();
        let recognize_result = router.recognize(path);
        match recognize_result {
            Ok(m) => {
                let params = m.params();
                let route = m.handler();
                debug!("matched route: {:?}, params: {:?}", route, params);
                let strip = route.strip();
                let uri = format!(
                    "{}{}",
                    route.uri(),
                    path.split('/')
                        .filter(|x| !x.is_empty())
                        .skip(strip)
                        .map(|x| format!("{}{}", "/", x))
                        .collect::<String>()
                );
                debug!("redirect uri: {}", uri);
                let mut filter_headers = HeaderMap::new();
                for (name, value) in headers {
                    if let Some(name) = name {
                        if !HEADERS_REMOVED_ON_REQUEST.contains(name.as_str()) {
                            filter_headers.insert(name, value);
                        }
                    }
                }
                let mut request = Request::builder()
                    .uri(uri)
                    .method(method)
                    .body(body)
                    .map_err(|e| anyhow!(e))?;
                *request.headers_mut() = filter_headers;
                client.request(request).await.or_else(|e| {
                    Response::builder()
                        .status(StatusCode::from_u16(HTTP_RESPONSE_STATUS_502).unwrap())
                        .body(Body::from(format!("{}", e)))
                        .map_err(|e| anyhow!(e))
                })
            }
            Err(_) => Response::builder()
                .status(StatusCode::from_u16(HTTP_RESPONSE_STATUS_404).unwrap())
                .body(Body::empty())
                .map_err(|e| anyhow!(e)),
        }
    });
    Ok(proxy_service)
}

fn make_static_ref<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}
