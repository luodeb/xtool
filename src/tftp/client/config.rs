use std::net::IpAddr;
use std::time::Duration;

/// TFTP client configuration
///
/// # Example
///
/// ```rust
/// use xtool::tftp::client::ClientConfig;
///
/// let config = ClientConfig::new("192.168.1.100".parse().unwrap(), 69);
/// ```
pub struct ClientConfig {
    /// Server IP address
    pub server_ip: IpAddr,
    /// Server port number
    pub server_port: u16,
    /// Block size (default 512, negotiable)
    pub block_size: u16,
    /// Timeout duration
    pub timeout: Duration,
    /// Window size (RFC 7440)
    pub window_size: u16,
    /// Transfer mode (currently only supports octet)
    pub mode: String,
}

impl ClientConfig {
    /// Create new client configuration
    ///
    /// # Arguments
    ///
    /// * `server_ip` - Server IP address
    /// * `server_port` - Server port number (usually 69)
    pub fn new(server_ip: IpAddr, server_port: u16) -> Self {
        Self {
            server_ip,
            server_port,
            block_size: 512,
            timeout: Duration::from_secs(5),
            window_size: 1,
            mode: "octet".to_string(),
        }
    }

    /// Set block size
    pub fn with_block_size(mut self, block_size: u16) -> Self {
        self.block_size = block_size;
        self
    }

    /// Set timeout duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::new("127.0.0.1".parse().unwrap(), 69)
    }
}
