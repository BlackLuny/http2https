# HTTP to HTTPS Proxy

A simple and efficient HTTP to HTTPS proxy tool written in Rust. This tool listens for HTTP requests on a specified port and forwards them to a configured HTTPS backend while maintaining all request properties (headers, body, method, etc.).

## Features

- HTTP to HTTPS request forwarding
- Configurable listening port
- Configurable target HTTPS backend
- Preserves all request headers and body
- Supports all HTTP methods (GET, POST, PUT, DELETE, etc.)
- Detailed logging with different log levels
- Error handling and reporting

## Prerequisites

- Rust and Cargo (latest stable version)

## Installation

1. Clone the repository:
```bash
git clone [repository-url]
cd http-to-https-proxy
```

2. Build the project:
```bash
cargo build --release
```

The compiled binary will be available at `target/release/http-to-https-proxy`

## Usage

### Basic Usage

```bash
# Run with default port (8080)
http-to-https-proxy -t https://api.example.com

# Run with custom port
http-to-https-proxy -p 8081 -t https://api.example.com

# Run with debug logging
RUST_LOG=debug http-to-https-proxy -p 8081 -t https://api.example.com
```

### Command Line Arguments

- `-p, --port <PORT>`: HTTP listening port (default: 8080)
- `-t, --target <URL>`: Target HTTPS backend URL (required)

### Environment Variables

- `RUST_LOG`: Log level (error, warn, info, debug, trace)

## Examples

1. Forward requests to a secure API:
```bash
http-to-https-proxy -p 8080 -t https://api.secure-service.com
```

2. Run with detailed logging:
```bash
RUST_LOG=debug http-to-https-proxy -p 8080 -t https://api.secure-service.com
```

3. Test the proxy with curl:
```bash
# Test GET request
curl -v http://localhost:8080/api/endpoint

# Test POST request
curl -v -X POST -d "data=test" http://localhost:8080/api/endpoint
```

## Error Handling

The proxy will:
- Return 404 if the backend request fails
- Return 400 if the request is malformed
- Log detailed error information when RUST_LOG is set to debug or trace

## Development

### Building from Source

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

### Running Tests

```bash
cargo test
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
