//! Synapse protobuf options
//!
//! This module contains the generated Rust types from synapse options protos.

/// Synapse protobuf options namespace
pub mod synapse {
    /// Storage layer options for entity/column/relation definitions
    #[allow(missing_docs)]
    pub mod storage {
        include!(concat!(env!("OUT_DIR"), "/synapse.storage.rs"));
    }

    /// Validation options for field constraints
    #[allow(missing_docs)]
    pub mod validate {
        include!(concat!(env!("OUT_DIR"), "/synapse.validate.rs"));
    }
}
