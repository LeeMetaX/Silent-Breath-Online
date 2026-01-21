//! Boot infrastructure for i9-12900K bare-metal firmware
//!
//! UEFI entry point, kernel initialization, and boot info

use crate::cpu;

/// Boot information structure
///
/// Contains information about the system state at boot time.
/// In a full implementation, this would contain memory maps,
/// framebuffer info, ACPI tables, etc.
#[repr(C)]
pub struct BootInfo {
    /// Memory map entries (if available)
    pub memory_regions: Option<&'static [MemoryRegion]>,
    /// Framebuffer info (if available)
    pub framebuffer: Option<FramebufferInfo>,
}

/// Memory region descriptor
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: u64,
    pub size: u64,
    pub region_type: MemoryRegionType,
}

/// Memory region types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    Usable = 1,
    Reserved = 2,
    AcpiReclaimable = 3,
    AcpiNvs = 4,
    BadMemory = 5,
}

/// Framebuffer information
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {
    pub addr: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

impl BootInfo {
    /// Create a default/empty boot info
    pub const fn empty() -> Self {
        Self {
            memory_regions: None,
            framebuffer: None,
        }
    }
}

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
    serial::init();

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
    use crate::coherency_runtime::CoherencyRuntime;

    let mut runtime = CoherencyRuntime::new();

    unsafe {
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
    serial::write_str(message);

    // For now, just a no-op in release mode
    #[cfg(not(feature = "serial"))]
    let _ = message;
}

/// Formatted logging
fn log_fmt(args: core::fmt::Arguments) {
    use core::fmt::Write;

    #[cfg(feature = "serial")]
    {
        let mut serial = serial::SerialPort;
        let _ = serial.write_fmt(args);
    }

    #[cfg(not(feature = "serial"))]
    let _ = args;
}

/// Optional serial port module for debugging
#[cfg(feature = "serial")]
mod serial {
    use core::fmt;
    use x86_64::instructions::port::Port;

    const SERIAL_IO_PORT: u16 = 0x3F8; // COM1

    pub struct SerialPort;

    pub fn init() {
        unsafe {
            let mut port = Port::<u8>::new(SERIAL_IO_PORT + 1);
            port.write(0x00); // Disable interrupts

            let mut port = Port::<u8>::new(SERIAL_IO_PORT + 3);
            port.write(0x80); // Enable DLAB

            let mut port = Port::<u8>::new(SERIAL_IO_PORT);
            port.write(0x03); // Divisor low byte (38400 baud)

            let mut port = Port::<u8>::new(SERIAL_IO_PORT + 1);
            port.write(0x00); // Divisor high byte

            let mut port = Port::<u8>::new(SERIAL_IO_PORT + 3);
            port.write(0x03); // 8N1

            let mut port = Port::<u8>::new(SERIAL_IO_PORT + 2);
            port.write(0xC7); // Enable FIFO

            let mut port = Port::<u8>::new(SERIAL_IO_PORT + 4);
            port.write(0x0B); // IRQs enabled, RTS/DSR set
        }
    }

    pub fn write_byte(byte: u8) {
        unsafe {
            let mut port = Port::<u8>::new(SERIAL_IO_PORT);
            port.write(byte);
        }
    }

    pub fn write_str(s: &str) {
        for byte in s.bytes() {
            write_byte(byte);
        }
        write_byte(b'\n');
    }

    impl fmt::Write for SerialPort {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for byte in s.bytes() {
                write_byte(byte);
            }
            Ok(())
        }
    }
}
