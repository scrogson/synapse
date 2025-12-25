//! gRPC/Tonic service generation
//!
//! This module generates Tonic gRPC service implementations from protobuf
//! service definitions. The generated services are backend-agnostic and
//! delegate to storage traits.

mod errors;
mod service;

pub use service::generate;
