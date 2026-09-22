//! Supervises one TCP connection per vMix instance.
//!
//! `vmix-rs` owns blocking reader and writer threads. This crate adds one more
//! thread per instance so the plugin runtime never waits on those sockets.

mod cache;
mod config;
mod numbering;
mod pool;
mod status;

pub use cache::VmixState;
pub use config::{resolve_targets, TargetGroup, TargetSelector, VmixInstanceConfig};
pub use numbering::{
    acts_preview_name, acts_program_name, tcp_mix_value, xml_mix_index, xml_mix_path,
};
pub use pool::{CallError, Command, PoolEvent, PoolOptions, VmixPool};
pub use status::{ConnectionStatus, InstanceStatus};

#[cfg(any(test, feature = "mock"))]
pub mod mock;
