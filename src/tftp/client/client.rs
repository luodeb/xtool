use std::fs::File;
use std::io::Write;
use std::net::{SocketAddr, UdpSocket};
use std::path::Path;

use super::config::ClientConfig;
use crate::tftp::core::options::{OptionsProtocol, RequestType};
use crate::tftp::core::{Packet, TransferOption, Window};

/// TFTP client
///
/// Supports file upload (PUT) and download (GET) operations
///
/// # Example
///
/// ```rust,no_run
/// use xtool::tftp::client::{Client, ClientConfig};
/// use std::path::Path;
///
/// let config = ClientConfig::new("192.168.1.100".parse().unwrap(), 69);
/// let client = Client::new(config).unwrap();
///
/// // Download file
/// client.get("remote.txt", Path::new("local.txt")).unwrap();
///
/// // Upload file
/// client.put(Path::new("local.txt"), "remote.txt").unwrap();
/// ```
pub struct Client {
    config: ClientConfig,
}

impl Client {
    /// Create a new TFTP client
    pub fn new(config: ClientConfig) -> anyhow::Result<Self> {
        Ok(Self { config })
    }

    /// Download a file from the server (RRQ - Read Request)
    ///
    /// # Arguments
    ///
    /// * `remote_file` - File name on the server
    /// * `local_file` - Local save path
    pub fn get(&self, remote_file: &str, local_file: &Path) -> anyhow::Result<()> {
        log::info!("Downloading {} to {}", remote_file, local_file.display());

        // Create local socket
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let server_addr = SocketAddr::new(self.config.server_ip, self.config.server_port);
        // Don't use connect, use send_to instead
        socket.set_read_timeout(Some(self.config.timeout))?;
        socket.set_write_timeout(Some(self.config.timeout))?;

        // Prepare options
        let mut options = vec![
            TransferOption {
                option: crate::tftp::core::OptionType::BlockSize,
                value: self.config.block_size as u64,
            },
            TransferOption {
                option: crate::tftp::core::OptionType::WindowSize,
                value: self.config.window_size as u64,
            },
            TransferOption {
                option: crate::tftp::core::OptionType::Timeout,
                value: self.config.timeout.as_secs(),
            },
            TransferOption {
                option: crate::tftp::core::OptionType::TransferSize,
                value: 0, // Request server to provide file size
            },
        ];

        // Send RRQ
        let rrq = Packet::Rrq {
            filename: remote_file.to_string(),
            mode: self.config.mode.clone(),
            options: options.clone(),
        };
        socket.send_to(&rrq.serialize()?, &server_addr)?;

        // Wait for response (OACK or first data packet)
        let mut buf = vec![0u8; 65536];
        let (amt, new_addr) = socket.recv_from(&mut buf)?;
        let response = Packet::deserialize(&buf[..amt])?;

        // Reconnect to the server's new port (TFTP server creates a new port for each transfer)
        if new_addr != server_addr {
            socket.connect(new_addr)?;
        } else {
            socket.connect(server_addr)?;
        }

        let worker_options = match response {
            Packet::Oack(ref opts) => {
                options = opts.clone();
                let opts = OptionsProtocol::parse(&mut options, RequestType::Read(0))?;

                // Send ACK 0 to confirm options
                let ack = Packet::Ack(0);
                socket.send(&ack.serialize()?)?;

                opts
            }
            Packet::Data { .. } => OptionsProtocol::default(),
            Packet::Error { code, msg } => {
                return Err(anyhow::anyhow!("Server error {}: {}", code, msg));
            }
            _ => {
                return Err(anyhow::anyhow!("Unexpected packet type"));
            }
        };

        // Receive file
        let file = File::create(local_file)?;

        // If received OACK, wait for first DATA packet; otherwise first packet is DATA
        let first_data_packet = if matches!(response, Packet::Oack(_)) {
            let (amt, _) = socket.recv_from(&mut buf)?;
            Packet::deserialize(&buf[..amt])?
        } else {
            response
        };

        self.receive_file(socket, file, worker_options, first_data_packet)?;

        log::info!("Download complete: {}", local_file.display());
        Ok(())
    }

