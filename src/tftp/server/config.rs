use std::net::IpAddr;
use std::path::PathBuf;

use crate::tftp::core::options::OptionsPrivate;

/// TFTP server configuration
///
/// Provides a simplified configuration interface for the xtool project
///
/// # Example
///
/// ```rust
/// use xtool::tftp::server::Config;
/// use std::path::PathBuf;
///
/// let config = Config::new(
///     "127.0.0.1".parse().unwrap(),
///     69,
///     PathBuf::from("/tmp/tftp"),
///     false,
/// );
/// ```
pub struct Config {
    /// IP address to listen on
    pub ip_address: IpAddr,
    /// Port number to listen on
    pub port: u16,
    /// Directory for uploaded files (defaults to same as directory)
    pub receive_directory: PathBuf,
    /// Directory for downloaded files (defaults to same as directory)
    pub send_directory: PathBuf,
    /// Whether to use single port mode (for NAT environments)
    pub single_port: bool,
    /// Whether to use read-only mode (reject all write requests)
    pub read_only: bool,
    /// Whether to overwrite existing files
    pub overwrite: bool,
    /// Internal options (retries, timeouts, etc.)
    pub opt_local: OptionsPrivate,
}

impl Config {
    /// Create a new configuration
    ///
    /// # Arguments
    ///
    /// * `ip_address` - IP address to listen on
    /// * `port` - Port number to listen on
    /// * `directory` - Root directory for files
    /// * `read_only` - Whether to use read-only mode
    pub fn new(ip_address: IpAddr, port: u16, directory: PathBuf, read_only: bool) -> Self {
        let receive_directory = directory.clone();
        let send_directory = directory;

        Self {
            ip_address,
            port,
            receive_directory,
            send_directory,
            single_port: false,
            read_only,
            overwrite: true, // Allow overwrite by default
            opt_local: OptionsPrivate::default(),
        }
    }

    /// Set whether to use single port mode
    pub fn with_single_port(mut self, single_port: bool) -> Self {
        self.single_port = single_port;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        use std::net::Ipv4Addr;

        Self::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            69,
            std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir()),
            false,
        )
    }
}
