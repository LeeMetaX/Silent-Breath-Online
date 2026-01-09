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

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::boxed::Box;

    /// Helper: Create simulated MMIO register in memory
    fn create_mock_register() -> Box<CoherencyRegister> {
        Box::new(CoherencyRegister {
            control: 0,
            address: 0,
            status: 0,
            data: [0; 16],
        })
    }

    #[test]
    fn test_coherency_register_control_read_write() {
        let mut reg = create_mock_register();

        unsafe {
            // Write control value
            reg.write_control(0xDEADBEEF);

            // Read it back
            let read_val = reg.read_control();
            assert_eq!(read_val, 0xDEADBEEF);
        }
    }

    #[test]
    fn test_coherency_register_address_read_write() {
        let mut reg = create_mock_register();

        unsafe {
            // Write 64-bit address
            reg.write_address(0x1234_5678_9ABC_DEF0);

            // Read it back
            let read_addr = reg.read_address();
            assert_eq!(read_addr, 0x1234_5678_9ABC_DEF0);
        }
    }

    #[test]
    fn test_coherency_register_busy_flag() {
        let mut reg = create_mock_register();

        unsafe {
            // Initially not busy
            reg.status = 0;
            assert!(!reg.is_busy());

            // Set busy flag (bit 0)
            reg.status = 0x1;
            assert!(reg.is_busy());

            // Clear busy flag
            reg.status = 0x0;
            assert!(!reg.is_busy());
        }
    }

    #[test]
    fn test_coherency_register_hw_state() {
        let mut reg = create_mock_register();

        unsafe {
            // Set state bits [7:4] = 0b1010 = 10
            reg.status = 0xA0; // bits [7:4] = 1010
            assert_eq!(reg.get_hw_state(), 0xA);

            // Set state bits [7:4] = 0b0011 = 3
            reg.status = 0x30;
            assert_eq!(reg.get_hw_state(), 0x3);

            // Test with busy flag set (should not affect state)
            reg.status = 0x31; // state=3, busy=1
            assert_eq!(reg.get_hw_state(), 0x3);
        }
    }

    #[test]
    fn test_coherency_op_enum_values() {
        assert_eq!(CoherencyOp::Read as u8, 0x1);
        assert_eq!(CoherencyOp::Write as u8, 0x2);
        assert_eq!(CoherencyOp::Invalidate as u8, 0x3);
        assert_eq!(CoherencyOp::Flush as u8, 0x4);
    }

    #[test]
    fn test_mmio_coherency_initialization() {
        let reg = create_mock_register();
        let reg_ptr = Box::into_raw(reg);

        unsafe {
            let mmio = MMIOCoherency::new(reg_ptr as usize);
            assert_eq!(mmio.reg, reg_ptr);

            // Cleanup
            let _ = Box::from_raw(reg_ptr);
        }
    }

    #[test]
    fn test_mmio_cache_read_operation() {
        let reg = create_mock_register();
        let reg_ptr = Box::into_raw(reg);

        unsafe {
            let mut mmio = MMIOCoherency::new(reg_ptr as usize);

            // Execute cache read for core 3, address 0x1000
            // Note: This will succeed because status.busy is already 0
            let result = mmio.mmio_cache_read(3, 0x1000);
            assert!(result.is_ok());

            // Check that control register was set correctly
            // Format: [3:0] = operation (Read=1), [7:4] = core_id (3)
            let expected_ctrl = (CoherencyOp::Read as u32) | (3u32 << 4);
            let actual_ctrl = (*reg_ptr).read_control();
            assert_eq!(actual_ctrl, expected_ctrl);

            // Check that address was set correctly
            let actual_addr = (*reg_ptr).read_address();
            assert_eq!(actual_addr, 0x1000);

            // Cleanup
            let _ = Box::from_raw(reg_ptr);
        }
    }

    #[test]
    fn test_mmio_cache_write_operation() {
        let reg = create_mock_register();
        let reg_ptr = Box::into_raw(reg);

        unsafe {
            let mut mmio = MMIOCoherency::new(reg_ptr as usize);

            // Execute cache write for core 5, address 0x2000
            let result = mmio.mmio_cache_write(5, 0x2000);
            assert!(result.is_ok());

            // Check control: [3:0] = Write(2), [7:4] = core_id(5)
            let expected_ctrl = (CoherencyOp::Write as u32) | (5u32 << 4);
            let actual_ctrl = (*reg_ptr).read_control();
            assert_eq!(actual_ctrl, expected_ctrl);

            // Check address
            let actual_addr = (*reg_ptr).read_address();
            assert_eq!(actual_addr, 0x2000);

            // Cleanup
            let _ = Box::from_raw(reg_ptr);
        }
    }

    #[test]
    fn test_mmio_invalidate_operation() {
        let reg = create_mock_register();
        let reg_ptr = Box::into_raw(reg);

        unsafe {
            let mut mmio = MMIOCoherency::new(reg_ptr as usize);

            // Execute invalidate for core 7, address 0x3000
            let result = mmio.mmio_invalidate(7, 0x3000);
            assert!(result.is_ok());

            // Check control: [3:0] = Invalidate(3), [7:4] = core_id(7)
            let expected_ctrl = (CoherencyOp::Invalidate as u32) | (7u32 << 4);
            let actual_ctrl = (*reg_ptr).read_control();
            assert_eq!(actual_ctrl, expected_ctrl);

            // Check address
            let actual_addr = (*reg_ptr).read_address();
            assert_eq!(actual_addr, 0x3000);

            // Cleanup
            let _ = Box::from_raw(reg_ptr);
        }
    }

    #[test]
    fn test_mmio_read_hw_state() {
        let reg = create_mock_register();
        let reg_ptr = Box::into_raw(reg);

        unsafe {
            // Set hardware state to 0b1100 (12)
            (*reg_ptr).status = 0xC0; // bits [7:4] = 1100

            let mmio = MMIOCoherency::new(reg_ptr as usize);
            let hw_state = mmio.read_hw_state();

            assert_eq!(hw_state, 0xC);

            // Cleanup
            let _ = Box::from_raw(reg_ptr);
        }
    }

    #[test]
    fn test_mmio_control_register_format() {
        // Test control register packing format
        let core_id = 5u8;
        let operation = CoherencyOp::Write as u32;

        let ctrl = operation | ((core_id as u32) << 4);

        // Extract operation (bits [3:0])
        let extracted_op = ctrl & 0xF;
        assert_eq!(extracted_op, CoherencyOp::Write as u32);

        // Extract core_id (bits [7:4])
        let extracted_core = ((ctrl >> 4) & 0xF) as u8;
        assert_eq!(extracted_core, 5);
    }

    #[test]
    fn test_coherency_register_volatile_semantics() {
        let mut reg = create_mock_register();

        unsafe {
            // Write multiple times - each should be independent
            reg.write_control(0x1111);
            reg.write_control(0x2222);
            reg.write_control(0x3333);

            // Last write should be visible
            assert_eq!(reg.read_control(), 0x3333);
        }
    }

    #[test]
    fn test_mmio_status_register_fields() {
        let mut reg = create_mock_register();

        unsafe {
            // Test status register bit layout
            // bit [0] = busy
            // bit [1] = error (not tested here, but reserved)
            // bits [7:4] = state

            // Set all fields
            reg.status = 0xA1; // state=10 (1010), busy=1

            assert!(reg.is_busy());
            assert_eq!(reg.get_hw_state(), 0xA);

            // Clear busy, keep state
            reg.status = 0xA0;
            assert!(!reg.is_busy());
            assert_eq!(reg.get_hw_state(), 0xA);
        }
    }

    #[test]
    fn test_mmio_multiple_cores() {
        let reg = create_mock_register();
        let reg_ptr = Box::into_raw(reg);

        unsafe {
            let mut mmio = MMIOCoherency::new(reg_ptr as usize);

            // Core 0 reads
            mmio.mmio_cache_read(0, 0x1000).unwrap();
            let ctrl0 = (*reg_ptr).read_control();
            assert_eq!(ctrl0 & 0xF0, 0x00); // Core 0

            // Core 7 reads
            mmio.mmio_cache_read(7, 0x2000).unwrap();
            let ctrl7 = (*reg_ptr).read_control();
            assert_eq!(ctrl7 & 0xF0, 0x70); // Core 7

            // Cleanup
            let _ = Box::from_raw(reg_ptr);
        }
    }
}
