//! Discord webhook notifier for device availability alerts.
//!
//! The Pi regularly loses internet connectivity, so delivery is best-effort
//! with a persistent fallback: a couple of immediate retries, then the
//! message is queued in SQLite and flushed periodically once the network
//! is back.

use std::sync::Arc;
use std::time::Duration;

use crate::db::Database;
use crate::events::SystemEvent;
use chrono::Utc;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::header::{CONTENT_TYPE, HOST};
use hyper::{Request, StatusCode};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// Immediate delivery attempts before falling back to the queue.
const IMMEDIATE_ATTEMPTS: u32 = 2;
/// Delay between immediate attempts.
const IMMEDIATE_RETRY_DELAY: Duration = Duration::from_secs(15);
/// Interval between queue flush attempts (also fires once at startup).
const FLUSH_INTERVAL: Duration = Duration::from_secs(300);
/// Max queued messages sent per flush round.
const FLUSH_BATCH_SIZE: usize = 10;
/// Timeout for a single webhook request.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Delivery outcome of a single webhook POST.
enum Delivery {
    Sent,
    /// Transient failure (network down, 429, 5xx): keep the message.
    Retry(String),
    /// Permanent failure (bad webhook URL, 4xx): drop the message.
    Reject(String),
}

pub struct NotifierService {
    db: Arc<Database>,
    event_rx: broadcast::Receiver<SystemEvent>,
    webhook_url: Option<String>,
}

impl NotifierService {
    pub fn new(db: Arc<Database>, event_rx: broadcast::Receiver<SystemEvent>) -> Self {
        let webhook_url = std::env::var("DISCORD_WEBHOOK_URL")
            .ok()
            .filter(|url| !url.trim().is_empty());
        Self {
            db,
            event_rx,
            webhook_url,
        }
    }

    pub async fn run(mut self) {
        match &self.webhook_url {
            Some(_) => info!("NotifierService started (Discord webhook configured)"),
            None => {
                warn!("NotifierService started without DISCORD_WEBHOOK_URL - alerts disabled")
            }
        }

        let mut flush_interval = tokio::time::interval(FLUSH_INTERVAL);

        loop {
            tokio::select! {
                event = self.event_rx.recv() => match event {
                    Ok(SystemEvent::DeviceAvailability {
                        device_name,
                        online,
                        was_online,
                        timestamp,
                        ..
                    }) => {
                        self.handle_availability(&device_name, online, was_online, timestamp.timestamp())
                            .await;
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(skipped_events = skipped, "NotifierService lagged behind the event bus");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        warn!("Event bus closed; NotifierService exiting");
                        break;
                    }
                },
                _ = flush_interval.tick() => self.flush_pending().await,
            }
        }
    }

    async fn handle_availability(
        &self,
        device_name: &str,
        online: bool,
        was_online: Option<bool>,
        timestamp: i64,
    ) {
        // Alert on any transition to offline; for online, only announce a
        // recovery (first-seen online states stay silent).
        let message = if !online {
            format!("🔴 Capteur **{device_name}** hors ligne depuis <t:{timestamp}:R>")
        } else if was_online == Some(false) {
            format!("🟢 Capteur **{device_name}** de nouveau en ligne (<t:{timestamp}:R>)")
        } else {
            return;
        };

        if self.webhook_url.is_none() {
            debug!(message = %message, "No webhook configured, dropping notification");
            return;
        }

        for attempt in 1..=IMMEDIATE_ATTEMPTS {
            match self.send(&message).await {
                Delivery::Sent => {
                    info!(message = %message, "Notification delivered");
                    return;
                }
                Delivery::Reject(reason) => {
                    error!(reason = %reason, message = %message, "Notification rejected by Discord, dropping");
                    return;
                }
                Delivery::Retry(reason) => {
                    warn!(
                        attempt = attempt,
                        reason = %reason,
                        "Notification delivery failed"
                    );
                    if attempt < IMMEDIATE_ATTEMPTS {
                        tokio::time::sleep(IMMEDIATE_RETRY_DELAY).await;
                    }
                }
            }
        }

        info!(message = %message, "Queueing notification for later delivery");
        if let Err(e) = self.db.enqueue_notification(&message, Utc::now().timestamp()) {
            error!(error = %e, "Failed to queue notification");
        }
    }

