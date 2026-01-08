#![no_std]

extern crate alloc;

// Cache Coherency System
pub mod cache_coherency;
pub mod mmio;
pub mod runtime;
pub mod state_machine;

// Shadow Register Management System
pub mod shadow_register;
pub mod fuse_manager;
pub mod sync_manager;
pub mod ecc_handler;
pub mod shadow_mmio;
pub mod version_control;
pub mod shadow_runtime;

// Re-export main cache coherency types
pub use cache_coherency::{CacheLine, CacheState, L3Directory};
pub use mmio::{CoherencyOp, MMIOCoherency};
pub use runtime::{CoherencyRuntime, CoreCacheController};
pub use state_machine::{CacheEvent, CoherencyStateMachine};

// Re-export main shadow register types
pub use shadow_register::{RegisterState, ShadowRegister, ShadowRegisterBank};
pub use fuse_manager::{FuseManager, FuseMode, FuseState, HardwareFuse};
pub use sync_manager::{SyncDirection, SyncManager, SyncPolicy, SyncResult};
pub use ecc_handler::{ECCError, ECCManager, ECCStrategy, HammingECC};
pub use shadow_mmio::{ShadowMMIOController, ShadowRegisterMMIO, MMIOCommand};
pub use version_control::{VersionedShadowRegister, VersionHistory, VersionEntry};
pub use shadow_runtime::{ShadowRegisterRuntime, VersionedShadowRuntime};
