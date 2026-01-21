//! Minimal i9-12900K Bare-Metal Kernel
//!
//! Demonstrates all features of the i9-12900K bare-metal ABI:
//! - Core type detection (P-core vs E-core)
//! - Performance monitoring (cycles, IPC)
//! - Cache coherency (MESI protocol)
//! - MSR access
//! - Exception handling
//!
//! Boots via Multiboot2 protocol (QEMU/GRUB compatible)

#![no_std]
#![no_main]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::fmt::Write;
use i9_12900k_baremetal_abi::{
    cpu, performance, CoreType,
    coherency_runtime::CoherencyRuntime,
    boot::BootInfo,
};

// ============================================================================
// Multiboot2 Header
// ============================================================================

/// Multiboot2 header structure
///
/// This header allows the kernel to be loaded by Multiboot2-compliant
/// bootloaders like QEMU (with -kernel flag) and GRUB.
#[repr(C, align(8))]
struct Multiboot2Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    // End tag
    end_tag_type: u16,
    end_tag_flags: u16,
    end_tag_size: u32,
}

/// Multiboot2 magic number
const MULTIBOOT2_MAGIC: u32 = 0xE85250D6;

/// Architecture: i386 (32-bit protected mode entry)
const MULTIBOOT2_ARCH_I386: u32 = 0;

/// Multiboot2 header (placed at the beginning of the binary)
#[used]
#[link_section = ".multiboot2"]
static MULTIBOOT2_HEADER: Multiboot2Header = {
    const HEADER_LENGTH: u32 = 24;
    Multiboot2Header {
        magic: MULTIBOOT2_MAGIC,
        architecture: MULTIBOOT2_ARCH_I386,
        header_length: HEADER_LENGTH,
        checksum: 0u32.wrapping_sub(MULTIBOOT2_MAGIC.wrapping_add(MULTIBOOT2_ARCH_I386).wrapping_add(HEADER_LENGTH)),
        end_tag_type: 0,
        end_tag_flags: 0,
        end_tag_size: 8,
    }
};

// ============================================================================
// Memory Allocator
// ============================================================================

/// Dummy allocator for bare-metal (fails all allocations)
/// This satisfies the linker requirement for a global allocator
struct DummyAllocator;

unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // No-op
    }
}

#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

/// Simple serial port driver (COM1)
struct SerialPort;

impl SerialPort {
    const PORT: u16 = 0x3F8; // COM1

    /// Initialize serial port
    #[allow(dead_code)]
    unsafe fn init() {
        use core::arch::asm;

        // Disable interrupts
        asm!("out dx, al", in("dx") Self::PORT + 1, in("al") 0x00u8, options(nomem, nostack));

        // Enable DLAB
        asm!("out dx, al", in("dx") Self::PORT + 3, in("al") 0x80u8, options(nomem, nostack));

        // Set divisor to 3 (38400 baud)
        asm!("out dx, al", in("dx") Self::PORT, in("al") 0x03u8, options(nomem, nostack));
        asm!("out dx, al", in("dx") Self::PORT + 1, in("al") 0x00u8, options(nomem, nostack));

        // 8N1
        asm!("out dx, al", in("dx") Self::PORT + 3, in("al") 0x03u8, options(nomem, nostack));

        // Enable FIFO
        asm!("out dx, al", in("dx") Self::PORT + 2, in("al") 0xC7u8, options(nomem, nostack));

        // Enable IRQs, RTS/DSR
        asm!("out dx, al", in("dx") Self::PORT + 4, in("al") 0x0Bu8, options(nomem, nostack));
    }

    /// Write a byte to serial port
    fn write_byte(byte: u8) {
        unsafe {
            use core::arch::asm;
            // Wait for transmit ready (bit 5 of line status)
            let mut ready = 0u8;
            while (ready & 0x20) == 0 {
                asm!("in al, dx", out("al") ready, in("dx") Self::PORT + 5, options(nomem, nostack));
            }
            // Write byte
            asm!("out dx, al", in("dx") Self::PORT, in("al") byte, options(nomem, nostack));
        }
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            Self::write_byte(byte);
        }
        Ok(())
    }
}

/// Serial output macro
macro_rules! serial_print {
    ($($arg:tt)*) => {{
        #[cfg(not(test))]
        {
            use core::fmt::Write;
            let _ = write!(SerialPort, $($arg)*);
        }
    }};
}

