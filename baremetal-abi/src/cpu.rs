//! CPU initialization and management for i9-12900K
//!
//! Provides MSR access, CPUID queries, and core type detection

use crate::{CoreAffinity, CoreType};
use core::arch::asm;
use x86_64::registers::model_specific::Msr;

/// Model Specific Registers for i9-12900K
pub mod msr {
    /// Hardware P-State Request
    pub const MSR_HWP_REQUEST: u32 = 0x774;
    /// Turbo Ratio Limit
    pub const MSR_TURBO_RATIO_LIMIT: u32 = 0x1AD;
    /// Platform Info
    pub const MSR_PLATFORM_INFO: u32 = 0xCE;
    /// Performance Energy Bias Hint
    pub const MSR_ENERGY_PERF_BIAS: u32 = 0x1B0;
    /// Thread Director feedback
    pub const MSR_HW_FEEDBACK_PTR: u32 = 0x17D0;
    /// Package C-State limit
    pub const MSR_PKG_CST_CONFIG_CONTROL: u32 = 0xE2;
    /// IA32 APIC Base
    pub const MSR_APIC_BASE: u32 = 0x1B;
    /// Time Stamp Counter
    pub const MSR_TSC: u32 = 0x10;
}

/// Read a Model Specific Register
///
/// # Safety
/// Must be called from ring 0 with valid MSR address
#[inline]
pub unsafe fn read_msr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nomem, nostack)
    );
    ((high as u64) << 32) | (low as u64)
}

/// Write a Model Specific Register
///
/// # Safety
/// Must be called from ring 0 with valid MSR address and value
#[inline]
pub unsafe fn write_msr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nomem, nostack)
    );
}

/// CPUID result
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

/// Execute CPUID instruction
#[inline]
pub fn cpuid(leaf: u32, subleaf: u32) -> CpuidResult {
    let mut eax: u32;
    let mut ebx: u32;
    let mut ecx: u32;
    let mut edx: u32;

    unsafe {
        // LLVM reserves ebx in PIC mode, so we need to save/restore it manually
        asm!(
            "mov {tmp:r}, rbx",
            "cpuid",
            "xchg {tmp:r}, rbx",
            tmp = out(reg) ebx,
            inout("eax") leaf => eax,
            inout("ecx") subleaf => ecx,
            out("edx") edx,
            options(nomem, nostack, preserves_flags)
        );
    }

    CpuidResult { eax, ebx, ecx, edx }
}

/// Get current core type (P-core or E-core)
pub fn get_core_type() -> CoreType {
    // CPUID leaf 0x1A provides native model ID and core type
    let result = cpuid(0x1A, 0);

    // EAX[31:24] contains core type
    let core_type_id = (result.eax >> 24) as u8;

    match core_type_id {
        0x40 => CoreType::Performance,  // Intel Core (P-core)
        0x20 => CoreType::Efficiency,   // Intel Atom (E-core)
        _ => CoreType::Unknown,
    }
}

/// Get current logical processor ID (APIC ID)
pub fn get_apic_id() -> u32 {
    // CPUID leaf 0x1: EDX[31:24] contains initial APIC ID
    let result = cpuid(0x1, 0);
    (result.edx >> 24) & 0xFF
}

/// Get current core ID (0-15 for i9-12900K)
pub fn get_core_id() -> u8 {
    let apic_id = get_apic_id();
    // For i9-12900K: APIC ID maps to core ID
    // P-cores: 0-7 (logical 0-15 with HT)
    // E-cores: 8-15 (logical 16-23)
    (apic_id & 0xFF) as u8
}

/// Check if current core is a P-core
#[inline]
pub fn is_performance_core() -> bool {
    get_core_type() == CoreType::Performance
}

/// Check if current core is an E-core
#[inline]
pub fn is_efficiency_core() -> bool {
    get_core_type() == CoreType::Efficiency
}

/// CPU Features detected via CPUID
#[derive(Debug, Clone, Copy)]
pub struct CpuFeatures {
    pub sse: bool,
    pub sse2: bool,
    pub sse3: bool,
    pub ssse3: bool,
    pub sse4_1: bool,
    pub sse4_2: bool,
    pub avx: bool,
    pub avx2: bool,
    pub avx512f: bool,
    pub aes: bool,
    pub rdrand: bool,
    pub rdseed: bool,
    pub bmi1: bool,
    pub bmi2: bool,
    pub fma: bool,
    pub movbe: bool,
    pub xsave: bool,
    pub hypervisor: bool,
}

