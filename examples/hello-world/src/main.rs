use axum::Router;
use axum::response::Html;
use axum::routing::get;
use axum_cgi::RouterCgi;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let app = Router::new().route("/", get(handler));

    app.cgi().await.unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
