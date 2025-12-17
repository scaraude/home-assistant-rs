use crate::http::responses;
use crate::services::WebSocketBroadcaster;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::header::{
    CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE,
};
use hyper::http::HeaderValue;
use hyper::upgrade::Upgraded;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::select;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::{Message, Role};
use tracing::{debug, error, info, warn};

pub async fn handle_websocket_upgrade(
    req: Request<Incoming>,
    broadcaster: Arc<WebSocketBroadcaster>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let sec_websocket_key = match validate_websocket_request(&req) {
        Ok(key) => key,
        Err(response) => return Ok(response),
    };

    let accept_key = derive_accept_key(sec_websocket_key.as_bytes());

    let response = Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(CONNECTION, "Upgrade")
        .header(UPGRADE, "websocket")
        .header(SEC_WEBSOCKET_ACCEPT, accept_key)
        .body(Full::new(Bytes::new()))
        .expect("valid WebSocket upgrade response");

    tokio::spawn(async move {
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                if let Err(err) = handle_websocket_connection(upgraded, broadcaster).await {
                    error!(error = ?err, "WebSocket connection handler exited with error");
                }
            }
            Err(err) => error!(error = ?err, "Failed to upgrade HTTP connection to WebSocket"),
        }
    });

    Ok(response)
}

fn validate_websocket_request(req: &Request<Incoming>) -> Result<String, Response<Full<Bytes>>> {
    if req.method() != Method::GET {
        return Err(responses::bad_request_response(
            "WebSocket upgrade requires GET request",
        ));
    }

    if !header_contains(req.headers().get(CONNECTION), "upgrade") {
        return Err(responses::bad_request_response(
            "Missing Connection: upgrade header",
        ));
    }

    if !header_equals(req.headers().get(UPGRADE), "websocket") {
        return Err(responses::bad_request_response(
            "Missing Upgrade: websocket header",
        ));
    }

    if req
        .headers()
        .get(SEC_WEBSOCKET_VERSION)
        .and_then(|v| v.to_str().ok())
        != Some("13")
    {
        return Err(responses::bad_request_response(
            "Unsupported WebSocket version",
        ));
    }

    let sec_key = req
        .headers()
        .get(SEC_WEBSOCKET_KEY)
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| responses::bad_request_response("Missing Sec-WebSocket-Key header"))?;

    Ok(sec_key)
}

fn header_contains(header: Option<&HeaderValue>, needle: &str) -> bool {
    header
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case(needle))
        })
        .unwrap_or(false)
}

fn header_equals(header: Option<&HeaderValue>, expected: &str) -> bool {
    header
        .and_then(|value| value.to_str().ok())
        .map(|value| value.eq_ignore_ascii_case(expected))
        .unwrap_or(false)
}

pub async fn handle_websocket_connection(
    upgraded: Upgraded,
    broadcaster: Arc<WebSocketBroadcaster>,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    let ws_stream =
        WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Server, None).await;
    let mut ws_stream = ws_stream;

    let (client_id, mut event_rx) = broadcaster.add_client().await;
    info!(client_id, "WebSocket client connected");

    let mut last_error = None;

    loop {
        select! {
            event = event_rx.recv() => {
                match event {
                    Some(payload) => {
                        if let Err(err) = ws_stream.send(Message::Text(payload)).await {
                            warn!(client_id, error = ?err, "Failed to deliver event to WebSocket client");
                            last_error = Some(err);
                            break;
                        }
                    }
                    None => {
                        warn!(client_id, "Event channel closed for WebSocket client");
                        break;
                    }
                }
            }
            ws_message = ws_stream.next() => {
                match ws_message {
                    Some(Ok(Message::Ping(payload))) => {
                        if let Err(err) = ws_stream.send(Message::Pong(payload)).await {
                            warn!(client_id, error = ?err, "Failed to respond to ping");
                            last_error = Some(err);
                            break;
                        }
                    }
                    Some(Ok(Message::Close(frame))) => {
                        if let Err(err) = ws_stream.send(Message::Close(frame)).await {
                            last_error = Some(err);
                        }
                        break;
                    }
                    Some(Ok(Message::Text(text))) => {
                        debug!(client_id, message = %text, "Ignoring inbound WebSocket text message");
                    }
                    Some(Ok(Message::Binary(_payload))) => {
                        debug!(client_id, "Ignoring inbound WebSocket binary message");
                    }
                    Some(Ok(Message::Frame(_))) => {
                        debug!(client_id, "Ignoring inbound WebSocket frame message");
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Err(err)) => {
                        warn!(client_id, error = ?err, "WebSocket stream error");
                        last_error = Some(err);
                        break;
                    }
                    None => {
                        debug!(client_id, "WebSocket stream closed by peer");
                        break;
                    }
                }
            }
        }
    }

    broadcaster.remove_client(client_id).await;
    info!(client_id, "WebSocket client disconnected");
    match last_error {
        Some(err) => Err(err),
        None => Ok(()),
    }
}
