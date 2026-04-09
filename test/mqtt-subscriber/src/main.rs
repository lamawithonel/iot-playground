//! Minimal MQTT subscriber for smoke test validation.
//!
//! Connects to the test broker over TLS, subscribes to the
//! telemetry topic, and writes received message payloads to a
//! file (one per line).  Exits on SIGTERM, SIGINT, or after
//! the configured timeout.
//!
//! # Usage
//!
//! ```text
//! mqtt-subscriber <output-file> [duration-secs]
//! ```
//!
//! # Environment
//!
//! - `BROKER_HOST_IP` — Broker IP address (required)
//! - `BROKER_PORT` — Broker port (default: 8883)
//! - `MQTT_CA_FILE` — CA certificate PEM file
//!   (default: `.local/certs/ca/root.crt`)
//! - `MQTT_TOPIC` — Subscribe topic
//!   (default: `device/+/telemetry`)

use std::io::Write;

use rumqttc::{
    AsyncClient, Event, Incoming, MqttOptions, QoS,
    TlsConfiguration, Transport,
};
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::{sleep, Duration};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let output_path = match args.get(1) {
        Some(p) => p.clone(),
        None => {
            eprintln!(
                "Usage: mqtt-subscriber <output-file> \
                 [duration-secs]"
            );
            std::process::exit(2);
        }
    };

    let duration_secs: u64 = args
        .get(2)
        .map(|s| {
            s.parse()
                .expect("duration must be a positive integer")
        })
        .unwrap_or(120);

    let host = std::env::var("BROKER_HOST_IP").unwrap_or_else(
        |_| {
            eprintln!("ERROR: BROKER_HOST_IP must be set");
            std::process::exit(1);
        },
    );

    let port: u16 = std::env::var("BROKER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8883);

    let ca_file = std::env::var("MQTT_CA_FILE")
        .unwrap_or_else(|_| {
            ".local/certs/ca/root.crt".to_string()
        });

    let topic = std::env::var("MQTT_TOPIC")
        .unwrap_or_else(|_| "device/+/telemetry".to_string());

    let ca = match std::fs::read(&ca_file) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!(
                "ERROR: failed to read CA cert {ca_file}: {e}"
            );
            std::process::exit(1);
        }
    };

    let mut opts =
        MqttOptions::new("smoke-test-sub", &host, port);
    opts.set_transport(Transport::Tls(
        TlsConfiguration::Simple {
            ca,
            alpn: None,
            client_auth: None,
        },
    ));
    opts.set_keep_alive(Duration::from_secs(30));

    let (client, mut eventloop) =
        AsyncClient::new(opts, 100);

    if let Err(e) =
        client.subscribe(&topic, QoS::AtLeastOnce).await
    {
        eprintln!("ERROR: subscribe failed: {e}");
        std::process::exit(1);
    }

    let file = match std::fs::File::create(&output_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "ERROR: cannot create {output_path}: {e}"
            );
            std::process::exit(1);
        }
    };
    let mut writer = std::io::BufWriter::new(file);
    let mut count: u64 = 0;

    let mut sigterm =
        signal(SignalKind::terminate()).expect("SIGTERM handler");
    let mut sigint =
        signal(SignalKind::interrupt()).expect("SIGINT handler");

    let timeout = sleep(Duration::from_secs(duration_secs));
    tokio::pin!(timeout);

    eprintln!(
        "Subscribing to {topic} at {host}:{port} (TLS)..."
    );

    loop {
        tokio::select! {
            event = eventloop.poll() => {
                match event {
                    Ok(Event::Incoming(
                        Incoming::Publish(msg),
                    )) => {
                        if let Ok(payload) =
                            std::str::from_utf8(&msg.payload)
                        {
                            let _ =
                                writeln!(writer, "{payload}");
                            let _ = writer.flush();
                            count += 1;
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("MQTT error: {e}");
                        break;
                    }
                }
            }
            _ = &mut timeout => {
                break;
            }
            _ = sigterm.recv() => {
                break;
            }
            _ = sigint.recv() => {
                break;
            }
        }
    }

    client.disconnect().await.ok();
    drop(writer);

    // Print count on last line of stderr for script parsing.
    eprintln!("{count}");
}
