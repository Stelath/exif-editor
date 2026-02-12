use gpui_http_client::{AsyncBody, HttpClient, Response};
use std::pin::Pin;
use std::sync::Arc;

type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// A simple HTTP client backed by reqwest for fetching remote resources (e.g. map tiles).
pub struct ReqwestClient {
    client: reqwest::Client,
}

impl ReqwestClient {
    pub fn new() -> Arc<Self> {
        let client = reqwest::Client::builder()
            .user_agent("ExifEditor/0.1")
            .build()
            .expect("failed to build reqwest client");
        Arc::new(Self { client })
    }
}

impl HttpClient for ReqwestClient {
    fn send(
        &self,
        req: gpui_http_client::Request<AsyncBody>,
    ) -> BoxFuture<'static, gpui_http_client::Result<Response<AsyncBody>>> {
        let (parts, _body) = req.into_parts();
        let url = parts.uri.to_string();
        let method = parts.method;

        let reqwest_method =
            reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET);

        let mut builder = self.client.request(reqwest_method, &url);
        for (key, value) in &parts.headers {
            builder = builder.header(key.as_str(), value.as_bytes());
        }

        Box::pin(async move {
            let response = builder.send().await.map_err(|e| {
                gpui_http_client::anyhow!("HTTP request failed: {e}")
            })?;
            let status = response.status();
            let mut http_response = Response::builder().status(status.as_u16());
            for (key, value) in response.headers() {
                http_response = http_response.header(key.as_str(), value.as_bytes());
            }
            let bytes = response.bytes().await.map_err(|e| {
                gpui_http_client::anyhow!("Failed to read response body: {e}")
            })?;
            let body = AsyncBody::from_bytes(bytes);
            http_response.body(body).map_err(|e| {
                gpui_http_client::anyhow!("Failed to build response: {e}")
            })
        })
    }

    fn user_agent(&self) -> Option<&gpui_http_client::http::HeaderValue> {
        None
    }

    fn proxy(&self) -> Option<&gpui_http_client::Url> {
        None
    }

    fn type_name(&self) -> &'static str {
        "ReqwestClient"
    }
}
