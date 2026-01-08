/// 4-State MESI Cache Coherency Protocol Implementation
/// Replaces traditional ROM/Firmware with Rust MMIO Real-Time Traversal

use core::sync::atomic::{AtomicU8, Ordering};

/// Cache Line States (4-State Logic Gating)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CacheState {
    Modified = 0b00,  // Dirty, exclusive - Core owns and modified
    Exclusive = 0b01, // Clean, exclusive - Core owns, not modified
    Shared = 0b10,    // Clean, shared - Multiple cores have copy
    Invalid = 0b11,   // Invalid - Must fetch from L3 or other core
}

impl From<u8> for CacheState {
    fn from(val: u8) -> Self {
        match val & 0b11 {
            0b00 => CacheState::Modified,
            0b01 => CacheState::Exclusive,
            0b10 => CacheState::Shared,
            0b11 => CacheState::Invalid,
            _ => unreachable!(),
        }
    }
}

/// Cache Line Metadata for Real-Time Traversal
#[repr(C, align(64))]
pub struct CacheLine {
    /// Atomic state for lock-free transitions
    state: AtomicU8,
    /// Physical address tag
    tag: u64,
    /// Core ID that owns this line (if Modified/Exclusive)
    owner_core: u8,
    /// Reference count for Shared state
    ref_count: AtomicU8,
    /// Data payload (64-byte cache line)
    data: [u8; 64],
}

impl CacheLine {
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(CacheState::Invalid as u8),
            tag: 0,
            owner_core: 0xFF,
            ref_count: AtomicU8::new(0),
            data: [0u8; 64],
        }
    }

    /// Get current cache state
    #[inline(always)]
    pub fn get_state(&self) -> CacheState {
        CacheState::from(self.state.load(Ordering::Acquire))
    }

    /// Atomic state transition with real-time guarantees
    #[inline(always)]
    pub fn transition(&self, from: CacheState, to: CacheState) -> Result<(), CacheState> {
        self.state
            .compare_exchange(
                from as u8,
                to as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|actual| CacheState::from(actual))
    }

    /// Force state transition (for invalidation)
    #[inline(always)]
    pub fn force_state(&self, new_state: CacheState) {
        self.state.store(new_state as u8, Ordering::Release);
    }
}

/// L3 Cache Directory for Multi-Core Coherency
pub struct L3Directory {
    /// Cache lines in L3
    lines: [CacheLine; 1024],
}

impl L3Directory {
    pub const fn new() -> Self {
        const INIT: CacheLine = CacheLine::new();
        Self {
            lines: [INIT; 1024],
        }
    }

    /// Real-Time Traversal: Step 1 - Core 1 reads data (Shared state)
    #[inline]
    pub fn core_read(&mut self, core_id: u8, address: u64) -> Result<&[u8; 64], ()> {
        let index = (address >> 6) % 1024;
        let line = &self.lines[index as usize];

        match line.get_state() {
            CacheState::Invalid => {
                // Fetch from memory, transition to Shared
                line.force_state(CacheState::Shared);
                line.ref_count.fetch_add(1, Ordering::AcqRel);
                Ok(&line.data)
            }
            CacheState::Shared => {
                // Already shared, increment ref count
                line.ref_count.fetch_add(1, Ordering::AcqRel);
                Ok(&line.data)
            }
            CacheState::Exclusive | CacheState::Modified => {
                // Transition to Shared if another core reads
                if line.owner_core != core_id {
                    line.force_state(CacheState::Shared);
                    line.ref_count.store(2, Ordering::Release);
                }
                Ok(&line.data)
            }
        }
    }

    /// Real-Time Traversal: Step 3 - Core 1 writes (Invalidates other cores)
    #[inline]
    pub fn core_write(&mut self, core_id: u8, address: u64) -> Result<&mut [u8; 64], ()> {
        let index = (address >> 6) % 1024;
        let line = &mut self.lines[index as usize];

        match line.get_state() {
            CacheState::Shared => {
                // Invalidate all other cores' copies via L3
                self.broadcast_invalidate(core_id, address);
                line.force_state(CacheState::Modified);
                line.owner_core = core_id;
                line.ref_count.store(0, Ordering::Release);
            }
            CacheState::Exclusive => {
                // Direct transition to Modified
                line.force_state(CacheState::Modified);
                line.owner_core = core_id;
            }
            CacheState::Invalid => {
                // Fetch exclusive, then modify
                line.force_state(CacheState::Modified);
                line.owner_core = core_id;
            }
            CacheState::Modified => {
                // Already modified by this or another core
                if line.owner_core != core_id {
                    // Writeback from other core, then acquire
                    self.writeback_and_acquire(line.owner_core, core_id, address);
                }
            }
        }

        Ok(unsafe { &mut *(&line.data as *const [u8; 64] as *mut [u8; 64]) })
    }

    /// Step 4: Broadcast invalidation to other cores
    #[inline(always)]
    fn broadcast_invalidate(&self, requesting_core: u8, address: u64) {
        // In real hardware, this would send invalidation messages
        // via the cache coherency bus to all other cores
        // Here we mark the operation as complete for real-time guarantees
        compiler_fence();
    }

    /// Writeback modified data and transfer ownership
    #[inline(always)]
    fn writeback_and_acquire(&mut self, old_owner: u8, new_owner: u8, address: u64) {
        compiler_fence();
        // Real implementation would DMA transfer or use coherency protocol
    }
}

/// Compiler fence for ordering guarantees
#[inline(always)]
fn compiler_fence() {
    core::sync::atomic::compiler_fence(Ordering::SeqCst);
}
