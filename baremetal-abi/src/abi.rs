//! i9-12900K Custom ABI Implementation
//!
//! Defines calling conventions, core affinity, and system calls

use crate::{CoreAffinity, CoreType};
use core::arch::asm;

/// Function calling convention
///
/// Follows System V AMD64 ABI with extensions for core affinity
#[repr(C)]
pub struct CallingConvention;

impl CallingConvention {
    /// Parameter registers (in order)
    pub const PARAM_REGS: [&'static str; 6] = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];

    /// Return value registers
    pub const RETURN_REG_PRIMARY: &'static str = "rax";
    pub const RETURN_REG_SECONDARY: &'static str = "rdx";

    /// FP parameter registers
    pub const FP_PARAM_REGS: [&'static str; 8] =
        ["xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7"];

    /// Callee-saved registers
    pub const CALLEE_SAVED: [&'static str; 6] = ["rbx", "rbp", "r12", "r13", "r14", "r15"];

    /// Caller-saved registers
    pub const CALLER_SAVED: [&'static str; 9] =
        ["rax", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10", "r11"];

    /// Stack alignment requirement (bytes)
    pub const STACK_ALIGNMENT: usize = 16;
}

/// Core affinity attribute for functions
///
/// Used to mark functions that should run on specific core types
#[repr(transparent)]
pub struct CoreAffinityAttr(pub CoreAffinity);

/// System call interface
#[derive(Debug, Clone, Copy)]
#[repr(u64)]
pub enum Syscall {
    Exit = 0x00,
    ReadMsr = 0x01,
    WriteMsr = 0x02,
    GetCoreType = 0x03,
    SetCoreAffinity = 0x04,
    CacheFlush = 0x05,
    CacheInvalidate = 0x06,
    PerfCounterRead = 0x10,
    PerfCounterStart = 0x11,
    PerfCounterStop = 0x12,
}

/// Execute a system call
///
/// # Safety
/// System calls interact with privileged CPU state
#[inline]
pub unsafe fn syscall0(number: Syscall) -> i64 {
    let result: i64;
    asm!(
        "syscall",
        in("rax") number as u64,
        lateout("rax") result,
        lateout("rcx") _,  // Clobbered by syscall
        lateout("r11") _,  // Clobbered by syscall
        options(nostack, preserves_flags)
    );
    result
}

/// System call with 1 argument
#[inline]
pub unsafe fn syscall1(number: Syscall, arg1: u64) -> i64 {
    let result: i64;
    asm!(
        "syscall",
        in("rax") number as u64,
        in("rdi") arg1,
        lateout("rax") result,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags)
    );
    result
}

/// System call with 2 arguments
#[inline]
pub unsafe fn syscall2(number: Syscall, arg1: u64, arg2: u64) -> i64 {
    let result: i64;
    asm!(
        "syscall",
        in("rax") number as u64,
        in("rdi") arg1,
        in("rsi") arg2,
        lateout("rax") result,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags)
    );
    result
}

/// System call with 3 arguments
#[inline]
pub unsafe fn syscall3(number: Syscall, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    let result: i64;
    asm!(
        "syscall",
        in("rax") number as u64,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        lateout("rax") result,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags)
    );
    result
}

/// High-level syscall wrappers
pub mod syscalls {
    use super::*;

    /// Exit the current execution context
    pub unsafe fn exit(code: i64) -> ! {
        syscall1(Syscall::Exit, code as u64);
        core::hint::unreachable_unchecked()
    }

    /// Read Model Specific Register
    pub unsafe fn read_msr(msr: u32) -> Result<u64, i64> {
        let result = syscall1(Syscall::ReadMsr, msr as u64);
        if result >= 0 {
            Ok(result as u64)
        } else {
            Err(result)
        }
    }

    /// Write Model Specific Register
    pub unsafe fn write_msr(msr: u32, value: u64) -> Result<(), i64> {
        let result = syscall2(Syscall::WriteMsr, msr as u64, value);
        if result >= 0 {
            Ok(())
        } else {
            Err(result)
        }
    }

    /// Get current core type
    pub fn get_core_type() -> CoreType {
        unsafe {
            let result = syscall0(Syscall::GetCoreType);
            match result as u8 {
                0x40 => CoreType::Performance,
                0x20 => CoreType::Efficiency,
                _ => CoreType::Unknown,
            }
        }
    }

    /// Set core affinity for current thread
    pub fn set_core_affinity(affinity: CoreAffinity) -> Result<(), i64> {
        unsafe {
            let result = syscall1(Syscall::SetCoreAffinity, affinity as u64);
            if result >= 0 {
                Ok(())
            } else {
                Err(result)
            }
        }
    }

    /// Flush cache line
    pub unsafe fn cache_flush(address: u64) -> Result<(), i64> {
        let result = syscall1(Syscall::CacheFlush, address);
        if result >= 0 {
            Ok(())
        } else {
            Err(result)
        }
    }

    /// Invalidate cache line
    pub unsafe fn cache_invalidate(address: u64) -> Result<(), i64> {
        let result = syscall1(Syscall::CacheInvalidate, address);
        if result >= 0 {
            Ok(())
        } else {
            Err(result)
        }
    }
}

/// Function prologue macro for custom ABI
#[macro_export]
macro_rules! abi_function_prologue {
    () => {
        unsafe {
            core::arch::asm!(
                "push rbp",
                "mov rbp, rsp",
                "push rbx",
                "push r12",
                "push r13",
                "push r14",
                "push r15",
                options(nostack)
            );
        }
    };
}

/// Function epilogue macro for custom ABI
#[macro_export]
macro_rules! abi_function_epilogue {
    () => {
        unsafe {
            core::arch::asm!(
                "pop r15",
                "pop r14",
                "pop r13",
                "pop r12",
                "pop rbx",
                "pop rbp",
                "ret",
                options(nostack, noreturn)
            );
        }
    };
}

/// Mark function as performance-critical (must run on P-core)
///
/// # Example
/// ```rust
/// #[performance_critical]
/// fn fast_computation(x: i64) -> i64 {
///     x * x
/// }
/// ```
#[macro_export]
macro_rules! performance_critical {
    () => {
        #[inline(never)]
        #[no_mangle]
    };
}

/// Mark function as background task (can run on E-core)
#[macro_export]
macro_rules! background_task {
    () => {
        #[inline(never)]
        #[cold]
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calling_convention_constants() {
        assert_eq!(CallingConvention::PARAM_REGS.len(), 6);
        assert_eq!(CallingConvention::FP_PARAM_REGS.len(), 8);
        assert_eq!(CallingConvention::STACK_ALIGNMENT, 16);
    }

    #[test]
    fn test_syscall_numbers() {
        assert_eq!(Syscall::Exit as u64, 0x00);
        assert_eq!(Syscall::GetCoreType as u64, 0x03);
    }
}