    /// Upload a file to the server (WRQ - Write Request)
    ///
    /// # Arguments
    ///
    /// * `local_file` - Local file path
    /// * `remote_file` - File name on the server
    pub fn put(&self, local_file: &Path, remote_file: &str) -> anyhow::Result<()> {
        log::info!("Uploading {} to {}", local_file.display(), remote_file);

        if !local_file.exists() {
            return Err(anyhow::anyhow!("Local file does not exist"));
        }

        let file_size = local_file.metadata()?.len();

        // Create local socket
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let server_addr = SocketAddr::new(self.config.server_ip, self.config.server_port);
        // Don't use connect, use send_to instead
        socket.set_read_timeout(Some(self.config.timeout))?;
        socket.set_write_timeout(Some(self.config.timeout))?;

        // Prepare options
        let mut options = vec![
            TransferOption {
                option: crate::tftp::core::OptionType::BlockSize,
                value: self.config.block_size as u64,
            },
            TransferOption {
                option: crate::tftp::core::OptionType::WindowSize,
                value: self.config.window_size as u64,
            },
            TransferOption {
                option: crate::tftp::core::OptionType::Timeout,
                value: self.config.timeout.as_secs(),
            },
            TransferOption {
                option: crate::tftp::core::OptionType::TransferSize,
                value: file_size,
            },
        ];

        // Send WRQ
        let wrq = Packet::Wrq {
            filename: remote_file.to_string(),
            mode: self.config.mode.clone(),
            options: options.clone(),
        };
        socket.send_to(&wrq.serialize()?, &server_addr)?;

        // Wait for response (OACK or ACK 0)
        let mut buf = vec![0u8; 65536];
        let (amt, new_addr) = socket.recv_from(&mut buf)?;
        let response = Packet::deserialize(&buf[..amt])?;

        // Reconnect to the server's new port (TFTP server creates a new port for each transfer)
        if new_addr != server_addr {
            socket.connect(new_addr)?;
        } else {
            socket.connect(server_addr)?;
        }

        let worker_options = match response {
            Packet::Oack(ref opts) => {
                options = opts.clone();
                OptionsProtocol::parse(&mut options, RequestType::Write)?
            }
            Packet::Ack(0) => OptionsProtocol::default(),
            Packet::Error { code, msg } => {
                return Err(anyhow::anyhow!("Server error {}: {}", code, msg));
            }
            _ => {
                return Err(anyhow::anyhow!("Unexpected packet type"));
            }
        };

        // Send file
        let file = File::open(local_file)?;
        self.send_file(socket, file, worker_options)?;

        log::info!("Upload complete: {}", remote_file);
        Ok(())
    }

    /// Receive file data
    fn receive_file(
        &self,
        socket: UdpSocket,
        mut file: File,
        options: OptionsProtocol,
        first_packet: Packet,
    ) -> anyhow::Result<()> {
        let mut expected_block: u16 = 1;
        let mut total_bytes = 0u64;

        // Process first packet (if it's DATA)
        if let Packet::Data { block_num, data } = first_packet {
            if block_num == 1 {
                file.write_all(&data)?;
                total_bytes += data.len() as u64;

                // Send ACK
                let ack = Packet::Ack(block_num);
                socket.send(&ack.serialize()?)?;

                expected_block = 2;

                // If data is less than block size, transfer is complete
                if data.len() < options.block_size as usize {
                    log::debug!("Transfer complete. Total bytes: {}", total_bytes);
                    return Ok(());
                }
            }
        }

        // Continue receiving subsequent data packets
        let mut buf = vec![0u8; 65536];
        loop {
            let (amt, _) = socket.recv_from(&mut buf)?;
            let packet = Packet::deserialize(&buf[..amt])?;

            match packet {
                Packet::Data { block_num, data } => {
                    if block_num == expected_block {
                        file.write_all(&data)?;
                        total_bytes += data.len() as u64;

                        // Send ACK
                        let ack = Packet::Ack(block_num);
                        socket.send(&ack.serialize()?)?;

                        // If data is less than block size, transfer is complete
                        if data.len() < options.block_size as usize {
                            log::debug!("Transfer complete. Total bytes: {}", total_bytes);
                            break;
                        }

                        expected_block = expected_block.wrapping_add(1);
                    } else {
                        log::warn!(
                            "Received unexpected block {}, expected {}",
                            block_num,
                            expected_block
                        );
                        // Resend previous ACK
                        let ack = Packet::Ack(expected_block.wrapping_sub(1));
                        socket.send(&ack.serialize()?)?;
                    }
                }
                Packet::Error { code, msg } => {
                    return Err(anyhow::anyhow!("Server error {}: {}", code, msg));
                }
                _ => {
                    log::warn!("Received unexpected packet type");
                }
            }
        }

        Ok(())
    }

    /// Send file data
    fn send_file(
        &self,
        socket: UdpSocket,
        file: File,
        options: OptionsProtocol,
    ) -> anyhow::Result<()> {
        let mut window = Window::new(options.window_size, options.block_size, file);
        let mut block_num: u16 = 1;
        let mut total_bytes = 0u64;

        loop {
            // Fill window
            let more = window.fill()?;

            // Send all packets in window
            for data in window.get_elements() {
                let packet = Packet::Data {
                    block_num,
                    data: data.clone(),
                };
                socket.send(&packet.serialize()?)?;
                total_bytes += data.len() as u64;
                block_num = block_num.wrapping_add(1);
            }

            // If no more data, wait for final ACK and exit
            if !more && window.get_elements().is_empty() {
                break;
            }

            // Wait for ACK
            let mut buf = vec![0u8; 65536];
            let (amt, _) = socket.recv_from(&mut buf)?;
            let packet = Packet::deserialize(&buf[..amt])?;

            match packet {
                Packet::Ack(ack_block) => {
                    log::debug!("Received ACK for block {}", ack_block);
                    // Clear window, prepare for next batch
                    window.clear();
                }
                Packet::Error { code, msg } => {
                    return Err(anyhow::anyhow!("Server error {}: {}", code, msg));
                }
                _ => {
                    log::warn!("Received unexpected packet type");
                }
            }

            if !more {
                break;
            }
        }

        log::debug!("Transfer complete. Total bytes: {}", total_bytes);
        Ok(())
    }
}
