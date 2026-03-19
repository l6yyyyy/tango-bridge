#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env,
    future::IntoFuture,
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

use axum::{
    body::Bytes,
    extract::{
        ws::{Message, WebSocket},
        Request, WebSocketUpgrade,
    },
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use http::{Method, StatusCode};
use reqwest::Url;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::channel,
};
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;

mod adb;

async fn handle_websocket(ws: WebSocket) {
    let (mut ws_writer, mut ws_reader) = ws.split();
    let (mut adb_reader, mut adb_writer) = adb::connect_or_start().await.unwrap().into_split();

    let (ws_to_adb_sender, mut ws_to_adb_receiver) = channel::<Bytes>(16);
    let (adb_to_ws_sender, mut adb_to_ws_receiver) = channel::<Vec<u8>>(16);

    tokio::join!(
        async move {
            while let Some(Ok(message)) = ws_reader.next().await {
                if let Message::Binary(packet) = message {
                    if ws_to_adb_sender.send(packet).await.is_err() {
                        break;
                    }
                }
            }
        },
        async move {
            while let Some(buf) = ws_to_adb_receiver.recv().await {
                if adb_writer.write_all(buf.as_ref()).await.is_err() {
                    break;
                }
            }
            adb_writer.shutdown().await.unwrap();
        },
        async move {
            loop {
                let mut buf = vec![0; 1024 * 1024];
                match adb_reader.read(&mut buf).await {
                    Ok(0) | Err(_) => {
                        break;
                    }
                    Ok(n) => {
                        buf.truncate(n);
                        if adb_to_ws_sender.send(buf).await.is_err() {
                            break;
                        }
                    }
                }
            }
        },
        async move {
            while let Some(buf) = adb_to_ws_receiver.recv().await {
                if ws_writer.send(Message::binary(buf)).await.is_err() {
                    break;
                }
            }
            ws_writer.close().await.unwrap();
        }
    );
}

#[cfg(debug_assertions)]
const PROXY_HOST: &str = "https://tangoapp.dev";
#[cfg(not(debug_assertions))]
const PROXY_HOST: &str = "https://tangoapp.dev";

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

async fn proxy_request(request: Request) -> Result<Response, Response> {
    println!("proxy_request: {} {}", request.method(), request.uri());

    let url = Url::options()
        .base_url(Some(&Url::parse(PROXY_HOST).unwrap()))
        .parse(&request.uri().to_string())
        .map_err(|_| (StatusCode::BAD_REQUEST, "Bad Request").into_response())?;

    let mut headers = request.headers().clone();
    headers.insert("Host", url.host_str().unwrap().parse().unwrap());

    let (client, request) = CLIENT
        .get_or_init(|| reqwest::Client::new())
        .request(request.method().clone(), url)
        .headers(headers)
        .body(reqwest::Body::wrap_stream(
            request.into_body().into_data_stream(),
        ))
        .build_split();

    let request = request.map_err(|_| (StatusCode::BAD_REQUEST, "Bad Request").into_response())?;

    let response = client
        .execute(request)
        .await
        .map_err(|_| (StatusCode::BAD_GATEWAY, "Bad Gateway").into_response())?;

    Ok((
        response.status(),
        response.headers().clone(),
        axum::body::Body::new(reqwest::Body::from(response)),
    )
        .into_response())
}

#[tokio::main]
async fn main() {
    println!("Starting Tango Bridge in headless mode...");

    adb::connect_or_start()
        .await
        .unwrap()
        .shutdown()
        .await
        .unwrap();

    let app = Router::new()
        .nest(
            "/bridge",
            Router::new()
                .route("/ping", get(|| async { env!("CARGO_PKG_VERSION") }))
                .route(
                    "/",
                    get(|ws: WebSocketUpgrade| async { ws.on_upgrade(handle_websocket) }),
                )
                .route_layer(
                    CorsLayer::new()
                        .allow_methods([Method::GET, Method::POST])
                        .allow_origin(
                            [
                                "http://localhost:3002",
                                "https://tangoapp.dev",
                                "https://app.tangoapp.dev",
                                "https://beta.tangoapp.dev",
                                "https://tunnel.tangoapp.dev",
                            ]
                            .map(|x| x.parse().unwrap()),
                        )
                        .allow_private_network(true),
                ),
        )
        .fallback(proxy_request);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:15037")
        .await
        .unwrap();

    let token = CancellationToken::new();

    println!("Tango Bridge server started on 0.0.0.0:15037");
    println!("WebSocket endpoint: ws://your-nas:15037/bridge/");
    println!("Press Ctrl+C to stop");

    axum::serve(
        listener,
        app.with_graceful_shutdown(token.cancelled()),
    )
    .into_future()
    .await
    .unwrap();
}
