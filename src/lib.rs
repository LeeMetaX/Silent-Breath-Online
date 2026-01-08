#![no_std]
#![feature(const_mut_refs)]

extern crate alloc;

pub mod cache_coherency;
pub mod mmio;
pub mod runtime;
pub mod state_machine;

// Re-export main types
pub use cache_coherency::{CacheLine, CacheState, L3Directory};
pub use mmio::{CoherencyOp, MMIOCoherency};
pub use runtime::{CoherencyRuntime, CoreCacheController};
pub use state_machine::{CacheEvent, CoherencyStateMachine};
