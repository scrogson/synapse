//! Storage layer generation
//!
//! This module contains:
//! - Trait generation for backend-agnostic storage interfaces
//! - Default implementations as standalone functions for partial overrides
//! - Backend implementations (SeaORM, Ecto, Diesel, etc.)

pub mod defaults;
pub mod seaorm;
mod traits;

pub use defaults::generate as generate_defaults;
pub use traits::generate;
