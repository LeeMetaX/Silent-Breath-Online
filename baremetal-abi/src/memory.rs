//! Memory management for i9-12900K bare-metal
//!
//! Physical and virtual memory management

use bootloader_api::BootInfo;

/// Physical memory layout for i9-12900K
pub mod layout {
    /// L3 Cache MMIO base
    pub const L3_CACHE_BASE: u64 = 0xFFFF_9000_4000_0000;
    /// Coherency control MMIO base
    pub const COHERENCY_CTRL_BASE: u64 = 0xFFFF_9000_4010_0000;
    /// Shadow registers MMIO base
    pub const SHADOW_REG_BASE: u64 = 0xFFFF_9000_5000_0000;
    /// Hardware fuses MMIO base
    pub const FUSE_BASE: u64 = 0xFFFF_9000_6000_0000;

    /// Kernel code start
    pub const KERNEL_CODE_START: u64 = 0xFFFF_FFFF_8000_0000;
    /// Kernel heap start
    pub const KERNEL_HEAP_START: u64 = 0xFFFF_8800_0000_0000;
    /// Kernel heap size (1 GiB)
    pub const KERNEL_HEAP_SIZE: u64 = 1024 * 1024 * 1024;
}

/// Initialize memory management
pub fn init(boot_info: &'static mut BootInfo) {
    // In a real implementation:
    // 1. Parse memory map from boot_info
    // 2. Initialize physical frame allocator
    // 3. Set up kernel heap
    // 4. Map MMIO regions
    let _ = boot_info;
}

/// Physical address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(pub u64);

/// Virtual address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(pub u64);

impl PhysAddr {
    /// Create a new physical address
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the inner value
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl VirtAddr {
    /// Create a new virtual address
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the inner value
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Check if address is canonical
    pub fn is_canonical(self) -> bool {
        let addr = self.0;
        // Canonical addresses have bits 48-63 equal to bit 47
        let bit_47 = (addr >> 47) & 1;
        let upper_bits = addr >> 48;

        if bit_47 == 1 {
            upper_bits == 0xFFFF
        } else {
            upper_bits == 0
        }
    }
}

/// Page size (4 KiB)
pub const PAGE_SIZE: u64 = 4096;

/// Huge page size (2 MiB)
pub const HUGE_PAGE_SIZE: u64 = 2 * 1024 * 1024;

/// Align address down to page boundary
pub const fn align_down(addr: u64, align: u64) -> u64 {
    addr & !(align - 1)
}

/// Align address up to page boundary
pub const fn align_up(addr: u64, align: u64) -> u64 {
    let mask = align - 1;
    if addr & mask == 0 {
        addr
    } else {
        (addr | mask) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_down() {
        assert_eq!(align_down(0x1234, PAGE_SIZE), 0x1000);
        assert_eq!(align_down(0x5000, PAGE_SIZE), 0x5000);
    }

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0x1234, PAGE_SIZE), 0x2000);
        assert_eq!(align_up(0x5000, PAGE_SIZE), 0x5000);
    }

    #[test]
    fn test_canonical_addresses() {
        // Low canonical range
        assert!(VirtAddr::new(0x0000_0000_0000_0000).is_canonical());
        assert!(VirtAddr::new(0x0000_7FFF_FFFF_FFFF).is_canonical());

        // High canonical range
        assert!(VirtAddr::new(0xFFFF_8000_0000_0000).is_canonical());
        assert!(VirtAddr::new(0xFFFF_FFFF_FFFF_FFFF).is_canonical());

        // Non-canonical (gap)
        assert!(!VirtAddr::new(0x0000_8000_0000_0000).is_canonical());
        assert!(!VirtAddr::new(0xFFFF_7FFF_FFFF_FFFF).is_canonical());
    }
}
