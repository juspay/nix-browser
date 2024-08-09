//! Rust crate to interact with Nix
//!
//! This crate exposes various types representing what nix command gives us,
//! along with a `from_nix` command to evaluate them.
pub mod command;
pub mod config;
pub mod copy;
pub mod env;
pub mod flake;
pub mod info;
pub mod refs;
pub mod version;
