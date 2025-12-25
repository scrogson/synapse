//! Error type generation for gRPC services
//!
//! This module provides token stream generators for error types used in
//! generated gRPC service implementations. The errors bridge between
//! storage layer errors and tonic::Status.

use proc_macro2::TokenStream;
use quote::quote;

/// Generate the ValidationError enum
///
/// This error type is used for request validation failures.
pub fn generate_validation_error() -> TokenStream {
    quote! {
        /// Validation error for request field failures
        #[derive(Debug, thiserror::Error)]
        pub enum ValidationError {
            /// A field failed validation
            #[error("Invalid field '{field}': {message}")]
            InvalidField {
                /// The field that failed validation
                field: String,
                /// Description of the validation failure
                message: String,
            },
            /// A required field was missing
            #[error("Missing required field: {0}")]
            MissingRequired(String),
        }
    }
}

/// Generate the ServiceError enum
///
/// This wraps ValidationError and StorageError for unified error handling
/// in gRPC service implementations.
pub fn generate_service_error() -> TokenStream {
    quote! {
        /// Service-level error wrapping validation and storage errors
        #[derive(Debug, thiserror::Error)]
        pub enum ServiceError {
            /// Request validation failed
            #[error("Validation failed: {0}")]
            Validation(#[from] ValidationError),
            /// Storage operation failed
            #[error("Storage error: {0}")]
            Storage(#[from] StorageError),
        }
    }
}

/// Generate the From<ServiceError> for tonic::Status implementation
///
/// This maps service errors to appropriate gRPC status codes.
pub fn generate_status_conversion() -> TokenStream {
    quote! {
        impl From<ServiceError> for tonic::Status {
            fn from(e: ServiceError) -> Self {
                match e {
                    ServiceError::Validation(v) => {
                        tonic::Status::invalid_argument(v.to_string())
                    }
                    ServiceError::Storage(s) => match s {
                        StorageError::NotFound(msg) => tonic::Status::not_found(msg),
                        StorageError::Database(db_err) => {
                            tonic::Status::internal(db_err.to_string())
                        }
                        StorageError::InvalidArgument(msg) => {
                            tonic::Status::invalid_argument(msg)
                        }
                    },
                }
            }
        }
    }
}

/// Generate all error types and conversions for a gRPC service file
pub fn generate_error_types() -> TokenStream {
    let validation_error = generate_validation_error();
    let service_error = generate_service_error();
    let status_conversion = generate_status_conversion();

    quote! {
        #validation_error
        #service_error
        #status_conversion
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_validation_error() {
        let tokens = generate_validation_error();
        let code = tokens.to_string();
        assert!(code.contains("ValidationError"));
        assert!(code.contains("InvalidField"));
        assert!(code.contains("MissingRequired"));
    }

    #[test]
    fn test_generate_service_error() {
        let tokens = generate_service_error();
        let code = tokens.to_string();
        assert!(code.contains("ServiceError"));
        assert!(code.contains("Validation"));
        assert!(code.contains("Storage"));
    }

    #[test]
    fn test_generate_status_conversion() {
        let tokens = generate_status_conversion();
        let code = tokens.to_string();
        // quote! generates "tonic :: Status" with spaces
        assert!(code.contains("tonic") && code.contains("Status"));
        assert!(code.contains("invalid_argument"));
        assert!(code.contains("not_found"));
    }
}
