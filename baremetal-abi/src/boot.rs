//! Boot infrastructure for i9-12900K bare-metal firmware
//!
//! UEFI entry point, kernel initialization, and boot info

use crate::coherency_runtime::CoherencyRuntime;
use crate::cpu;
use bootloader_api::BootInfo;

/// Kernel-lifetime coherency runtime, initialized once during boot.
/// Lives in BSS so the initialization in init_cache_coherency() is retained
/// instead of being dropped at end of scope.
static mut COHERENCY_RUNTIME: CoherencyRuntime = CoherencyRuntime::new();

/// Main kernel initialization function
///
/// This function can be used by binaries as their kernel entry point.
/// Binaries should call `entry_point!(kernel_main)` in their own code.
///
/// Initializes:
/// - CPU (SSE/AVX)
/// - Interrupts (IDT)
/// - Cache coherency (MESI protocol)
/// - Memory management
/// - Performance monitoring
pub fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // Initialize serial output for debugging (if available)
    #[cfg(feature = "serial")]
    crate::serial::init();

    log("i9-12900K Bare-Metal Firmware ABI v0.1.0");
    log("Initializing CPU...");

    // Initialize CPU (enable SSE/AVX)
    unsafe {
        cpu::init_cpu();
    }

    log("CPU initialized");

    // Detect CPU features
    let features = cpu::CpuFeatures::detect();
    log("CPU Features detected:");
    log_fmt(format_args!("  AVX2: {}", features.avx2));
    log_fmt(format_args!("  AES-NI: {}", features.aes));
    log_fmt(format_args!("  AVX-512: {}", features.avx512f));

    // Detect core type
    let core_type = cpu::get_core_type();
    let core_id = cpu::get_core_id();
    log_fmt(format_args!(
        "Running on Core {} - Type: {:?}",
        core_id, core_type
    ));

    // Initialize cache coherency system
    log("Initializing cache coherency...");
    init_cache_coherency();

    // Initialize interrupts
    log("Initializing interrupt handlers...");
    crate::interrupts::init();

    // Initialize memory management
    log("Initializing memory management...");
    crate::memory::init(boot_info);

    // Initialize performance monitoring
    log("Initializing performance counters...");
    crate::performance::init();

    log("Boot complete! Entering kernel loop...");

    // Kernel main loop
    kernel_loop()
}

/// Initialize cache coherency for hybrid architecture
fn init_cache_coherency() {
    unsafe {
        let runtime = coherency_runtime();

        // Initialize P-cores (0-7)
        for p_core in 0..8 {
            runtime.init_core(p_core);
        }

        // Initialize E-cores (8-15)
        for e_core in 8..16 {
            runtime.init_core(e_core);
        }
    }

    log("Cache coherency initialized for 16 cores (8P+8E)");
}

/// Access the boot-initialized coherency runtime
///
/// # Safety
/// The caller must guarantee exclusive access: the kernel is single-threaded
/// during boot and the kernel loop, and the returned reference must not be
/// held across a point where another mutable reference is created.
pub unsafe fn coherency_runtime() -> &'static mut CoherencyRuntime {
    &mut *core::ptr::addr_of_mut!(COHERENCY_RUNTIME)
}

/// Kernel main loop
fn kernel_loop() -> ! {
    log("Kernel loop started");

    let mut counter: u64 = 0;

    loop {
        // Read TSC
        let tsc = cpu::read_tsc();

        // Every billion cycles, print a message
        if counter % 1_000_000_000 == 0 {
            log_fmt(format_args!("TSC: {}", tsc));
        }

        counter += 1;

        // Hint to CPU to reduce power in idle loop
        unsafe {
            core::arch::asm!("pause", options(nomem, nostack));
        }
    }
}

/// Simple logging function
fn log(message: &str) {
    // In a real implementation, this would write to serial port or framebuffer
    #[cfg(feature = "serial")]
    crate::serial::write_line(message);

    // For now, just a no-op in release mode
    #[cfg(not(feature = "serial"))]
    let _ = message;
}

/// Formatted logging
fn log_fmt(args: core::fmt::Arguments) {
    #[cfg(feature = "serial")]
    {
        use core::fmt::Write;
        let mut serial = crate::serial::SerialPort;
        let _ = serial.write_fmt(args);
    }

    #[cfg(not(feature = "serial"))]
    let _ = args;
}
