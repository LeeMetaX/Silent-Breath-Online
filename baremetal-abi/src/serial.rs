//! Shared COM1 serial driver for the library boot path and kernel binaries
//!
//! Both `boot` and the kernel binaries used to carry their own copy of this
//! UART setup sequence; this module is the single implementation.

use core::fmt;
use x86_64::instructions::port::Port;

const SERIAL_IO_PORT: u16 = 0x3F8; // COM1

/// Bound on the transmit-ready wait so a wedged UART cannot hang the kernel
const TX_READY_SPIN_LIMIT: u32 = 100_000;

/// Zero-sized writer over COM1, usable with `core::fmt::Write`
pub struct SerialPort;

/// Initialize COM1: 38400 baud, 8N1, FIFO enabled
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

/// Write one byte, waiting (bounded) for the transmit holding register
pub fn write_byte(byte: u8) {
    unsafe {
        let mut line_status = Port::<u8>::new(SERIAL_IO_PORT + 5);
        let mut spins = 0u32;
        while line_status.read() & 0x20 == 0 {
            if spins >= TX_READY_SPIN_LIMIT {
                break; // UART wedged: drop rather than hang the kernel
            }
            spins += 1;
            core::hint::spin_loop();
        }

        let mut data = Port::<u8>::new(SERIAL_IO_PORT);
        data.write(byte);
    }
}

/// Write a string followed by a newline
pub fn write_line(s: &str) {
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
