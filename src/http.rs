use gpui_http_client::{AsyncBody, HttpClient, Response};
use std::pin::Pin;
use std::sync::Arc;
use anyhow::Result;
use tokio::runtime::Runtime;

type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// A simple HTTP client backed by reqwest for fetching remote resources (e.g. map tiles).
/// It maintains its own Tokio runtime to ensure a reactor is available for reqwest's background operations.
pub struct ReqwestClient {
    client: reqwest::Client,
    runtime: Runtime,
}

impl ReqwestClient {
    pub fn new() -> Arc<Self> {
        let client = reqwest::Client::builder()
            .user_agent("ExifEditor/0.1")
            .build()
            .expect("failed to build reqwest client");
        
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("exif-editor-http")
            .build()
            .expect("failed to build tokio runtime");

        Arc::new(Self { client, runtime })
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

        let handle = self.runtime.handle().clone();

        Box::pin(async move {
            // Spawn the request on the Tokio runtime to ensure a reactor is present
            let result = handle.spawn(async move {
                let response = builder.send().await.map_err(|e| {
                    gpui_http_client::anyhow!("HTTP request failed: {e}")
                })?;
                
                let status = response.status();
                let mut header_map = Vec::new();
                for (key, value) in response.headers() {
                    header_map.push((key.as_str().to_string(), value.as_bytes().to_vec()));
                }
                
                let bytes = response.bytes().await.map_err(|e| {
                    gpui_http_client::anyhow!("Failed to read response body: {e}")
                })?;
                
                Ok::<_, anyhow::Error>((status.as_u16(), header_map, bytes))
            }).await.map_err(|e| gpui_http_client::anyhow!("Tokio task failed: {e}"))??;

            let (status, headers, bytes) = result;
            
            let mut http_response = Response::builder().status(status);
            for (key, value) in headers {
                http_response = http_response.header(key, value);
            }
            
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
