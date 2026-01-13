#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]

extern crate alloc;

pub mod abi;
pub mod boot;
pub mod cpu;
pub mod interrupts;
pub mod memory;
pub mod performance;

// Re-export Silent-Breath-Online cache coherency system
pub use silent_breath_mmio::{
    cache_coherency, mmio, runtime as coherency_runtime, shadow_runtime, CacheState, CacheLine,
    CoherencyRuntime, ShadowRegisterRuntime,
};

/// Core type for i9-12900K hybrid architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CoreType {
    /// Performance core (Golden Cove, cores 0-7)
    Performance = 0x40,
    /// Efficiency core (Gracemont, cores 8-15)
    Efficiency = 0x20,
    /// Unknown core type
    Unknown = 0x00,
}

/// Core affinity hint for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum CoreAffinity {
    /// Any core type
    Any = 0x0000,
    /// Must run on P-core
    PerformanceRequired = 0x0001,
    /// Prefer E-core
    EfficiencyPreferred = 0x0002,
    /// P-core with HyperThreading
    PerformanceWithHT = 0x0003,
    /// Let Thread Director decide
    ThreadDirector = 0x00FF,
}

/// ABI version
pub const ABI_VERSION_MAJOR: u8 = 0;
pub const ABI_VERSION_MINOR: u8 = 1;
pub const ABI_VERSION_PATCH: u8 = 0;

/// Get ABI version as u32
pub const fn abi_version() -> u32 {
    ((ABI_VERSION_MAJOR as u32) << 16) | ((ABI_VERSION_MINOR as u32) << 8) | (ABI_VERSION_PATCH as u32)
}
