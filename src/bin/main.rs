// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! rust-lin CLI binary — RELAY spec §11 conformant command surface.

use std::io::BufRead;
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;

use rust_lin::relay::{Message, Protocol, SubscriberOptions};
use rust_lin::virtual_bus::VirtualBus;
use rust_lin::{Bus, Frame};

// ---------------------------------------------------------------------------
// CLI argument definitions
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "rust-lin",
    version = env!("CARGO_PKG_VERSION"),
    about = "rust-LIN: RELAY-conformant LIN bus tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Report tool and protocol version.
    Version {
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// Report supported capabilities as JSON.
    Capabilities,

    /// Report self-assessed health status.
    Status {
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// Publish a LIN slave response on the virtual bus.
    ///
    /// In text mode (default) publishes a single frame built from --id/--data.
    /// In json mode reads NDJSON relay.Message objects from stdin.
    Send {
        /// LIN frame ID (0–63 decimal or hex with 0x prefix).
        #[arg(long)]
        id: Option<String>,
        /// Frame data as hex string (e.g. 01020304). Optional when --format json.
        #[arg(long)]
        data: Option<String>,
        /// Input/output format.
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// Subscribe to LIN frames on the virtual bus.
    Subscribe {
        /// Stop after receiving N frames (0 = unlimited).
        #[arg(long, default_value = "0")]
        count: usize,
        /// Output format.
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// Convert a lin.Frame JSON from stdin to relay.Message JSON on stdout.
    ///
    /// Exit codes: 0 = converted, 1 = invalid input, 2 = invalid args.
    Convert {
        /// Protocol identifier; must be LIN for this tool.
        #[arg(long, default_value = "LIN")]
        protocol: String,
        /// Output format.
        #[arg(long, value_enum, default_value = "json")]
        format: OutputFormat,
    },
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let exit_code = match run(cli).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("rust-lin: error: {}", e);
            1
        }
    };

    std::process::exit(exit_code);
}

async fn run(cli: Cli) -> Result<i32, Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Version { format } => cmd_version(format),
        Commands::Capabilities => cmd_capabilities(),
        Commands::Status { format } => cmd_status(format),
        Commands::Send { id, data, format } => cmd_send(id, data, format).await,
        Commands::Subscribe { count, format } => cmd_subscribe(count, format).await,
        Commands::Convert { protocol, format } => cmd_convert(protocol, format),
    }
}

// ---------------------------------------------------------------------------
// version
// ---------------------------------------------------------------------------

fn cmd_version(format: OutputFormat) -> Result<i32, Box<dyn std::error::Error>> {
    let doc = json!({
        "tool":         "rust-lin",
        "protocol":     "LIN",
        "protocol_int": Protocol::Lin as i32,
        "version":      env!("CARGO_PKG_VERSION"),
        "spec_version": rust_lin::SPEC_VERSION,
        "language":     "rust",
        "runtime":      "rustc 1.75+",
    });

    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&doc)?),
        OutputFormat::Text => {
            println!("tool:         {}", doc["tool"].as_str().unwrap_or(""));
            println!("protocol:     {}", doc["protocol"].as_str().unwrap_or(""));
            println!("version:      {}", doc["version"].as_str().unwrap_or(""));
            println!(
                "spec_version: {}",
                doc["spec_version"].as_str().unwrap_or("")
            );
            println!("language:     {}", doc["language"].as_str().unwrap_or(""));
            println!("runtime:      {}", doc["runtime"].as_str().unwrap_or(""));
        }
    }

    Ok(0)
}

// ---------------------------------------------------------------------------
// capabilities
// ---------------------------------------------------------------------------

fn cmd_capabilities() -> Result<i32, Box<dyn std::error::Error>> {
    let doc = json!({
        "kind":                "capabilities",
        "tool":                "rust-lin",
        "protocol":            "LIN",
        "protocol_int":        Protocol::Lin as i32,
        "version":             env!("CARGO_PKG_VERSION"),
        "spec_version":        rust_lin::SPEC_VERSION,
        "commands":            ["version", "capabilities", "status", "send", "subscribe", "convert"],
        "transports":          ["virtual"],
        "features":            ["master", "slave", "schedule", "checksum", "pid"],
        "interfaces":          ["Bus", "MasterBus"],
        "optional_interfaces": ["HealthProvider", "MetricsProvider"],
        "adapt":               true,
    });

    println!("{}", serde_json::to_string_pretty(&doc)?);
    Ok(0)
}

// ---------------------------------------------------------------------------
// status
// ---------------------------------------------------------------------------

fn cmd_status(format: OutputFormat) -> Result<i32, Box<dyn std::error::Error>> {
    let doc = json!({
        "protocol":  "LIN",
        "tool":      "rust-lin",
        "version":   env!("CARGO_PKG_VERSION"),
        "healthy":   true,
        "connected": false,
        "endpoint":  "",
        "details":   {},
    });

    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&doc)?),
        OutputFormat::Text => {
            println!("tool:      {}", doc["tool"].as_str().unwrap_or(""));
            println!("protocol:  {}", doc["protocol"].as_str().unwrap_or(""));
            println!("version:   {}", doc["version"].as_str().unwrap_or(""));
            println!("healthy:   {}", doc["healthy"]);
            println!("connected: {}", doc["connected"]);
        }
    }

    Ok(0)
}

