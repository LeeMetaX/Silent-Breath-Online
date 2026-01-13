//! Interrupt handling for i9-12900K
//!
//! IDT setup and exception handlers

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

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

        IDT.load();
    }
}

/// Breakpoint exception handler (INT 3)
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    // In a real implementation, print debug info
    let _ = stack_frame;
}

/// Double fault handler
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    let _ = stack_frame;
    let _ = error_code;
    panic!("DOUBLE FAULT");
}

/// General protection fault handler
extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    let _ = stack_frame;
    let _ = error_code;
    panic!("GENERAL PROTECTION FAULT - Error code: {:#x}", error_code);
}

/// Page fault handler
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: x86_64::structures::idt::PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    let faulting_address = Cr2::read();
    let _ = stack_frame;

    panic!(
        "PAGE FAULT - Address: {:?}, Error code: {:?}",
        faulting_address, error_code
    );
}

/// Invalid opcode handler
extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    let _ = stack_frame;
    panic!("INVALID OPCODE");
}

/// Device not available (no x87 FPU)
extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    let _ = stack_frame;
    panic!("DEVICE NOT AVAILABLE (FPU)");
}