/// Serial output with newline
macro_rules! serial_println {
    () => { serial_print!("\n") };
    ($($arg:tt)*) => {{
        serial_print!($($arg)*);
        serial_print!("\n");
    }};
}

/// Static boot info
///
/// Populated by parsing Multiboot2 information.
static mut BOOT_INFO: BootInfo = BootInfo {
    memory_regions: None,
    framebuffer: None,
};

// ============================================================================
// Boot Entry Point
// ============================================================================

/// Multiboot2 entry point
///
/// Called by the bootloader with:
/// - EAX = Multiboot2 magic value (0x36d76289)
/// - EBX = Physical address of Multiboot2 information structure
///
/// The bootloader loads us in 32-bit protected mode, so we need to:
/// 1. Set up a minimal 64-bit environment
/// 2. Parse Multiboot2 info
/// 3. Jump to 64-bit kernel_main
#[no_mangle]
#[unsafe(naked)]
pub extern "C" fn _start() -> ! {
    // For now, we'll use a simplified entry that assumes we're already in 64-bit mode
    // (QEMU -kernel does this for us)
    core::arch::naked_asm!(
        // Save Multiboot2 info pointer (in EBX)
        "mov rdi, rbx",
        // Call Rust entry point
        "call {rust_entry}",
        // Should never return, but halt just in case
        "2:",
        "cli",
        "hlt",
        "jmp 2b",
        rust_entry = sym rust_entry,
    )
}

/// Rust entry point called from assembly
///
/// Arguments:
/// - multiboot_info_addr: Physical address of Multiboot2 info structure
unsafe fn rust_entry(_multiboot_info_addr: u64) -> ! {
    // TODO: Parse Multiboot2 info structure and populate BOOT_INFO
    // For now, just use empty boot info
    kernel_main(&mut BOOT_INFO)
}

/// Main kernel entry point
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // Suppress unused warning
    let _ = boot_info;

    serial_println!("========================================");
    serial_println!("i9-12900K Minimal Bare-Metal Kernel");
    serial_println!("ABI Version: 0.1.0");
    serial_println!("========================================\n");

    // Step 1: Initialize CPU (enable SSE/AVX)
    serial_println!("[1/7] Initializing CPU...");
    unsafe {
        cpu::init_cpu();
    }
    serial_println!("      ✓ CPU initialized (SSE/AVX enabled)\n");

    // Step 2: Initialize interrupts
    serial_println!("[2/7] Setting up interrupt handlers...");
    i9_12900k_baremetal_abi::interrupts::init();
    serial_println!("      ✓ IDT loaded with 10 exception handlers\n");

    // Step 3: Detect CPU features
    serial_println!("[3/7] Detecting CPU features...");
    let features = cpu::CpuFeatures::detect();
    serial_println!("      CPU Feature Support:");
    serial_println!("      - SSE4.2:    {}", if features.sse4_2 { "✓" } else { "✗" });
    serial_println!("      - AVX:       {}", if features.avx { "✓" } else { "✗" });
    serial_println!("      - AVX2:      {}", if features.avx2 { "✓" } else { "✗" });
    serial_println!("      - AVX-512:   {}", if features.avx512f { "✓" } else { "✗ (disabled for E-core compat)" });
    serial_println!("      - AES-NI:    {}", if features.aes { "✓" } else { "✗" });
    serial_println!("      - RDRAND:    {}", if features.rdrand { "✓" } else { "✗" });
    serial_println!("      - BMI2:      {}", if features.bmi2 { "✓" } else { "✗" });
    serial_println!("");

    // Step 4: Detect current core type
    serial_println!("[4/7] Detecting core type...");
    let core_type = cpu::get_core_type();
    let core_id = cpu::get_core_id();
    let apic_id = cpu::get_apic_id();

    match core_type {
        CoreType::Performance => {
            serial_println!("      ✓ Running on P-CORE (Performance Core)");
            serial_println!("        Architecture: Golden Cove");
            serial_println!("        Core ID: {}", core_id);
            serial_println!("        APIC ID: {}", apic_id);
            serial_println!("        Features: HyperThreading, High IPC (~5.5-6.0)");
        }
        CoreType::Efficiency => {
            serial_println!("      ✓ Running on E-CORE (Efficiency Core)");
            serial_println!("        Architecture: Gracemont");
            serial_println!("        Core ID: {}", core_id);
            serial_println!("        APIC ID: {}", apic_id);
            serial_println!("        Features: No HT, Power Efficient (~4.0-4.5 IPC)");
        }
        CoreType::Unknown => {
            serial_println!("      ⚠ Unknown core type!");
        }
    }
    serial_println!("");

    // Step 5: Initialize cache coherency
    serial_println!("[5/7] Initializing cache coherency (MESI protocol)...");
    let mut coherency = CoherencyRuntime::new();
    unsafe {
        // Initialize P-cores (0-7)
        for p_core in 0..8 {
            coherency.init_core(p_core);
        }
        // Initialize E-cores (8-15)
        for e_core in 8..16 {
            coherency.init_core(e_core);
        }
    }
    serial_println!("      ✓ Cache coherency initialized for 16 cores");
    serial_println!("        - P-cores: 0-7 (Golden Cove)");
    serial_println!("        - E-cores: 8-15 (Gracemont)");
    serial_println!("");

    // Step 6: Initialize performance monitoring
    serial_println!("[6/7] Initializing performance counters...");
    performance::init();
    serial_println!("      ✓ Performance monitoring enabled");
    serial_println!("        - Fixed counters: Instructions, Cycles, Ref Cycles");
    serial_println!("        - Programmable counters: 4 available");
    serial_println!("");

    // Step 7: Read MSRs for platform info
    serial_println!("[7/7] Reading platform MSRs...");
    unsafe {
        let platform_info = cpu::read_msr(cpu::msr::MSR_PLATFORM_INFO);
        let max_non_turbo = (platform_info >> 8) & 0xFF;
        serial_println!("      Platform Info:");
        serial_println!("      - Max Non-Turbo Ratio: {}x (base frequency)", max_non_turbo);
        serial_println!("      - MSR_PLATFORM_INFO: {:#018x}", platform_info);
    }
    serial_println!("");

    serial_println!("========================================");
    serial_println!("Kernel Initialization Complete!");
    serial_println!("========================================\n");

    // Main kernel loop - demonstrate performance monitoring
    serial_println!("Entering demonstration loop...\n");

    demonstration_loop();
}