// ---------------------------------------------------------------------------
// send
// ---------------------------------------------------------------------------

async fn cmd_send(
    id_str: Option<String>,
    data_hex: Option<String>,
    format: OutputFormat,
) -> Result<i32, Box<dyn std::error::Error>> {
    let bus = Arc::new(VirtualBus::new());

    match format {
        OutputFormat::Json => cmd_send_json(bus).await,
        OutputFormat::Text => {
            let id_str = id_str.ok_or("rust-lin: --id is required in text mode")?;
            let data_hex = data_hex.ok_or("rust-lin: --data is required in text mode")?;

            let id: u8 = if id_str.starts_with("0x") || id_str.starts_with("0X") {
                u8::from_str_radix(&id_str[2..], 16)?
            } else {
                id_str.parse()?
            };

            let data = hex::decode(data_hex.replace(' ', ""))?;

            if data.len() > rust_lin::LIN_MAX_DATA_LEN {
                return Err(format!(
                    "rust-lin: data length {} exceeds maximum {}",
                    data.len(),
                    rust_lin::LIN_MAX_DATA_LEN
                )
                .into());
            }

            bus.publish(id, Some(data.clone())).await?;

            println!("published: id=0x{:02X} data={}", id, hex::encode(&data));

            Ok(0)
        }
    }
}

async fn cmd_send_json(bus: Arc<VirtualBus>) -> Result<i32, Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let mut sent = 0usize;
    let mut errors = 0usize;

    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let msg: Message = match serde_json::from_str(line) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("rust-lin: send --format json: parse error: {}", e);
                errors += 1;
                continue;
            }
        };

        let frame = match rust_lin::from_message(&msg) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("rust-lin: send --format json: conversion error: {}", e);
                errors += 1;
                continue;
            }
        };

        if let Err(e) = rust_lin::validate_frame(&frame) {
            eprintln!("rust-lin: send --format json: invalid frame: {}", e);
            errors += 1;
            continue;
        }

        bus.publish(frame.id, Some(frame.data)).await?;
        sent += 1;
    }

    eprintln!(
        "rust-lin: send --format json: sent={} errors={}",
        sent, errors
    );
    Ok(0)
}

// ---------------------------------------------------------------------------
// subscribe
// ---------------------------------------------------------------------------

async fn cmd_subscribe(
    count: usize,
    format: OutputFormat,
) -> Result<i32, Box<dyn std::error::Error>> {
    let bus = Arc::new(VirtualBus::new());
    let rx = bus.subscribe(vec![], SubscriberOptions::default()).await?;

    eprintln!(
        "rust-lin: subscribing on virtual bus ({})",
        if count == 0 {
            "unlimited".to_string()
        } else {
            format!("{} frames", count)
        }
    );

    let mut received = 0usize;
    loop {
        if count > 0 && received >= count {
            break;
        }

        match rx.recv().await {
            None => break,
            Some(frame) => {
                received += 1;
                let msg = rust_lin::to_message(&frame);

                match format {
                    OutputFormat::Json => {
                        let doc = json!({
                            "protocol": "LIN",
                            "id":       msg.id,
                            "data":     hex::encode(&frame.data),
                            "checksum": frame.checksum,
                            "checksum_type": frame.checksum_type.to_string(),
                            "seq":      received,
                        });
                        println!("{}", serde_json::to_string(&doc)?);
                    }
                    OutputFormat::Text => {
                        println!(
                            "[{}] id=0x{:02X} checksum=0x{:02X} ct={} data={}",
                            received,
                            frame.id,
                            frame.checksum,
                            frame.checksum_type,
                            hex::encode(&frame.data)
                        );
                    }
                }
            }
        }
    }

    bus.close().await?;
    Ok(0)
}

// ---------------------------------------------------------------------------
// convert  (RELAY spec §11.2)
// ---------------------------------------------------------------------------

fn cmd_convert(protocol: String, _format: OutputFormat) -> Result<i32, Box<dyn std::error::Error>> {
    if !protocol.eq_ignore_ascii_case("LIN") {
        eprintln!(
            "rust-lin: convert: unsupported protocol '{}'; this tool implements LIN",
            protocol
        );
        // Exit 2 = invalid args per spec §11.2.
        return Ok(2);
    }

    // Frame derives Deserialize — parse stdin directly.
    let frame: Frame = match serde_json::from_reader(std::io::stdin().lock()) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}", e);
            eprintln!("INVALID_ARGUMENT");
            return Ok(1);
        }
    };

    if let Err(e) = rust_lin::validate_frame(&frame) {
        eprintln!("{}", e);
        eprintln!("INVALID_ARGUMENT");
        return Ok(1);
    }

    let mut msg = rust_lin::to_message(&frame);
    // Zero the timestamp per spec §11.2.
    msg.timestamp = chrono::DateTime::UNIX_EPOCH;

    println!("{}", serde_json::to_string_pretty(&msg)?);
    Ok(0)
}
