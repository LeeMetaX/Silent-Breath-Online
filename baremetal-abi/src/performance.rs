//! Performance monitoring for i9-12900K
//!
//! Intel Performance Monitoring Unit (PMU) interface

use crate::cpu::{read_msr, write_msr};

/// Performance counter MSRs
pub mod msr {
    /// Performance counter 0
    pub const IA32_PMC0: u32 = 0xC1;
    /// Performance counter 1
    pub const IA32_PMC1: u32 = 0xC2;
    /// Performance event select 0
    pub const IA32_PERFEVTSEL0: u32 = 0x186;
    /// Performance event select 1
    pub const IA32_PERFEVTSEL1: u32 = 0x187;
    /// Fixed-function performance counter 0 (instructions retired)
    pub const IA32_FIXED_CTR0: u32 = 0x309;
    /// Fixed-function performance counter 1 (unhalted core cycles)
    pub const IA32_FIXED_CTR1: u32 = 0x30A;
    /// Fixed-function performance counter 2 (unhalted reference cycles)
    pub const IA32_FIXED_CTR2: u32 = 0x30B;
    /// Fixed-function counter control
    pub const IA32_FIXED_CTR_CTRL: u32 = 0x38D;
    /// Global performance counter control
    pub const IA32_PERF_GLOBAL_CTRL: u32 = 0x38F;
}

/// Performance event types
#[derive(Debug, Clone, Copy)]
#[repr(u64)]
pub enum PerfEvent {
    /// Instructions retired
    InstructionsRetired = 0x00C0,
    /// Unhalted core cycles
    UnhaltedCoreCycles = 0x003C,
    /// Branch instructions retired
    BranchInstructions = 0x00C4,
    /// Branch mispredictions
    BranchMispredictions = 0x00C5,
    /// L1 data cache misses
    L1DataCacheMisses = 0x0151,
    /// L2 cache misses
    L2CacheMisses = 0x0124,
    /// LLC (L3) cache misses
    LLCMisses = 0x412E,
    /// TLB misses
    TLBMisses = 0x0108,
}

/// Performance counter
pub struct PerfCounter {
    counter_msr: u32,
    event_select_msr: u32,
    enabled: bool,
}

impl PerfCounter {
    /// Create a new performance counter
    pub const fn new(index: u8) -> Self {
        Self {
            counter_msr: msr::IA32_PMC0 + index as u32,
            event_select_msr: msr::IA32_PERFEVTSEL0 + index as u32,
            enabled: false,
        }
    }

    /// Start counting an event
    ///
    /// # Safety
    /// Must be called from ring 0
    pub unsafe fn start(&mut self, event: PerfEvent) {
        // Event select format:
        // [7:0]   Event select (low)
        // [15:8]  UMask (unit mask)
        // [16]    USR (count in user mode)
        // [17]    OS (count in OS mode)
        // [18]    E (edge detect)
        // [19]    PC (pin control)
        // [20]    INT (APIC interrupt enable)
        // [21]    ANY (count on any thread)
        // [22]    EN (enable counter)
        // [23]    INV (invert counter mask)
        // [31:24] Counter mask

        let event_value = event as u64;
        let config = event_value | (1 << 22) | (1 << 16) | (1 << 17); // EN | USR | OS

        write_msr(self.event_select_msr, config);
        write_msr(self.counter_msr, 0); // Reset counter
        self.enabled = true;
    }

    /// Stop counting
    ///
    /// # Safety
    /// Must be called from ring 0
    pub unsafe fn stop(&mut self) {
        write_msr(self.event_select_msr, 0);
        self.enabled = false;
    }

    /// Read current counter value
    ///
    /// # Safety
    /// Must be called from ring 0
    pub unsafe fn read(&self) -> u64 {
        read_msr(self.counter_msr)
    }

    /// Reset counter to zero
    ///
    /// # Safety
    /// Must be called from ring 0
    pub unsafe fn reset(&self) {
        write_msr(self.counter_msr, 0);
    }
}

/// Fixed-function performance counters
pub struct FixedPerfCounters;

impl FixedPerfCounters {
    /// Read instructions retired
    ///
    /// # Safety
    /// Must be called from ring 0
    pub unsafe fn instructions_retired() -> u64 {
        read_msr(msr::IA32_FIXED_CTR0)
    }

    /// Read unhalted core cycles
    ///
    /// # Safety
    /// Must be called from ring 0
    pub unsafe fn unhalted_core_cycles() -> u64 {
        read_msr(msr::IA32_FIXED_CTR1)
    }

