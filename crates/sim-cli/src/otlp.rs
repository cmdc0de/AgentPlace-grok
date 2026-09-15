//! Hash-neutral OTLP/JSON POST from sim-cli. Not stored on Simulation.

use sim_core::Simulation;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

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

pub fn metrics_json(sim: &Simulation) -> String {
    let wall = sim
        .last_tick_timing
        .as_ref()
        .map(|t| t.wall_ns)
        .unwrap_or(0);
    let rss = sim.telemetry_rss_last.unwrap_or(0);
    let cpu_user = sim.telemetry_cpu_user_ns.unwrap_or(0);
    let cpu_system = sim.telemetry_cpu_system_ns.unwrap_or(0);
    let cpu_percent = sim.telemetry_cpu_percent.unwrap_or(0);
    let disk_read = sim.telemetry_disk_read_bytes.unwrap_or(0);
    let disk_write = sim.telemetry_disk_write_bytes.unwrap_or(0);
    let out_dir_bytes = sim.telemetry_out_dir_bytes.unwrap_or(0);
    serde_json::json!({
        "resourceMetrics": [{
            "resource": {
                "attributes": [{
                    "key": "service.name",
                    "value": { "stringValue": "agentplace-sim" }
                }]
            },
            "scopeMetrics": [{
                "metrics": [
                    {
                        "name": "agentplace.tick.wall_ns",
                        "gauge": { "dataPoints": [{ "asInt": wall.to_string() }] }
                    },
                    {
                        "name": "agentplace.process.rss_bytes",
                        "gauge": { "dataPoints": [{ "asInt": rss.to_string() }] }
                    },
                    {
                        "name": "agentplace.process.cpu_user_ns",
                        "gauge": { "dataPoints": [{ "asInt": cpu_user.to_string() }] }
                    },
                    {
                        "name": "agentplace.process.cpu_system_ns",
                        "gauge": { "dataPoints": [{ "asInt": cpu_system.to_string() }] }
                    },
                    {
                        "name": "agentplace.process.cpu_percent",
                        "gauge": { "dataPoints": [{ "asInt": cpu_percent.to_string() }] }
                    },
                    {
                        "name": "agentplace.process.disk_read_bytes",
                        "gauge": { "dataPoints": [{ "asInt": disk_read.to_string() }] }
                    },
                    {
                        "name": "agentplace.process.disk_write_bytes",
                        "gauge": { "dataPoints": [{ "asInt": disk_write.to_string() }] }
                    },
                    {
                        "name": "agentplace.process.out_dir_bytes",
                        "gauge": { "dataPoints": [{ "asInt": out_dir_bytes.to_string() }] }
                    }
                ]
            }]
        }]
    })
    .to_string()
}

pub fn post_tick_metrics(sim: &Simulation) {
    if sim.telemetry_otlp_endpoint.is_empty() {
        return;
    }
    let url = metrics_url(&sim.telemetry_otlp_endpoint);
    if url.is_empty() {
        return;
    }
    let body = metrics_json(sim);
    let _ = post_json(&url, &body);
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
    fn metrics_url_appends_when_path_empty() {
        assert_eq!(
            metrics_url("http://127.0.0.1:4318"),
            "http://127.0.0.1:4318/v1/metrics"
        );
        assert_eq!(
            metrics_url("http://127.0.0.1:4318/"),
            "http://127.0.0.1:4318/v1/metrics"
        );
        assert_eq!(
            metrics_url("http://127.0.0.1:4318/v1/metrics"),
            "http://127.0.0.1:4318/v1/metrics"
        );
    }
}
