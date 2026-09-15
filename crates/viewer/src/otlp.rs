//! Hash-neutral viewer-frame OTLP/JSON. Not stored on Simulation.

use bevy::prelude::Resource;
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Mutex, OnceLock, mpsc};
use std::time::Duration;

#[derive(Resource, Clone, Default)]
pub struct OtlpEndpoint(pub String);

pub fn metrics_url(endpoint: &str) -> String {
    let e = endpoint.trim().trim_end_matches('/');
    if e.is_empty() {
        return String::new();
    }
    let after_scheme = e.split("://").nth(1).unwrap_or(e);
    if after_scheme.contains('/') {
        e.to_string()
    } else {
        format!("{e}/v1/metrics")
    }
}

pub fn viewer_frame_metrics_json(last_ns: u64) -> String {
    json!({
        "resourceMetrics": [{
            "resource": {
                "attributes": [{
                    "key": "service.name",
                    "value": { "stringValue": "agentplace-viewer" }
                }]
            },
            "scopeMetrics": [{
                "metrics": [
                    {
                        "name": "agentplace.viewer.frame_ns",
                        "gauge": { "dataPoints": [{ "asInt": last_ns.to_string() }] }
                    }
                ]
            }]
        }]
    })
    .to_string()
}

static POSTER: OnceLock<Mutex<Option<(String, mpsc::SyncSender<u64>)>>> = OnceLock::new();

/// Best-effort: latest frame ns on a background thread. Never blocks the render loop.
pub fn offer_frame_ns(endpoint: &str, last_ns: u64) {
    let url = metrics_url(endpoint);
    if url.is_empty() {
        return;
    }
    let slot = POSTER.get_or_init(|| Mutex::new(None));
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    let need_new = guard.as_ref().is_none_or(|(u, _)| u != &url);
    if need_new {
        let (tx, rx) = mpsc::sync_channel::<u64>(1);
        let post_url = url.clone();
        std::thread::spawn(move || {
            while let Ok(ns) = rx.recv() {
                let body = viewer_frame_metrics_json(ns);
                let _ = post_json(&post_url, &body);
            }
        });
        *guard = Some((url, tx));
    }
    if let Some((_, tx)) = guard.as_ref() {
        let _ = tx.try_send(last_ns);
    }
}

fn post_json(url: &str, body: &str) -> std::io::Result<()> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| std::io::Error::other("otlp endpoint must be http://"))?;
    let (hostport, path) = rest.split_once('/').unwrap_or((rest, ""));
    let path = if path.is_empty() {
        "/".to_string()
    } else {
        format!("/{path}")
    };
    let mut stream = TcpStream::connect(hostport)?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {hostport}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes())?;
    let mut buf = [0u8; 256];
    let _ = stream.read(&mut buf);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewer_frame_json_has_locked_gauge() {
        let s = viewer_frame_metrics_json(16_666_667);
        assert!(s.contains("agentplace.viewer.frame_ns"), "{s}");
        assert!(s.contains("agentplace-viewer"), "{s}");
        assert!(s.contains("16666667"), "{s}");
    }

    #[test]
    fn metrics_url_appends_when_path_empty() {
        assert_eq!(
            metrics_url("http://127.0.0.1:4318"),
            "http://127.0.0.1:4318/v1/metrics"
        );
    }
}
