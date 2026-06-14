# axum-cgi

A lightweight library to run [Axum](https://github.com/tokio-rs/axum) applications as CGI scripts. This allows you to deploy Axum-based web applications in **shared hosting environments** or legacy systems that only support CGI.

---

## Features

- **Seamless Integration**: Convert your existing Axum `Router` into a CGI-compatible script with minimal changes.
- **Simple API**: Just call `.cgi()` on your Axum router and you're ready to go.
- **Shared Hosting Ready**: Designed to work in environments where CGI is the only available interface.

---

## Usage

```rust
use axum::Router;
use axum::response::Html;
use axum::routing::get;
use axum_cgi::RouterCgi;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(handler));
    app.cgi().await.unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
```

---

## Limitations

- **CGI Environment**: CGI has inherent limitations (e.g., process spawning per request). This library is designed for low-traffic or legacy use cases.
- **Performance**: Not suitable for high-performance applications. Consider using a dedicated async server for production workloads.
- **Streaming**: Response streaming is not supported due to CGI constraints.

---

## Why This Library?

This library was created to bridge the gap between modern async Rust web frameworks like Axum and **shared hosting environments** or legacy systems that only support CGI. It enables developers to:

- Deploy Axum apps on shared hosting with CGI support.
- Migrate legacy CGI scripts to Rust/Axum incrementally.
- Experiment with Axum in constrained environments.

---

## License

This project is licensed under the [MIT License](LICENSE.md).
