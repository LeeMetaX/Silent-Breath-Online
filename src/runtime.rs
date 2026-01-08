/// Real-Time Runtime Implementation
/// Demonstrates the complete 5-step cache coherency flow

use crate::cache_coherency::{CacheLine, CacheState, L3Directory};
use crate::mmio::{MMIOCoherency, COHERENCY_CTL_BASE};
use crate::state_machine::{CacheEvent, CoherencyStateMachine};

/// Per-Core Cache Controller
pub struct CoreCacheController {
    core_id: u8,
    l1_cache: [CacheLine; 64],
    mmio: MMIOCoherency,
    state_machine: CoherencyStateMachine,
}

impl CoreCacheController {
    /// Initialize core cache controller
    pub unsafe fn new(core_id: u8) -> Self {
        const INIT: CacheLine = CacheLine::new();
        Self {
            core_id,
            l1_cache: [INIT; 64],
            mmio: MMIOCoherency::new(COHERENCY_CTL_BASE + (core_id as usize * 0x1000)),
            state_machine: CoherencyStateMachine::new(),
        }
    }

    /// Step 1 & 2: Core reads data (becomes Shared)
    #[inline]
    pub unsafe fn read(&mut self, address: u64) -> Result<u64, ()> {
        let cache_idx = (address >> 6) % 64;
        let line = &self.l1_cache[cache_idx as usize];

        match line.get_state() {
            CacheState::Invalid => {
                // Trigger MMIO read to fetch from L3
                self.mmio.mmio_cache_read(self.core_id, address)?;

                // Transition Invalid → Shared
                line.force_state(CacheState::Shared);

                Ok(address) // Return data
            }
            CacheState::Shared | CacheState::Exclusive | CacheState::Modified => {
                // Cache hit - no state change needed
                Ok(address)
            }
        }
    }

    /// Step 3: Core writes data (invalidates other cores)
    #[inline]
    pub unsafe fn write(&mut self, address: u64, _value: u64) -> Result<(), ()> {
        let cache_idx = (address >> 6) % 64;
        let line = &self.l1_cache[cache_idx as usize];

        let current_state = line.get_state();

        // Determine next state based on current state
        let next_state = self.state_machine.transition(current_state, CacheEvent::LocalWrite);

        // Trigger MMIO write (broadcasts invalidation via L3)
        self.mmio.mmio_cache_write(self.core_id, address)?;

        // Transition to Modified state
        line.force_state(next_state);

        Ok(())
    }

    /// Step 4: Handle invalidation from another core's write
    #[inline]
    pub unsafe fn handle_invalidation(&mut self, address: u64) {
        let cache_idx = (address >> 6) % 64;
        let line = &self.l1_cache[cache_idx as usize];

        // Transition to Invalid when another core writes
        line.force_state(CacheState::Invalid);
    }
}

/// Multi-Core Coherency Runtime
/// Demonstrates the complete 5-step flow from your example
pub struct CoherencyRuntime {
    cores: [Option<CoreCacheController>; 8],
    l3_directory: L3Directory,
}

impl CoherencyRuntime {
    pub const fn new() -> Self {
        Self {
            cores: [None, None, None, None, None, None, None, None],
            l3_directory: L3Directory::new(),
        }
    }

    /// Initialize core
    pub unsafe fn init_core(&mut self, core_id: u8) {
        if (core_id as usize) < 8 {
            self.cores[core_id as usize] = Some(CoreCacheController::new(core_id));
        }
    }

    /// Execute the complete 5-step coherency flow
    pub unsafe fn execute_coherency_flow(&mut self, address: u64) -> Result<(), ()> {
        // Step 1: Core 1 reads data → stored in L1, L2, L3 (Shared state)
        if let Some(ref mut core1) = self.cores[1] {
            core1.read(address)?;
        }

        // Step 2: Core 2 reads same data → also Shared across all levels
        if let Some(ref mut core2) = self.cores[2] {
            core2.read(address)?;
        }

        // Step 3: Core 1 writes to data → invalidates Core 2's copy via L3
        if let Some(ref mut core1) = self.cores[1] {
            core1.write(address, 0xDEADBEEF)?;

            // Step 4: Core 2's cache line marked Invalid
            if let Some(ref mut core2) = self.cores[2] {
                core2.handle_invalidation(address);
            }
        }

        // Step 5: Core 2 reads again → fetches from Core 1 or L3
        if let Some(ref mut core2) = self.cores[2] {
            core2.read(address)?;
        }

        Ok(())
    }
}

/// Entry point for firmware replacement
#[no_mangle]
pub unsafe extern "C" fn mmio_coherency_init() -> *mut CoherencyRuntime {
    let runtime = Box::leak(Box::new(CoherencyRuntime::new()));

    // Initialize all cores
    for core_id in 0..8 {
        runtime.init_core(core_id);
    }

    runtime as *mut CoherencyRuntime
}

/// Execute real-time coherency operation
#[no_mangle]
pub unsafe extern "C" fn mmio_coherency_execute(
    runtime: *mut CoherencyRuntime,
    address: u64,
) -> i32 {
    if runtime.is_null() {
        return -1;
    }

    match (*runtime).execute_coherency_flow(address) {
        Ok(()) => 0,
        Err(()) => -1,
    }
}
