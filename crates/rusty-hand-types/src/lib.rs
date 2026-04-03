//! Core types and traits for the RustyHand Agent Operating System.
//!
//! This crate defines all shared data structures used across the RustyHand kernel,
//! runtime, memory substrate, and wire protocol. It contains no business logic.

pub mod agent;
pub mod approval;
pub mod capability;
pub mod config;
pub mod error;
pub mod event;
pub mod manifest_signing;
pub mod media;
pub mod memory;
pub mod message;
pub mod model_catalog;
pub mod scheduler;
pub mod serde_compat;
pub mod taint;
pub mod tool;
pub mod tool_compat;
pub mod webhook;