/// Main demonstration loop
fn demonstration_loop() -> ! {
    let mut iteration = 0u64;

    loop {
        iteration += 1;

        // Every 100 million iterations, print status
        if iteration % 100_000_000 == 0 {
            serial_println!("--- Iteration {} ---", iteration / 100_000_000);

            // Read TSC
            let tsc = cpu::read_tsc();
            serial_println!("TSC: {} cycles", tsc);

            // Measure IPC
            let ipc = unsafe {
                let instructions = performance::FixedPerfCounters::instructions_retired();
                let cycles = performance::FixedPerfCounters::unhalted_core_cycles();
                if cycles > 0 {
                    instructions as f64 / cycles as f64
                } else {
                    0.0
                }
            };
            serial_println!("IPC: {:.3} instructions per cycle", ipc);

            // Core info
            let core_type = cpu::get_core_type();
            let core_id = cpu::get_core_id();
            match core_type {
                CoreType::Performance => serial_println!("Core: P-core {}", core_id),
                CoreType::Efficiency => serial_println!("Core: E-core {}", core_id),
                CoreType::Unknown => serial_println!("Core: Unknown"),
            }
            serial_println!("");
        }

        // Yield CPU (reduces power consumption)
        unsafe {
            core::arch::asm!("pause", options(nomem, nostack));
        }

        // Every billion iterations, halt briefly
        if iteration % 1_000_000_000 == 0 {
            unsafe {
                core::arch::asm!("hlt", options(nomem, nostack));
            }
        }
    }
}

/// Panic handler with detailed debug information
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial_println!("\n========================================");
    serial_println!("KERNEL PANIC!");
    serial_println!("========================================");

    if let Some(location) = info.location() {
        serial_println!(
            "Location: {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }

    let message = info.message();
    serial_println!("Message: {}", message);

    serial_println!("\nCore State:");
    let core_type = cpu::get_core_type();
    let core_id = cpu::get_core_id();
    serial_println!("  Core ID: {}", core_id);
    serial_println!("  Core Type: {:?}", core_type);

    let tsc = cpu::read_tsc();
    serial_println!("  TSC: {}", tsc);

    serial_println!("\nHalting CPU...");
    serial_println!("========================================\n");

    // Halt forever
    loop {
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack));
        }
    }
}
