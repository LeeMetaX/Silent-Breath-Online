//! Interrupt handling for i9-12900K
//!
//! IDT setup and exception handlers with full debug information

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::registers::control::Cr2;
use core::sync::atomic::{AtomicU64, Ordering};

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

/// Breakpoint counter for debugging
static BREAKPOINT_COUNT: AtomicU64 = AtomicU64::new(0);

/// Initialize Interrupt Descriptor Table
pub fn init() {
    unsafe {
        IDT.breakpoint.set_handler_fn(breakpoint_handler);
        IDT.double_fault
            .set_handler_fn(double_fault_handler);
        IDT.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        IDT.page_fault.set_handler_fn(page_fault_handler);
        IDT.invalid_opcode
            .set_handler_fn(invalid_opcode_handler);
        IDT.device_not_available
            .set_handler_fn(device_not_available_handler);
        IDT.divide_error
            .set_handler_fn(divide_error_handler);
        IDT.overflow
            .set_handler_fn(overflow_handler);
        IDT.bound_range_exceeded
            .set_handler_fn(bound_range_exceeded_handler);
        IDT.alignment_check
            .set_handler_fn(alignment_check_handler);

        IDT.load();
    }
}

/// Breakpoint exception handler (INT 3)
///
/// Used for debugging - logs the breakpoint and continues execution
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    let count = BREAKPOINT_COUNT.fetch_add(1, Ordering::Relaxed);

    // Log breakpoint occurrence with full stack frame
    #[cfg(feature = "serial")]
    {
        use core::fmt::Write;
        let _ = writeln!(
            crate::boot::serial::SerialPort,
            "BREAKPOINT #{} at RIP: {:#x}",
            count,
            stack_frame.instruction_pointer.as_u64()
        );
    }

    // Breakpoint doesn't halt - execution continues after INT 3
    let _ = (count, stack_frame);
}

/// Double fault handler (non-recoverable)
///
/// Occurs when an exception happens while handling another exception
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "DOUBLE FAULT\n\
         Error Code: {:#x}\n\
         RIP: {:#x}\n\
         RSP: {:#x}\n\
         CS: {:#x}\n\
         RFLAGS: {:#x}",
        error_code,
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64(),
        stack_frame.code_segment,
        stack_frame.cpu_flags,
    );
}

/// General protection fault handler
///
/// Occurs on segment violations, privilege violations, or accessing invalid memory
extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    // Decode error code
    let external = (error_code & 0x01) != 0;
    let table = (error_code >> 1) & 0x03;
    let index = error_code >> 3;

    let table_name = match table {
        0b00 => "GDT",
        0b01 => "IDT",
        0b10 => "LDT",
        0b11 => "IDT",
        _ => "Unknown",
    };

    panic!(
        "GENERAL PROTECTION FAULT\n\
         Error Code: {:#x} (External: {}, Table: {}, Index: {})\n\
         RIP: {:#x}\n\
         RSP: {:#x}\n\
         CS: {:#x}",
        error_code,
        external,
        table_name,
        index,
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64(),
        stack_frame.code_segment,
    );
}

/// Page fault handler
///
/// Occurs on invalid memory access or page table violations
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let faulting_address = Cr2::read();

    panic!(
        "PAGE FAULT\n\
         Faulting Address: {:?}\n\
         Error Code: {:?}\n\
         - Present: {}\n\
         - Write: {}\n\
         - User: {}\n\
         - Reserved Write: {}\n\
         - Instruction Fetch: {}\n\
         RIP: {:#x}\n\
         RSP: {:#x}",
        faulting_address,
        error_code,
        error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION),
        error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE),
        error_code.contains(PageFaultErrorCode::USER_MODE),
        error_code.contains(PageFaultErrorCode::MALFORMED_TABLE),
        error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH),
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64(),
    );
}

/// Invalid opcode handler
///
/// Occurs when CPU encounters an instruction it doesn't recognize
extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!(
        "INVALID OPCODE\n\
         RIP: {:#x} (instruction causing fault)\n\
         RSP: {:#x}\n\
         CS: {:#x}",
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64(),
        stack_frame.code_segment,
    );
}

/// Device not available handler (x87 FPU not present)
///
/// Should not occur on i9-12900K as it has integrated FPU
extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    panic!(
        "DEVICE NOT AVAILABLE (FPU)\n\
         This should not occur on i9-12900K!\n\
         RIP: {:#x}\n\
         Check CR0.EM and CR0.TS bits",
        stack_frame.instruction_pointer.as_u64(),
    );
}

/// Divide by zero handler
extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    panic!(
        "DIVIDE ERROR (Division by zero)\n\
         RIP: {:#x}\n\
         RSP: {:#x}",
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64(),
    );
}

/// Overflow exception handler (INTO instruction with OF=1)
extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    panic!(
        "OVERFLOW EXCEPTION\n\
         RIP: {:#x}\n\
         RSP: {:#x}",
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64(),
    );
}

/// Bound range exceeded handler (BOUND instruction)
extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    panic!(
        "BOUND RANGE EXCEEDED\n\
         RIP: {:#x}\n\
         RSP: {:#x}",
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64(),
    );
}

/// Alignment check exception (unaligned memory access with AC flag set)
extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "ALIGNMENT CHECK EXCEPTION\n\
         Error Code: {:#x}\n\
         RIP: {:#x}\n\
         RSP: {:#x}\n\
         Unaligned memory access detected",
        error_code,
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64(),
    );
}

/// Get breakpoint count for diagnostics
pub fn get_breakpoint_count() -> u64 {
    BREAKPOINT_COUNT.load(Ordering::Relaxed)
}
