use std::net::SocketAddr;
use std::time::Instant;
use warp::{Filter, http};
use hyper::{Body, Client, Request, Uri, StatusCode};
use hyper_rustls::HttpsConnectorBuilder;
use clap::Parser;
use anyhow::{Result, anyhow};
use log::{info, error, debug};

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

    let target_filter = warp::any().map(move || target.clone());

    // Create the proxy route
    let proxy = warp::path::full()
        .and(warp::method())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .and(target_filter)
        .and_then(handle_request);

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
    info!("Starting proxy server on http://{}", addr);
    info!("Forwarding requests to {}", args.target);
    info!("TLS implementation: rustls (no connection pooling)");

    warp::serve(proxy).run(addr).await;
    Ok(())
}

async fn handle_request(
    path: warp::path::FullPath,
    method: http::Method,
    headers: http::HeaderMap,
    body: bytes::Bytes,
    target: Uri,
) -> Result<impl warp::Reply, warp::Rejection> {
    let start_time = Instant::now();
    let path_and_query = path.as_str();
    
    // Create a new HTTPS client for each request
    let https = HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_only()
        .enable_http1()
        .build();

    let client = Client::builder()
        .pool_idle_timeout(std::time::Duration::from_secs(0)) // Disable idle pooling
        .pool_max_idle_per_host(0) // Disable connection pooling
        .build::<_, Body>(https);
    
    // Construct the target URI, properly handling the path
    let target_path = target.path().trim_end_matches('/');
    let request_path = path_and_query.trim_start_matches('/');
    
    let uri = format!(
        "{}{}{}{}",
        target,
        if target_path.is_empty() { "" } else { "/" },
        request_path,
        if let Some(q) = target.query() {
            format!("?{}", q)
        } else {
            String::new()
        }
    );

    let uri: Uri = uri.parse().map_err(|e| {
        error!("Failed to parse target URI: {}", e);
        warp::reject::not_found()
    })?;

    info!("Incoming request: {} {} -> {}", method, path_and_query, uri);
    debug!("Request headers: {:?}", headers);
    
    if !body.is_empty() {
        debug!("Request body size: {} bytes", body.len());
    }

    // Build the proxied request
    let mut proxy_req = Request::builder()
        .method(method.clone())
        .uri(uri.clone());

    // Copy headers
    if let Some(headers_mut) = proxy_req.headers_mut() {
        // Copy original headers except those we want to modify
        for (key, value) in headers.iter() {
            if key != "host" && key != "connection" {
                headers_mut.insert(key, value.clone());
            }
        }
        
        // Set the correct Host header
        if let Some(host) = uri.host() {
            if let Some(port) = uri.port_u16() {
                headers_mut.insert("host", format!("{}:{}", host, port).parse().unwrap());
            } else {
                headers_mut.insert("host", host.parse().unwrap());
            }
        }

        // Add proxy-related headers
        headers_mut.insert("x-forwarded-proto", "http".parse().unwrap());
        if let Some(client_ip) = headers.get("x-real-ip").or(headers.get("x-forwarded-for")) {
            headers_mut.insert("x-forwarded-for", client_ip.clone());
        }
        
        // Force close connection
        headers_mut.insert("connection", "close".parse().unwrap());
    }

    // Create the request with the body
    let proxy_req = proxy_req
        .body(Body::from(body))
        .map_err(|e| {
            error!("Failed to create proxy request: {}", e);
            warp::reject::not_found()
        })?;

    // Send the request
    match client.request(proxy_req).await {
        Ok(res) => {
            let status = res.status();
            let elapsed = start_time.elapsed();
            
            if status.is_success() {
                info!("Request successful: {} {} -> {} ({}ms)", 
                    method, path_and_query, status, elapsed.as_millis());
            } else {
                error!("Request failed: {} {} -> {} ({}ms)", 
                    method, path_and_query, status, elapsed.as_millis());
            }

            debug!("Response headers: {:?}", res.headers());
            Ok(res)
        }
        Err(e) => {
            let elapsed = start_time.elapsed();
            error!("Request error: {} {} -> {} ({}ms)", 
                method, path_and_query, e, elapsed.as_millis());
            Err(warp::reject::not_found())
        }
    }
}
