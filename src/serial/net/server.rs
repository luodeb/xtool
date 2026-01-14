use anyhow::{Result, Context};
use crate::serial::config::SerialConfig;
use log::{info, error}; // Removed warn
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::SerialPortBuilderExt;
// Removed std::sync::Arc

pub async fn run(uart: Option<String>, baud: Option<u32>, port: Option<u16>, bind: Option<String>, config: Option<SerialConfig>) -> Result<()> {
    // Resolve UART and Baud
    let final_uart = uart.or(config.as_ref().and_then(|c| c.uart.clone()));
    let final_baud = baud.or(config.as_ref().and_then(|c| c.baud)).unwrap_or(115200);

    // Resolve Port and Bind IP
    let final_port = port.or(config.as_ref().and_then(|c| c.net_port)).unwrap_or(5432);
    let final_bind = bind.or(config.as_ref().and_then(|c| c.net_bind.clone())).unwrap_or_else(|| "0.0.0.0".to_string());

    let uart_name = final_uart.ok_or_else(|| anyhow::anyhow!("Serial port not specified. Please use UART argument or config file."))?;

    info!("Starting Netd: Serial <-> TCP Server (Multi-client broadcast)");
    info!("Serial Port: {}, Baud: {}", uart_name, final_baud);

    // Open Serial Port
    let mut serial_stream = tokio_serial::new(&uart_name, final_baud)
        .open_native_async()
        .with_context(|| format!("Failed to open serial port {}", uart_name))?;

    #[cfg(unix)]
    {
        // use tokio_serial::SerialPort as _; // Ensure trait is used
        // serial_stream.set_exclusive(false).ok(); 
        
        // Try importing trait directly if "use ... as _" triggers warning
        #[allow(unused)]
        use tokio_serial::SerialPort;
        serial_stream.set_exclusive(false).ok(); 
    }

    // Split serial stream
    let (mut serial_reader, mut serial_writer) = tokio::io::split(serial_stream);

    // Channels
    // 1. Broadcast channel for Serial -> Clients (Many subscribers)
    let (broadcast_tx, _) = broadcast::channel::<Vec<u8>>(1024);
    
    // 2. MPSC channel for Clients -> Serial (Many producers, single consumer)
    let (mpsc_tx, mut mpsc_rx) = mpsc::channel::<Vec<u8>>(1024);


    // Task 1: Serial Reader -> Broadcast
    let b_tx = broadcast_tx.clone();
    tokio::spawn(async move {
        let mut buf = [0u8; 1024];
        loop {
            match serial_reader.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    let data = buf[..n].to_vec();
                    // Send to all connected clients. Ignore error if no listeners.
                    let _ = b_tx.send(data);
                }
                Ok(_) => {
                     error!("Serial port closed (EOF).");
                     break;
                }
                Err(e) => {
                    error!("Error reading from serial: {}", e);
                    break;
                }
            }
        }
    });

    // Task 2: MPSC -> Serial Writer
    tokio::spawn(async move {
        while let Some(data) = mpsc_rx.recv().await {
            if let Err(e) = serial_writer.write_all(&data).await {
                error!("Failed to write to serial port: {}", e);
                break;
            }
            let _ = serial_writer.flush().await;
        }
    });

    // Task 3: TCP Listener
    let addr = format!("{}:{}", final_bind, final_port);
    let listener = TcpListener::bind(&addr).await.with_context(|| format!("Failed to bind to {}", addr))?;
    
    info!("Listening on {}", addr);
    info!("Ready to accept connections...");

    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {
                info!("Client connected from {}", peer_addr);
                
                let client_b_rx = broadcast_tx.subscribe();
                let client_m_tx = mpsc_tx.clone();
                
                tokio::spawn(async move {
                    handle_client(socket, client_b_rx, client_m_tx, peer_addr).await;
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_client(
    socket: tokio::net::TcpStream, 
    mut broadcast_rx: broadcast::Receiver<Vec<u8>>, 
    mpsc_tx: mpsc::Sender<Vec<u8>>,
    peer_addr: std::net::SocketAddr
) {
    let (mut socket_read, mut socket_write) = socket.into_split();
    
    // Client specific tasks container
    let mut handle_read = tokio::task::spawn(async move {
        let mut buf = [0u8; 1024];
        loop {
            match socket_read.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    let data = buf[..n].to_vec();
                    if mpsc_tx.send(data).await.is_err() {
                        break; // Serial writer task died?
                    }
                }
                Ok(_) => break, // EOF
                Err(_) => break, // Error
            }
        }
    });

    let mut handle_write = tokio::task::spawn(async move {
        while let Ok(data) = broadcast_rx.recv().await {
            if socket_write.write_all(&data).await.is_err() {
                break;
            }
        }
    });

    // Wait for either direction to fail/finish
    tokio::select! {
        _ = &mut handle_read => {
            // Read loop finished (client disconnect)
        }
        _ = &mut handle_write => {
            // Write loop finished
        }
    }
    
    // Cleanup
    handle_read.abort();
    handle_write.abort();
    info!("Client disconnected: {}", peer_addr);
}
