//! Optional observations over Umbriel snapshots, independent of sockets and runtimes.
//!
//! [`inventory`] installs complete families with coherent local evidence.
//! [`active`] measures continuous *observed* activation with injected time.
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]
pub mod active;
pub mod inventory;
