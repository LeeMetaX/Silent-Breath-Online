//! Silent-Breath-Online :: GRUB-bootable Linux-style stub.
//!
//! Entry from `boot.S` once the CPU is in 64-bit long mode. We have:
//!   - Identity-mapped first 1 GiB
//!   - 16 KiB boot stack
//!   - Interrupts disabled, no IDT installed
//!   - Multiboot2 info pointer in `multiboot_info_phys`
//!
//! This stub prints a banner to VGA text mode and the COM1 serial port,
//! then halts. It is the launching point for the rest of the kernel.

#![no_std]
#![no_main]

use core::arch::asm;
use core::fmt::{self, Write};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU8, Ordering};

const COM1: u16 = 0x3F8;
const VGA_BUFFER: *mut u16 = 0xb8000 as *mut u16;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;

#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info_phys: u64) -> ! {
    unsafe { serial_init() };

    let mut con = Console::new();
    let _ = writeln!(con, "Silent-Breath GRUB stub: long mode reached");
    let _ = writeln!(con, "Multiboot2 info @ {:#018x}", multiboot_info_phys);
    let _ = writeln!(con, "CR3 = {:#018x}", read_cr3());
    let _ = writeln!(con, "Identity map: 0..1 GiB (2 MiB pages)");
    let _ = writeln!(con, "Stub halting. Replace `kernel_main` to extend.");

    halt_forever();
}

// ---------------------------------------------------------------------------
// Console: writes to both VGA text buffer and COM1 serial.
// ---------------------------------------------------------------------------

struct Console {
    vga_row: usize,
    vga_col: usize,
}

impl Console {
    const fn new() -> Self {
        Self { vga_row: 0, vga_col: 0 }
    }

    fn put_byte(&mut self, byte: u8) {
        unsafe { serial_write_byte(byte) };
        self.vga_put_byte(byte);
    }

    fn vga_put_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.vga_newline(),
            b'\r' => self.vga_col = 0,
            _ => {
                if self.vga_col >= VGA_WIDTH {
                    self.vga_newline();
                }
                let entry = 0x0f00u16 | (byte as u16);
                let idx = self.vga_row * VGA_WIDTH + self.vga_col;
                unsafe { VGA_BUFFER.add(idx).write_volatile(entry) };
                self.vga_col += 1;
            }
        }
    }

    fn vga_newline(&mut self) {
        self.vga_col = 0;
        if self.vga_row + 1 < VGA_HEIGHT {
            self.vga_row += 1;
        } else {
            self.vga_scroll();
        }
    }

    fn vga_scroll(&mut self) {
        for row in 1..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                unsafe {
                    let src = VGA_BUFFER.add(row * VGA_WIDTH + col).read_volatile();
                    VGA_BUFFER.add((row - 1) * VGA_WIDTH + col).write_volatile(src);
                }
            }
        }
        for col in 0..VGA_WIDTH {
            unsafe {
                VGA_BUFFER
                    .add((VGA_HEIGHT - 1) * VGA_WIDTH + col)
                    .write_volatile(0x0f20);
            }
        }
        self.vga_col = 0;
    }
}

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.put_byte(byte);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Serial port (COM1, 38400 8N1).
// ---------------------------------------------------------------------------

static SERIAL_READY: AtomicU8 = AtomicU8::new(0);

unsafe fn serial_init() {
    if SERIAL_READY.swap(1, Ordering::AcqRel) != 0 {
        return;
    }
    outb(COM1 + 1, 0x00); // disable interrupts
    outb(COM1 + 3, 0x80); // enable DLAB
    outb(COM1, 0x03);     // divisor low (38400 baud)
    outb(COM1 + 1, 0x00); // divisor high
    outb(COM1 + 3, 0x03); // 8 bits, no parity, one stop
    outb(COM1 + 2, 0xC7); // FIFO, clear, 14-byte threshold
    outb(COM1 + 4, 0x0B); // IRQs enabled, RTS/DSR set
}

unsafe fn serial_write_byte(byte: u8) {
    while (inb(COM1 + 5) & 0x20) == 0 {}
    outb(COM1, byte);
    if byte == b'\n' {
        while (inb(COM1 + 5) & 0x20) == 0 {}
        outb(COM1, b'\r');
    }
}

#[inline(always)]
unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

#[inline(always)]
unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

// ---------------------------------------------------------------------------
// Misc helpers.
// ---------------------------------------------------------------------------

fn read_cr3() -> u64 {
    let value: u64;
    unsafe { asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags)) };
    value
}

fn halt_forever() -> ! {
    loop {
        unsafe { asm!("cli; hlt", options(nomem, nostack)) };
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut con = Console::new();
    let _ = writeln!(con, "\n!!! KERNEL PANIC: {info}");
    halt_forever()
}
