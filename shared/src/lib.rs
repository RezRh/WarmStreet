// lib.rs - Shared types and utilities without Crux
// This module provides shared types, error handling, and utility functions
// for the Tauri backend

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_lines)]

pub mod types;
pub mod image_processing;
pub mod crypto;

// Re-export types for convenience
pub use types::*;
