//! Start Chrome browser if not running

use anyhow::{anyhow, Result};
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio_tungstenite::connect_async;

use crate::config::WS_URL;

/// Check if we can connect to the WebSocket server
async fn is_connected() -> bool {
    match connect_async(WS_URL).await {
        Ok((_, _)) => true,
        Err(_) => false,
    }
}

/// Start Chrome and wait for connection
pub async fn run(url: Option<&str>, timeout_secs: u64) -> Result<()> {
    // First check if already connected
    if is_connected().await {
        println!("Chrome is already running and connected");
        return Ok(());
    }

    println!("Starting Chrome...");

    // Build the chrome command
    let mut cmd = Command::new("chrome");
    if let Some(url) = url {
        cmd.arg(url);
    }

    // Spawn Chrome in background (detached)
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| anyhow!("Failed to start Chrome: {}", e))?;

    // Wait for connection with timeout
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    println!("Waiting for Chrome extension to connect...");

    loop {
        if start.elapsed() > timeout {
            return Err(anyhow!(
                "Timeout waiting for Chrome extension to connect.\n\
                Make sure the Chrome Agent Bridge extension is installed and enabled.\n\
                You can load it from: about:debugging -> This Chrome -> Load Temporary Add-on"
            ));
        }

        if is_connected().await {
            println!("Connected to Chrome Agent Bridge");
            return Ok(());
        }

        sleep(Duration::from_millis(500)).await;
    }
}
