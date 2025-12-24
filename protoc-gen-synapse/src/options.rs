//! Synapse storage options
//!
//! This module contains the generated Rust types from synapse/storage/options.proto

pub mod synapse {
    pub mod storage {
        include!(concat!(env!("OUT_DIR"), "/synapse.storage.rs"));
    }
}
