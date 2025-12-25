//! Storage layer generation
//!
//! This module contains:
//! - Trait generation for backend-agnostic storage interfaces
//! - Backend implementations (SeaORM, Ecto, Diesel, etc.)

pub mod seaorm;
mod traits;

pub use traits::generate;
