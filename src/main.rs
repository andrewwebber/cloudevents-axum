use axum::{
    handler::{get, post},
    routing::BoxRoute,
    Router,
};
use cloudevents::Event;
use http::StatusCode;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

fn app() -> Router<BoxRoute> {
    Router::new()
        .route("/", get(|| async { "hello from cloudevents server" }))
        .route(
            "/",
            post(|event: Event| async move {
                println!("received cloudevent {}", &event);
                (StatusCode::OK, event)
            }),
        )
        .boxed()
}

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var(
            "RUST_LOG",
            "example_tracing_aka_logging=debug,tower_http=debug",
        )
    }
    tracing_subscriber::fmt::init();
    let service = app().layer(TraceLayer::new_for_http());
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(service.into_make_service())
        .await
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use cloudevents::binding::reqwest::{RequestBuilderExt, ResponseExt};
    use cloudevents::{EventBuilder, EventBuilderV10};
    use serde_json::json;

    #[tokio::test]
    async fn hello_world() {
        let s =
            axum::Server::bind(&"0.0.0.0:3000".parse().unwrap()).serve(app().into_make_service());
        let _ = tokio::spawn(s);

        let time = Utc::now();

        let j = json!({"hello": "world"});
        let req_event = EventBuilderV10::new()
            .id("0001")
            .ty("example.test")
            //TODO this is required now because the message deserializer implictly set default values
            // As soon as this defaulting doesn't happen anymore, we can remove it (Issues #40/#41)
            .time(time)
            .source("http://localhost")
            .data("application/json", j.clone())
            .extension("someint", "10")
            .build()
            .unwrap();

        let client = reqwest::Client::new();
        let resp = client
            .post("http://localhost:3000")
            .event(req_event)
            .unwrap()
            .send()
            .await
            .unwrap()
            .into_event()
            .await
            .unwrap();

        println!("client received event {}", resp);
    }
}
