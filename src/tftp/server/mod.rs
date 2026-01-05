//! TFTP server implementation
//!
//! This module provides complete TFTP server functionality:
//! - `server`: Main server logic, handles client requests
//! - `worker`: Worker threads, handles file transfers
//! - `config`: Server configuration

mod config;
mod server;
mod worker;

use anyhow::Result;
use std::path::PathBuf;

// Public server types
pub use config::Config;
pub use server::Server;
pub use worker::Worker;

/// Run the TFTP server
pub fn run(ip: String, port: u16, path: PathBuf, read_only: bool, single_port: bool) -> Result<()> {
    log::info!("Starting TFTP server on {}:{}", ip, port);
    log::info!("Root directory: {}", path.display());
    log::info!("Read-only mode: {}", read_only);
    log::info!("Single port mode: {}", single_port);

    // Ensure directory exists
    if !path.exists() {
        log::error!("Directory does not exist: {}", path.display());
        return Err(anyhow::anyhow!("Directory does not exist"));
    }

    let ip_addr = ip
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid IP address '{}': {}", ip, e))?;

    let config = Config::new(ip_addr, port, path, read_only).with_single_port(single_port);

    let mut server = Server::new(&config)?;

    log::info!("TFTP server listening, press Ctrl+C to stop");
    server.listen();

    Ok(())
}
