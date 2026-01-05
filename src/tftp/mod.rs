//! TFTP (Trivial File Transfer Protocol) implementation
//!
//! This module implements the complete TFTP protocol, based on the following RFC standards:
//! - [RFC 1350](https://www.rfc-editor.org/rfc/rfc1350) TFTP Protocol version 2
//! - [RFC 2347](https://www.rfc-editor.org/rfc/rfc2347) TFTP Option Extension
//! - [RFC 2348](https://www.rfc-editor.org/rfc/rfc2348) Blocksize Option
//! - [RFC 2349](https://www.rfc-editor.org/rfc/rfc2349) Timeout and Transfer Size Options
//! - [RFC 7440](https://www.rfc-editor.org/rfc/rfc7440) Windowsize Option
//!
//! ## Module Structure
//!
//! ```text
//! tftp/
//! ├── core/           # Core protocol implementation
//! │   ├── packet      # Packet serialization/deserialization
//! │   ├── socket      # Socket abstraction layer
//! │   ├── options     # Protocol options
//! │   ├── window      # Windowed transfer
//! │   └── convert     # Data conversion utilities
//! │
//! ├── server/         # TFTP server
//! │   ├── server      # Main server logic
//! │   ├── worker      # Transfer worker threads
//! │   └── config      # Server configuration
//! │
//! └── client/         # TFTP client
//!     └── ...
//! ```
//!
//! ## Usage Examples
//!
//! ### Start TFTP Server
//!
//! ```rust,no_run
//! use xtool::tftp::{server::Config, server::Server};
//! use std::path::PathBuf;
//!
//! let config = Config::with_defaults().merge_cli(
//!     "0.0.0.0".to_string(),
//!     69,
//!     PathBuf::from("/var/tftp"),
//!     false,
//!     false,
//! );
//!
//! let mut server = Server::new(&config).unwrap();
//! server.listen();
//! ```

// Submodules
pub mod client;
pub mod core;
pub mod server;

// Re-export commonly used types for convenience
