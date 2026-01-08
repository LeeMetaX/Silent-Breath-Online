/// Memory-Mapped I/O Register Interface
/// Direct hardware access for ROM/Firmware replacement

use core::ptr::{read_volatile, write_volatile};

/// MMIO Base Addresses (platform-specific)
pub const L3_CACHE_BASE: usize = 0x4000_0000;
pub const COHERENCY_CTL_BASE: usize = 0x4010_0000;
pub const CORE_STATUS_BASE: usize = 0x4020_0000;

/// MMIO Register for Cache Coherency Control
#[repr(C)]
pub struct CoherencyRegister {
    /// Control register: [3:0] = operation, [7:4] = core_id
    pub control: u32,
    /// Address register: target cache line address
    pub address: u64,
    /// Status register: [0] = busy, [1] = error, [7:4] = state
    pub status: u32,
    /// Data register: cache line payload
    pub data: [u32; 16],
}

impl CoherencyRegister {
    /// Read from MMIO register
    #[inline(always)]
    pub unsafe fn read_control(&self) -> u32 {
        read_volatile(&self.control as *const u32)
    }

    /// Write to MMIO register
    #[inline(always)]
    pub unsafe fn write_control(&mut self, value: u32) {
        write_volatile(&mut self.control as *mut u32, value);
    }

    /// Read cache line address
    #[inline(always)]
    pub unsafe fn read_address(&self) -> u64 {
        read_volatile(&self.address as *const u64)
    }

    /// Write cache line address
    #[inline(always)]
    pub unsafe fn write_address(&mut self, addr: u64) {
        write_volatile(&mut self.address as *mut u64, addr);
    }

    /// Check if operation is complete
    #[inline(always)]
    pub unsafe fn is_busy(&self) -> bool {
        (read_volatile(&self.status as *const u32) & 0x1) != 0
    }

    /// Get current cache state from hardware
    #[inline(always)]
    pub unsafe fn get_hw_state(&self) -> u8 {
        ((read_volatile(&self.status as *const u32) >> 4) & 0xF) as u8
    }
}

/// MMIO Operations for Cache Coherency
#[repr(u8)]
pub enum CoherencyOp {
    Read = 0x1,
    Write = 0x2,
    Invalidate = 0x3,
    Flush = 0x4,
}

/// Real-Time MMIO Accessor
pub struct MMIOCoherency {
    reg: *mut CoherencyRegister,
}

impl MMIOCoherency {
    /// Initialize MMIO interface
    pub const unsafe fn new(base_addr: usize) -> Self {
        Self {
            reg: base_addr as *mut CoherencyRegister,
        }
    }

    /// Execute cache read via MMIO (Step 1 & 2 from your flow)
    #[inline]
    pub unsafe fn mmio_cache_read(&mut self, core_id: u8, address: u64) -> Result<(), ()> {
        let reg = &mut *self.reg;

        // Write operation: read request from specific core
        let ctrl = (CoherencyOp::Read as u32) | ((core_id as u32) << 4);
        reg.write_control(ctrl);
        reg.write_address(address);

        // Spin until operation completes (real-time guarantee)
        while reg.is_busy() {
            core::hint::spin_loop();
        }

        Ok(())
    }

    /// Execute cache write via MMIO (Step 3 from your flow)
    #[inline]
    pub unsafe fn mmio_cache_write(&mut self, core_id: u8, address: u64) -> Result<(), ()> {
        let reg = &mut *self.reg;

        // Write operation: triggers invalidation broadcast
        let ctrl = (CoherencyOp::Write as u32) | ((core_id as u32) << 4);
        reg.write_control(ctrl);
        reg.write_address(address);

        // Spin until invalidation completes
        while reg.is_busy() {
            core::hint::spin_loop();
        }

        Ok(())
    }

    /// Invalidate cache line (Step 4 from your flow)
    #[inline]
    pub unsafe fn mmio_invalidate(&mut self, core_id: u8, address: u64) -> Result<(), ()> {
        let reg = &mut *self.reg;

        let ctrl = (CoherencyOp::Invalidate as u32) | ((core_id as u32) << 4);
        reg.write_control(ctrl);
        reg.write_address(address);

        // Real-time spin-wait
        while reg.is_busy() {
            core::hint::spin_loop();
        }

        Ok(())
    }

    /// Read current cache state from hardware
    #[inline(always)]
    pub unsafe fn read_hw_state(&self) -> u8 {
        (*self.reg).get_hw_state()
    }
}

unsafe impl Send for MMIOCoherency {}
unsafe impl Sync for MMIOCoherency {}
