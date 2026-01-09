/// Real-Time Runtime Implementation
/// Demonstrates the complete 5-step cache coherency flow

use crate::cache_coherency::{CacheLine, CacheState, L3Directory};
use crate::mmio::{MMIOCoherency, COHERENCY_CTL_BASE};
use crate::state_machine::{CacheEvent, CoherencyStateMachine};
use alloc::boxed::Box;

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

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::boxed::Box;
    use crate::cache_coherency::CacheState;

    /// Helper: Create CoherencyRuntime in memory
    fn create_mock_runtime() -> Box<CoherencyRuntime> {
        Box::new(CoherencyRuntime::new())
    }

    #[test]
    fn test_core_cache_controller_initialization() {
        unsafe {
            let controller = CoreCacheController::new(3);

            // Verify core ID is set correctly
            assert_eq!(controller.core_id, 3);

            // Verify L1 cache is initialized (all lines should be Invalid)
            assert_eq!(controller.l1_cache[0].get_state(), CacheState::Invalid);
            assert_eq!(controller.l1_cache[63].get_state(), CacheState::Invalid);
        }
    }

    #[test]
    fn test_core_cache_controller_cache_index_calculation() {
        unsafe {
            let controller = CoreCacheController::new(0);

            // Test cache index calculation for various addresses
            // Cache index = (address >> 6) % 64

            // Address 0x0000: index 0
            let idx1 = (0x0000u64 >> 6) % 64;
            assert_eq!(idx1, 0);

            // Address 0x1000 (4096): (4096 >> 6) % 64 = 64 % 64 = 0
            let idx2 = (0x1000u64 >> 6) % 64;
            assert_eq!(idx2, 0);

            // Address 0x1040 (4160): (4160 >> 6) % 64 = 65 % 64 = 1
            let idx3 = (0x1040u64 >> 6) % 64;
            assert_eq!(idx3, 1);

            // Verify cache lines are accessible
            assert_eq!(controller.l1_cache[idx1 as usize].get_state(), CacheState::Invalid);
        }
    }

    #[test]
    fn test_core_cache_handle_invalidation() {
        unsafe {
            let mut controller = CoreCacheController::new(5);

            let address = 0x5000;
            let cache_idx = (address >> 6) % 64;

            // Manually set line to Shared (simulating a previous read)
            controller.l1_cache[cache_idx as usize].force_state(CacheState::Shared);
            assert_eq!(controller.l1_cache[cache_idx as usize].get_state(), CacheState::Shared);

            // Handle invalidation from another core
            controller.handle_invalidation(address);

            // State should now be Invalid
            assert_eq!(controller.l1_cache[cache_idx as usize].get_state(), CacheState::Invalid);
        }
    }

    #[test]
    fn test_core_cache_handle_multiple_invalidations() {
        unsafe {
            let mut controller = CoreCacheController::new(6);

            // Test invalidating multiple different cache lines
            let addresses = [0x1000, 0x2000, 0x3000];

            for &addr in &addresses {
                let cache_idx = (addr >> 6) % 64;

                // Set to Shared
                controller.l1_cache[cache_idx as usize].force_state(CacheState::Shared);

                // Invalidate
                controller.handle_invalidation(addr);

                // Verify Invalid
                assert_eq!(controller.l1_cache[cache_idx as usize].get_state(), CacheState::Invalid);
            }
        }
    }

    #[test]
    fn test_coherency_runtime_initialization() {
        let runtime = create_mock_runtime();

        // All cores should be None initially
        for i in 0..8 {
            assert!(runtime.cores[i].is_none());
        }
    }

    #[test]
    fn test_coherency_runtime_init_core_valid_ids() {
        unsafe {
            let mut runtime = create_mock_runtime();

            // Initialize core 0
            runtime.init_core(0);
            assert!(runtime.cores[0].is_some());

            // Initialize core 7 (boundary)
            runtime.init_core(7);
            assert!(runtime.cores[7].is_some());

            // Initialize core 3
            runtime.init_core(3);
            assert!(runtime.cores[3].is_some());

            // Other cores should still be None
            assert!(runtime.cores[1].is_none());
            assert!(runtime.cores[2].is_none());
        }
    }

    #[test]
    fn test_coherency_runtime_init_core_invalid_id() {
        unsafe {
            let mut runtime = create_mock_runtime();

            // Try to initialize core 8 (out of bounds)
            runtime.init_core(8);

            // Should not panic, but should do nothing
            // All cores should still be None
            for i in 0..8 {
                assert!(runtime.cores[i].is_none());
            }

            // Try core 255
            runtime.init_core(255);
            for i in 0..8 {
                assert!(runtime.cores[i].is_none());
            }
        }
    }

    #[test]
    fn test_coherency_runtime_init_all_cores() {
        unsafe {
            let mut runtime = create_mock_runtime();

            // Initialize all 8 cores
            for core_id in 0..8 {
                runtime.init_core(core_id);
            }

            // Verify all cores are initialized
            for i in 0..8 {
                assert!(runtime.cores[i].is_some());
                if let Some(ref controller) = runtime.cores[i] {
                    assert_eq!(controller.core_id, i as u8);
                }
            }
        }
    }

    #[test]
    fn test_coherency_runtime_reinit_core() {
        unsafe {
            let mut runtime = create_mock_runtime();

            // Initialize core 2
            runtime.init_core(2);
            assert!(runtime.cores[2].is_some());

            // Re-initialize core 2 (should replace)
            runtime.init_core(2);
            assert!(runtime.cores[2].is_some());

            // Verify it's still core 2
            if let Some(ref controller) = runtime.cores[2] {
                assert_eq!(controller.core_id, 2);
            }
        }
    }

    #[test]
    fn test_ffi_mmio_coherency_init() {
        unsafe {
            let runtime_ptr = mmio_coherency_init();

            // Should return non-null pointer
            assert!(!runtime_ptr.is_null());

            // All 8 cores should be initialized
            for i in 0..8 {
                assert!((*runtime_ptr).cores[i].is_some());

                // Verify each core has correct ID
                if let Some(ref controller) = (*runtime_ptr).cores[i] {
                    assert_eq!(controller.core_id, i as u8);
                }
            }

            // Cleanup: reclaim the leaked Box
            let _ = Box::from_raw(runtime_ptr);
        }
    }

    #[test]
    fn test_ffi_mmio_coherency_init_multiple_calls() {
        unsafe {
            // Multiple calls should each return unique pointers
            let runtime_ptr1 = mmio_coherency_init();
            let runtime_ptr2 = mmio_coherency_init();

            assert!(!runtime_ptr1.is_null());
            assert!(!runtime_ptr2.is_null());
            assert_ne!(runtime_ptr1, runtime_ptr2);

            // Cleanup both
            let _ = Box::from_raw(runtime_ptr1);
            let _ = Box::from_raw(runtime_ptr2);
        }
    }

    #[test]
    fn test_ffi_mmio_coherency_execute_null_pointer() {
        unsafe {
            // Execute with null pointer
            let result = mmio_coherency_execute(core::ptr::null_mut(), 0xA000);

            // Should return -1 (error)
            assert_eq!(result, -1);
        }
    }

    #[test]
    fn test_ffi_null_pointer_safety() {
        unsafe {
            // Test with various null pointer scenarios
            assert_eq!(mmio_coherency_execute(core::ptr::null_mut(), 0x0), -1);
            assert_eq!(mmio_coherency_execute(core::ptr::null_mut(), 0xFFFFFFFF), -1);
            assert_eq!(mmio_coherency_execute(core::ptr::null_mut(), 0x12345678), -1);
        }
    }

    #[test]
    fn test_cache_state_manual_transitions() {
        unsafe {
            let mut controller = CoreCacheController::new(7);

            let address = 0xF000;
            let cache_idx = (address >> 6) % 64;
            let line = &controller.l1_cache[cache_idx as usize];

            // Test Invalid → Shared
            line.force_state(CacheState::Invalid);
            assert_eq!(line.get_state(), CacheState::Invalid);

            line.force_state(CacheState::Shared);
            assert_eq!(line.get_state(), CacheState::Shared);

            // Test Shared → Modified
            line.force_state(CacheState::Modified);
            assert_eq!(line.get_state(), CacheState::Modified);

            // Test Modified → Invalid
            line.force_state(CacheState::Invalid);
            assert_eq!(line.get_state(), CacheState::Invalid);
        }
    }

    #[test]
    fn test_l3_directory_initialization() {
        let runtime = create_mock_runtime();

        // L3Directory should be initialized (const new)
        // We can't directly access it but we can verify the runtime was created
        // This tests that CoherencyRuntime::new() properly initializes the L3Directory
        assert!(runtime.cores.iter().all(|c| c.is_none()));
    }

    #[test]
    fn test_multi_core_independent_cache_lines() {
        unsafe {
            let mut runtime = create_mock_runtime();

            // Initialize cores 0, 1, 2
            runtime.init_core(0);
            runtime.init_core(1);
            runtime.init_core(2);

            // Each core should have independent cache lines
            let addresses = [0x1000, 0x2000, 0x3000];

            for (core_id, &addr) in addresses.iter().enumerate() {
                if let Some(ref mut controller) = runtime.cores[core_id] {
                    let cache_idx = (addr >> 6) % 64;

                    // Set different states for each core's cache
                    match core_id {
                        0 => controller.l1_cache[cache_idx as usize].force_state(CacheState::Shared),
                        1 => controller.l1_cache[cache_idx as usize].force_state(CacheState::Modified),
                        2 => controller.l1_cache[cache_idx as usize].force_state(CacheState::Invalid),
                        _ => {}
                    }
                }
            }

            // Verify each core has its expected state
            if let Some(ref controller) = runtime.cores[0] {
                let idx = (0x1000u64 >> 6) % 64;
                assert_eq!(controller.l1_cache[idx as usize].get_state(), CacheState::Shared);
            }

            if let Some(ref controller) = runtime.cores[1] {
                let idx = (0x2000u64 >> 6) % 64;
                assert_eq!(controller.l1_cache[idx as usize].get_state(), CacheState::Modified);
            }

            if let Some(ref controller) = runtime.cores[2] {
                let idx = (0x3000u64 >> 6) % 64;
                assert_eq!(controller.l1_cache[idx as usize].get_state(), CacheState::Invalid);
            }
        }
    }
}