    /// Read unhalted reference cycles
    ///
    /// # Safety
    /// Must be called from ring 0
    pub unsafe fn unhalted_ref_cycles() -> u64 {
        read_msr(msr::IA32_FIXED_CTR2)
    }

    /// Calculate IPC (Instructions Per Cycle)
    ///
    /// # Safety
    /// Must be called from ring 0
    pub unsafe fn calculate_ipc() -> f64 {
        let instructions = Self::instructions_retired() as f64;
        let cycles = Self::unhalted_core_cycles() as f64;

        if cycles > 0.0 {
            instructions / cycles
        } else {
            0.0
        }
    }
}

/// Performance monitoring manager
pub struct PerfMonitor {
    counters: [PerfCounter; 4],
}

impl PerfMonitor {
    /// Create a new performance monitor
    pub const fn new() -> Self {
        Self {
            counters: [
                PerfCounter::new(0),
                PerfCounter::new(1),
                PerfCounter::new(2),
                PerfCounter::new(3),
            ],
        }
    }

    /// Get a mutable reference to a counter
    pub fn counter_mut(&mut self, index: usize) -> Option<&mut PerfCounter> {
        self.counters.get_mut(index)
    }

    /// Get a reference to a counter
    pub fn counter(&self, index: usize) -> Option<&PerfCounter> {
        self.counters.get(index)
    }

    /// Enable all fixed-function counters
    ///
    /// # Safety
    /// Must be called from ring 0
    pub unsafe fn enable_fixed_counters() {
        // Enable all fixed counters: instructions, core cycles, ref cycles
        // Bits [3:0] = 0x3 (enable in user and OS mode for counter 0)
        // Bits [7:4] = 0x3 (enable in user and OS mode for counter 1)
        // Bits [11:8] = 0x3 (enable in user and OS mode for counter 2)
        let ctrl = 0x333u64;
        write_msr(msr::IA32_FIXED_CTR_CTRL, ctrl);

        // Enable fixed counters in global control
        // Bits [34:32] enable fixed counters 0-2
        let global_ctrl = (0x7u64 << 32) | 0xF; // Enable fixed 0-2 and PMC 0-3
        write_msr(msr::IA32_PERF_GLOBAL_CTRL, global_ctrl);
    }
}

/// Global performance monitor instance
static mut PERF_MONITOR: PerfMonitor = PerfMonitor::new();

/// Initialize performance monitoring
pub fn init() {
    unsafe {
        PerfMonitor::enable_fixed_counters();
    }
}

/// Get global performance monitor
///
/// # Safety
/// Mutable access to static mut
pub unsafe fn get_monitor() -> &'static mut PerfMonitor {
    &mut PERF_MONITOR
}

/// Benchmark a function and return elapsed cycles
pub fn benchmark<F, R>(f: F) -> (R, u64)
where
    F: FnOnce() -> R,
{
    unsafe {
        let start_cycles = FixedPerfCounters::unhalted_core_cycles();
        let result = f();
        let end_cycles = FixedPerfCounters::unhalted_core_cycles();
        (result, end_cycles - start_cycles)
    }
}

/// Benchmark a function and return IPC
pub fn benchmark_ipc<F, R>(f: F) -> (R, f64)
where
    F: FnOnce() -> R,
{
    unsafe {
        let start_instructions = FixedPerfCounters::instructions_retired();
        let start_cycles = FixedPerfCounters::unhalted_core_cycles();

        let result = f();

        let end_instructions = FixedPerfCounters::instructions_retired();
        let end_cycles = FixedPerfCounters::unhalted_core_cycles();

        let instructions = (end_instructions - start_instructions) as f64;
        let cycles = (end_cycles - start_cycles) as f64;

        let ipc = if cycles > 0.0 {
            instructions / cycles
        } else {
            0.0
        };

        (result, ipc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perf_event_values() {
        assert_eq!(PerfEvent::InstructionsRetired as u64, 0x00C0);
        assert_eq!(PerfEvent::UnhaltedCoreCycles as u64, 0x003C);
    }

    #[test]
    fn test_perf_counter_creation() {
        let counter = PerfCounter::new(0);
        assert_eq!(counter.counter_msr, msr::IA32_PMC0);
        assert_eq!(counter.event_select_msr, msr::IA32_PERFEVTSEL0);
        assert!(!counter.enabled);
    }
}
