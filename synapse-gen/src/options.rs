//! Internal prost-generated option types.
//! Used by the parser to decode extensions; NOT part of public API.

#[allow(clippy::all)]
pub mod synapse {
    pub mod storage {
        include!(concat!(env!("OUT_DIR"), "/synapse.storage.rs"));
    }
    pub mod validate {
        include!(concat!(env!("OUT_DIR"), "/synapse.validate.rs"));
    }
    pub mod grpc {
        include!(concat!(env!("OUT_DIR"), "/synapse.grpc.rs"));
    }
    pub mod graphql {
        include!(concat!(env!("OUT_DIR"), "/synapse.graphql.rs"));
    }
}