    /// Try to deliver queued notifications, oldest first. Stops at the first
    /// transient failure (the network is probably still down).
    async fn flush_pending(&self) {
        if self.webhook_url.is_none() {
            return;
        }

        let pending = match self.db.get_pending_notifications(FLUSH_BATCH_SIZE) {
            Ok(pending) => pending,
            Err(e) => {
                error!(error = %e, "Failed to read pending notifications");
                return;
            }
        };

        if pending.is_empty() {
            return;
        }

        info!(count = pending.len(), "Flushing pending notifications");
        for notification in pending {
            match self.send(&notification.message).await {
                Delivery::Sent => {
                    info!(id = notification.id, "Queued notification delivered");
                    if let Err(e) = self.db.delete_notification(notification.id) {
                        error!(error = %e, id = notification.id, "Failed to delete delivered notification");
                    }
                }
                Delivery::Reject(reason) => {
                    error!(
                        id = notification.id,
                        reason = %reason,
                        "Queued notification rejected by Discord, dropping"
                    );
                    if let Err(e) = self.db.delete_notification(notification.id) {
                        error!(error = %e, id = notification.id, "Failed to delete rejected notification");
                    }
                }
                Delivery::Retry(reason) => {
                    debug!(
                        id = notification.id,
                        attempts = notification.attempts + 1,
                        queued_at = notification.created_at,
                        reason = %reason,
                        "Still unable to deliver, keeping queue"
                    );
                    if let Err(e) = self
                        .db
                        .record_notification_attempt(notification.id, Utc::now().timestamp())
                    {
                        error!(error = %e, id = notification.id, "Failed to record delivery attempt");
                    }
                    break;
                }
            }
        }
    }

    async fn send(&self, message: &str) -> Delivery {
        let Some(url) = &self.webhook_url else {
            return Delivery::Reject("no webhook configured".into());
        };

        let body = serde_json::json!({ "content": message }).to_string();
        match tokio::time::timeout(REQUEST_TIMEOUT, post_json(url, body)).await {
            Ok(Ok(status)) => {
                if status.is_success() {
                    Delivery::Sent
                } else if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                    Delivery::Retry(format!("HTTP {status}"))
                } else {
                    Delivery::Reject(format!("HTTP {status}"))
                }
            }
            Ok(Err(e)) => Delivery::Retry(e),
            Err(_) => Delivery::Retry("request timed out".into()),
        }
    }
}

/// Minimal HTTPS POST built on the crates already in the tree (hyper +
/// tokio-rustls); avoids pulling a full HTTP client stack onto the Pi.
async fn post_json(url: &str, body: String) -> Result<StatusCode, String> {
    let (host, port, path) = parse_https_url(url)?;

    let tcp = tokio::net::TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|e| format!("tcp connect: {e}"))?;

    let mut roots = tokio_rustls::rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().certs {
        let _ = roots.add(cert);
    }
    if roots.is_empty() {
        return Err("no CA certificates available".into());
    }

    let config = tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let server_name = tokio_rustls::rustls::pki_types::ServerName::try_from(host.clone())
        .map_err(|e| format!("invalid server name: {e}"))?;
    let tls = tokio_rustls::TlsConnector::from(Arc::new(config))
        .connect(server_name, tcp)
        .await
        .map_err(|e| format!("tls handshake: {e}"))?;

    let io = hyper_util::rt::TokioIo::new(tls);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .map_err(|e| format!("http handshake: {e}"))?;
    tokio::spawn(conn);

    let request = Request::post(path)
        .header(HOST, host)
        .header(CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .map_err(|e| format!("build request: {e}"))?;

    let response = sender
        .send_request(request)
        .await
        .map_err(|e| format!("send request: {e}"))?;
    let status = response.status();

    // Drain the body so the connection shuts down cleanly.
    let _ = response.into_body().collect().await;

    Ok(status)
}

fn parse_https_url(url: &str) -> Result<(String, u16, String), String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| "only https:// URLs are supported".to_string())?;

    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (rest, "/".to_string()),
    };

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host.to_string(),
            port.parse::<u16>().map_err(|e| format!("invalid port: {e}"))?,
        ),
        None => (authority.to_string(), 443),
    };

    if host.is_empty() {
        return Err("empty host".into());
    }

    Ok((host, port, path))
}

#[cfg(test)]
mod tests {
    use super::parse_https_url;

    #[test]
    fn parses_discord_webhook_url() {
        let (host, port, path) =
            parse_https_url("https://discord.com/api/webhooks/123/token").unwrap();
        assert_eq!(host, "discord.com");
        assert_eq!(port, 443);
        assert_eq!(path, "/api/webhooks/123/token");
    }

    #[test]
    fn parses_custom_port() {
        let (host, port, path) = parse_https_url("https://example.org:8443").unwrap();
        assert_eq!(host, "example.org");
        assert_eq!(port, 8443);
        assert_eq!(path, "/");
    }

    #[test]
    fn rejects_plain_http() {
        assert!(parse_https_url("http://example.org/hook").is_err());
    }
}
