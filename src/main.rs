use std::net::SocketAddr;
use warp::{Filter, http};
use hyper::{Body, Client, Request, Uri};
use hyper_tls::HttpsConnector;
use clap::Parser;
use anyhow::{Result, anyhow};
use log::{info, error};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// HTTP listening port
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Target HTTPS backend URL (e.g., https://api.example.com)
    #[arg(short, long)]
    target: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    // Validate target URL
    let target = args.target.parse::<Uri>()
        .map_err(|e| anyhow!("Invalid target URL: {}", e))?;
    if target.scheme_str() != Some("https") {
        return Err(anyhow!("Target URL must use HTTPS scheme"));
    }

    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, Body>(https);
    let client = warp::any().map(move || client.clone());

    let target_filter = warp::any().map(move || target.clone());

    // Create the proxy route
    let proxy = warp::path::full()
        .and(warp::method())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .and(client)
        .and(target_filter)
        .and_then(handle_request);

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
    info!("Starting proxy server on http://{}", addr);
    info!("Forwarding requests to {}", args.target);

    warp::serve(proxy).run(addr).await;
    Ok(())
}

async fn handle_request(
    path: warp::path::FullPath,
    method: http::Method,
    headers: http::HeaderMap,
    body: bytes::Bytes,
    client: Client<HttpsConnector<hyper::client::HttpConnector>>,
    target: Uri,
) -> Result<impl warp::Reply, warp::Rejection> {
    let path_and_query = path.as_str().to_string();
    
    // Construct the target URI
    let uri = format!(
        "{}{}{}",
        target,
        path_and_query,
        if let Some(q) = target.query() {
            format!("?{}", q)
        } else {
            String::new()
        }
    );

    let uri: Uri = uri.parse().map_err(|_| warp::reject::not_found())?;

    // Build the proxied request
    let mut proxy_req = Request::builder()
        .method(method)
        .uri(uri);

    // Copy headers
    if let Some(headers_mut) = proxy_req.headers_mut() {
        *headers_mut = headers.clone();
    }

    // Create the request with the body
    let proxy_req = proxy_req
        .body(Body::from(body))
        .map_err(|_| warp::reject::not_found())?;

    // Send the request
    match client.request(proxy_req).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("Proxy request failed: {}", e);
            Err(warp::reject::not_found())
        }
    }
}
