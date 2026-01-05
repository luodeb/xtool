//! TFTP core protocol implementation
//!
//! This module contains the core components of the TFTP protocol:
//! - `packet`: Packet serialization and deserialization
//! - `socket`: Socket abstraction layer
//! - `options`: Protocol options and parameters
//! - `window`: Windowed transfer management
//! - `convert`: Data conversion utilities

mod convert;
pub mod options;
mod packet;
mod socket;
mod window;

// Public core types
pub use convert::Convert;
pub use options::{OptionType, TransferOption};
pub use packet::{ErrorCode, Packet};
pub use socket::{ServerSocket, Socket};
pub use window::Window;