impl CpuFeatures {
    /// Detect CPU features using CPUID
    pub fn detect() -> Self {
        let leaf_1 = cpuid(0x1, 0);
        let leaf_7 = cpuid(0x7, 0);

        Self {
            // Leaf 1, ECX
            sse3: (leaf_1.ecx & (1 << 0)) != 0,
            ssse3: (leaf_1.ecx & (1 << 9)) != 0,
            fma: (leaf_1.ecx & (1 << 12)) != 0,
            sse4_1: (leaf_1.ecx & (1 << 19)) != 0,
            sse4_2: (leaf_1.ecx & (1 << 20)) != 0,
            movbe: (leaf_1.ecx & (1 << 22)) != 0,
            aes: (leaf_1.ecx & (1 << 25)) != 0,
            xsave: (leaf_1.ecx & (1 << 26)) != 0,
            avx: (leaf_1.ecx & (1 << 28)) != 0,
            rdrand: (leaf_1.ecx & (1 << 30)) != 0,
            hypervisor: (leaf_1.ecx & (1 << 31)) != 0,

            // Leaf 1, EDX
            sse: (leaf_1.edx & (1 << 25)) != 0,
            sse2: (leaf_1.edx & (1 << 26)) != 0,

            // Leaf 7, EBX
            bmi1: (leaf_7.ebx & (1 << 3)) != 0,
            avx2: (leaf_7.ebx & (1 << 5)) != 0,
            bmi2: (leaf_7.ebx & (1 << 8)) != 0,
            rdseed: (leaf_7.ebx & (1 << 18)) != 0,
            avx512f: (leaf_7.ebx & (1 << 16)) != 0,
        }
    }
}

/// Initialize CPU for bare-metal operation
///
/// # Safety
/// Must be called once during boot from BSP (Bootstrap Processor)
pub unsafe fn init_cpu() {
    // Enable SSE/AVX (required for Rust)
    let mut cr0: u64;
    asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack));
    cr0 &= !(1 << 2); // Clear CR0.EM (emulation)
    cr0 |= 1 << 1;    // Set CR0.MP (monitor coprocessor)
    asm!("mov cr0, {}", in(reg) cr0, options(nomem, nostack));

    let mut cr4: u64;
    asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack));
    cr4 |= 1 << 9;  // Set CR4.OSFXSR (enable SSE)
    cr4 |= 1 << 10; // Set CR4.OSXMMEXCPT (enable SSE exceptions)
    cr4 |= 1 << 18; // Set CR4.OSXSAVE (enable XSAVE)
    asm!("mov cr4, {}", in(reg) cr4, options(nomem, nostack));

    // Enable AVX via XSETBV (set XCR0)
    let xcr0: u64 = 0x7; // Enable x87, SSE, AVX (bits 0, 1, 2)
    let xcr0_low = xcr0 as u32;
    let xcr0_high = (xcr0 >> 32) as u32;
    asm!(
        "xsetbv",
        in("ecx") 0u32,
        in("eax") xcr0_low,
        in("edx") xcr0_high,
        options(nomem, nostack)
    );
}

/// Read Time Stamp Counter (TSC)
#[inline]
pub fn read_tsc() -> u64 {
    unsafe { read_msr(msr::MSR_TSC) }
}

/// Serialize and read TSC (prevents reordering)
#[inline]
pub fn read_tsc_serialized() -> u64 {
    let mut eax: u32;
    let mut edx: u32;
    unsafe {
        asm!(
            "lfence",
            "rdtsc",
            out("eax") eax,
            out("edx") edx,
            options(nomem, nostack)
        );
    }
    ((edx as u64) << 32) | (eax as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_type_values() {
        assert_eq!(CoreType::Performance as u8, 0x40);
        assert_eq!(CoreType::Efficiency as u8, 0x20);
    }

    #[test]
    fn test_core_affinity_values() {
        assert_eq!(CoreAffinity::Any as u64, 0x0000);
        assert_eq!(CoreAffinity::PerformanceRequired as u64, 0x0001);
    }
}
