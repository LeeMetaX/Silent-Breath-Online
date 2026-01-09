/// MMIO Interface for Shadow Register System
/// Provides memory-mapped I/O access to shadow registers and fuses

use crate::fuse_manager::FuseManager;
use crate::shadow_register::{RegisterState, ShadowRegisterBank};
use crate::sync_manager::{SyncDirection, SyncManager, SyncPolicy};
use core::ptr::{read_volatile, write_volatile};

/// MMIO Base Addresses for Shadow Register System
pub const SHADOW_REG_BASE: usize = 0x5000_0000;
pub const FUSE_CTRL_BASE: usize = 0x5100_0000;
pub const SYNC_CTRL_BASE: usize = 0x5200_0000;

/// Shadow Register MMIO Control Register
#[repr(C)]
pub struct ShadowRegisterMMIO {
    /// Control register
    /// [7:0]   = Command
    /// [15:8]  = Register ID
    /// [23:16] = Status
    /// [31:24] = Reserved
    pub control: u32,

    /// Data register (64-bit value)
    pub data: u64,

    /// Address register (fuse address)
    pub address: u64,

    /// Status register
    /// [0]     = Busy
    /// [1]     = Error
    /// [2]     = Locked
    /// [7:3]   = State
    /// [15:8]  = Version
    /// [31:16] = Checksum
    pub status: u32,

    /// ECC register (8-bit parity)
    pub ecc: u32,
}

/// MMIO Commands
#[repr(u8)]
pub enum MMIOCommand {
    /// No operation
    Nop = 0x00,
    /// Read shadow register
    Read = 0x01,
    /// Write shadow register
    Write = 0x02,
    /// Commit shadow to active
    Commit = 0x03,
    /// Rollback to backup
    Rollback = 0x04,
    /// Lock register
    Lock = 0x05,
    /// Unlock register
    Unlock = 0x06,
    /// Verify checksum
    Verify = 0x07,
    /// Load from fuse
    LoadFuse = 0x08,
    /// Commit to fuse
    CommitFuse = 0x09,
    /// Sync operation
    Sync = 0x0A,
}

impl ShadowRegisterMMIO {
    /// Read control register
    #[inline(always)]
    pub unsafe fn read_control(&self) -> u32 {
        read_volatile(&self.control as *const u32)
    }

    /// Write control register
    #[inline(always)]
    pub unsafe fn write_control(&mut self, value: u32) {
        write_volatile(&mut self.control as *mut u32, value);
    }

    /// Read data register
    #[inline(always)]
    pub unsafe fn read_data(&self) -> u64 {
        read_volatile(&self.data as *const u64)
    }

    /// Write data register
    #[inline(always)]
    pub unsafe fn write_data(&mut self, value: u64) {
        write_volatile(&mut self.data as *mut u64, value);
    }

    /// Read status register
    #[inline(always)]
    pub unsafe fn read_status(&self) -> u32 {
        read_volatile(&self.status as *const u32)
    }

    /// Check if operation is busy
    #[inline(always)]
    pub unsafe fn is_busy(&self) -> bool {
        (self.read_status() & 0x1) != 0
    }

    /// Check if error occurred
    #[inline(always)]
    pub unsafe fn has_error(&self) -> bool {
        (self.read_status() & 0x2) != 0
    }

    /// Get register state from status
    #[inline(always)]
    pub unsafe fn get_state(&self) -> RegisterState {
        let status = self.read_status();
        let state_bits = ((status >> 3) & 0x1F) as u8;
        RegisterState::from(state_bits)
    }

    /// Get version from status
    #[inline(always)]
    pub unsafe fn get_version(&self) -> u8 {
        let status = self.read_status();
        ((status >> 8) & 0xFF) as u8
    }

    /// Execute a command and wait for completion
    #[inline]
    pub unsafe fn execute_command(
        &mut self,
        command: MMIOCommand,
        register_id: u8,
    ) -> Result<(), &'static str> {
        // Build control word
        let ctrl = (command as u32) | ((register_id as u32) << 8);

        // Write command
        self.write_control(ctrl);

        // Wait for completion (spin-wait for real-time guarantee)
        while self.is_busy() {
            core::hint::spin_loop();
        }

        // Check for errors
        if self.has_error() {
            return Err("MMIO command failed");
        }

        Ok(())
    }
}

/// Shadow Register MMIO Controller
pub struct ShadowMMIOController {
    /// MMIO register interface
    mmio: *mut ShadowRegisterMMIO,
    /// Shadow register bank (cached)
    shadow_bank: *mut ShadowRegisterBank,
    /// Fuse manager
    fuse_manager: *mut FuseManager,
    /// Sync manager
    sync_manager: SyncManager,
}

impl ShadowMMIOController {
    /// Create a new MMIO controller
    pub unsafe fn new(
        shadow_bank: *mut ShadowRegisterBank,
        fuse_manager: *mut FuseManager,
    ) -> Self {
        Self {
            mmio: SHADOW_REG_BASE as *mut ShadowRegisterMMIO,
            shadow_bank,
            fuse_manager,
            sync_manager: SyncManager::new(),
        }
    }

    /// Read shadow register via MMIO
    #[inline]
    pub unsafe fn mmio_read(&mut self, register_id: u8) -> Result<u64, &'static str> {
        let mmio = &mut *self.mmio;

        // Execute read command
        mmio.execute_command(MMIOCommand::Read, register_id)?;

        // Read data from MMIO
        Ok(mmio.read_data())
    }

    /// Write shadow register via MMIO
    #[inline]
    pub unsafe fn mmio_write(&mut self, register_id: u8, value: u64) -> Result<(), &'static str> {
        let mmio = &mut *self.mmio;

        // Write data to MMIO
        mmio.write_data(value);

        // Execute write command
        mmio.execute_command(MMIOCommand::Write, register_id)?;

        Ok(())
    }

    /// Commit shadow register via MMIO
    #[inline]
    pub unsafe fn mmio_commit(&mut self, register_id: u8) -> Result<(), &'static str> {
        let mmio = &mut *self.mmio;
        mmio.execute_command(MMIOCommand::Commit, register_id)?;
        Ok(())
    }

    /// Rollback shadow register via MMIO
    #[inline]
    pub unsafe fn mmio_rollback(&mut self, register_id: u8) -> Result<(), &'static str> {
        let mmio = &mut *self.mmio;
        mmio.execute_command(MMIOCommand::Rollback, register_id)?;
        Ok(())
    }

    /// Lock shadow register via MMIO
    #[inline]
    pub unsafe fn mmio_lock(&mut self, register_id: u8) -> Result<(), &'static str> {
        let mmio = &mut *self.mmio;
        mmio.execute_command(MMIOCommand::Lock, register_id)?;
        Ok(())
    }

    /// Unlock shadow register via MMIO
    #[inline]
    pub unsafe fn mmio_unlock(&mut self, register_id: u8) -> Result<(), &'static str> {
        let mmio = &mut *self.mmio;
        mmio.execute_command(MMIOCommand::Unlock, register_id)?;
        Ok(())
    }

    /// Verify shadow register checksum via MMIO
    #[inline]
    pub unsafe fn mmio_verify(&mut self, register_id: u8) -> Result<bool, &'static str> {
        let mmio = &mut *self.mmio;
        mmio.execute_command(MMIOCommand::Verify, register_id)?;

        // Check if verification passed (error bit should be clear)
        Ok(!mmio.has_error())
    }

    /// Load from fuse via MMIO
    #[inline]
    pub unsafe fn mmio_load_fuse(&mut self, register_id: u8) -> Result<(), &'static str> {
        let mmio = &mut *self.mmio;
        mmio.execute_command(MMIOCommand::LoadFuse, register_id)?;
        Ok(())
    }

    /// Commit to fuse via MMIO
    #[inline]
    pub unsafe fn mmio_commit_fuse(&mut self, register_id: u8) -> Result<(), &'static str> {
        let mmio = &mut *self.mmio;
        mmio.execute_command(MMIOCommand::CommitFuse, register_id)?;
        Ok(())
    }

    /// Synchronize shadow register via MMIO
    #[inline]
    pub unsafe fn mmio_sync(
        &mut self,
        register_id: u8,
        direction: SyncDirection,
        policy: SyncPolicy,
    ) -> Result<(), &'static str> {
        if self.fuse_manager.is_null() {
            return Err("Fuse manager not initialized");
        }

        // Use sync manager to perform sync
        self.sync_manager.sync_register(
            &mut *self.fuse_manager,
            register_id as u32,
            direction,
            policy,
        )
    }

    /// Get register state via MMIO
    #[inline]
    pub unsafe fn mmio_get_state(&self, _register_id: u8) -> Result<RegisterState, &'static str> {
        let mmio = &*self.mmio;
        Ok(mmio.get_state())
    }

    /// Get register version via MMIO
    #[inline]
    pub unsafe fn mmio_get_version(&self, _register_id: u8) -> Result<u8, &'static str> {
        let mmio = &*self.mmio;
        Ok(mmio.get_version())
    }

    /// Batch read multiple registers
    pub unsafe fn mmio_batch_read(&mut self, register_ids: &[u8]) -> Result<Vec<u64>, &'static str> {
        let mut values = Vec::with_capacity(register_ids.len());

        for &id in register_ids {
            let value = self.mmio_read(id)?;
            values.push(value);
        }

        Ok(values)
    }

    /// Batch write multiple registers
    pub unsafe fn mmio_batch_write(
        &mut self,
        operations: &[(u8, u64)],
    ) -> Result<(), &'static str> {
        for &(id, value) in operations {
            self.mmio_write(id, value)?;
        }

        Ok(())
    }

    /// Batch commit multiple registers
    pub unsafe fn mmio_batch_commit(&mut self, register_ids: &[u8]) -> Result<usize, &'static str> {
        let mut committed = 0;

        for &id in register_ids {
            if self.mmio_commit(id).is_ok() {
                committed += 1;
            }
        }

        Ok(committed)
    }
}

unsafe impl Send for ShadowMMIOController {}
unsafe impl Sync for ShadowMMIOController {}

// Vec implementation for no_std
extern crate alloc;
use alloc::vec::Vec;

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    /// Helper: Create simulated shadow MMIO register in memory
    fn create_mock_shadow_register() -> Box<ShadowRegisterMMIO> {
        Box::new(ShadowRegisterMMIO {
            control: 0,
            data: 0,
            address: 0,
            status: 0,
            ecc: 0,
        })
    }

    #[test]
    fn test_shadow_register_mmio_control_read_write() {
        let mut reg = create_mock_shadow_register();

        unsafe {
            reg.write_control(0x12345678);
            let val = reg.read_control();
            assert_eq!(val, 0x12345678);
        }
    }

    #[test]
    fn test_shadow_register_mmio_data_read_write() {
        let mut reg = create_mock_shadow_register();

        unsafe {
            reg.write_data(0xDEADBEEFCAFEBABE);
            let val = reg.read_data();
            assert_eq!(val, 0xDEADBEEFCAFEBABE);
        }
    }

    #[test]
    fn test_shadow_register_mmio_status_busy() {
        let mut reg = create_mock_shadow_register();

        unsafe {
            // Not busy initially
            reg.status = 0;
            assert!(!reg.is_busy());

            // Set busy bit
            reg.status = 0x1;
            assert!(reg.is_busy());

            // Clear busy bit
            reg.status = 0x0;
            assert!(!reg.is_busy());
        }
    }

    #[test]
    fn test_shadow_register_mmio_status_error() {
        let mut reg = create_mock_shadow_register();

        unsafe {
            // No error initially
            reg.status = 0;
            assert!(!reg.has_error());

            // Set error bit (bit 1)
            reg.status = 0x2;
            assert!(reg.has_error());

            // Both busy and error
            reg.status = 0x3;
            assert!(reg.is_busy());
            assert!(reg.has_error());
        }
    }

    #[test]
    fn test_shadow_register_mmio_get_state() {
        let mut reg = create_mock_shadow_register();

        unsafe {
            // Set state bits [7:3] = 0b00010 = RegisterState::Loaded (1)
            reg.status = 0b00001000; // bit 3 = 1
            let state = reg.get_state();
            assert_eq!(state, RegisterState::Loaded);

            // Set state to Modified (2)
            reg.status = 0b00010000; // bit 4 = 1
            let state = reg.get_state();
            assert_eq!(state, RegisterState::Modified);

            // Set state to Committed (3)
            reg.status = 0b00011000; // bits [4:3] = 11
            let state = reg.get_state();
            assert_eq!(state, RegisterState::Committed);
        }
    }

    #[test]
    fn test_shadow_register_mmio_get_version() {
        let mut reg = create_mock_shadow_register();

        unsafe {
            // Set version bits [15:8] = 42
            reg.status = 42 << 8;
            let version = reg.get_version();
            assert_eq!(version, 42);

            // Set version to 255
            reg.status = 255 << 8;
            let version = reg.get_version();
            assert_eq!(version, 255);
        }
    }

    #[test]
    fn test_mmio_command_enum_values() {
        assert_eq!(MMIOCommand::Nop as u8, 0x00);
        assert_eq!(MMIOCommand::Read as u8, 0x01);
        assert_eq!(MMIOCommand::Write as u8, 0x02);
        assert_eq!(MMIOCommand::Commit as u8, 0x03);
        assert_eq!(MMIOCommand::Rollback as u8, 0x04);
        assert_eq!(MMIOCommand::Lock as u8, 0x05);
        assert_eq!(MMIOCommand::Unlock as u8, 0x06);
        assert_eq!(MMIOCommand::Verify as u8, 0x07);
        assert_eq!(MMIOCommand::LoadFuse as u8, 0x08);
        assert_eq!(MMIOCommand::CommitFuse as u8, 0x09);
        assert_eq!(MMIOCommand::Sync as u8, 0x0A);
    }

    #[test]
    fn test_shadow_register_mmio_execute_command_success() {
        let mut reg = create_mock_shadow_register();

        unsafe {
            // Status is not busy, no error
            reg.status = 0;

            let result = reg.execute_command(MMIOCommand::Read, 5);
            assert!(result.is_ok());

            // Check control register format: [7:0] = command, [15:8] = register_id
            let ctrl = reg.read_control();
            let cmd = (ctrl & 0xFF) as u8;
            let reg_id = ((ctrl >> 8) & 0xFF) as u8;

            assert_eq!(cmd, MMIOCommand::Read as u8);
            assert_eq!(reg_id, 5);
        }
    }

    #[test]
    fn test_shadow_register_mmio_execute_command_error() {
        let mut reg = create_mock_shadow_register();

        unsafe {
            // Set error bit (bit 1)
            reg.status = 0x2;

            let result = reg.execute_command(MMIOCommand::Write, 10);
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "MMIO command failed");
        }
    }

    #[test]
    fn test_shadow_register_mmio_control_format() {
        // Test control register packing
        let command = MMIOCommand::Commit as u32;
        let register_id = 42u32;

        let ctrl = command | (register_id << 8);

        // Extract command (bits [7:0])
        let extracted_cmd = (ctrl & 0xFF) as u8;
        assert_eq!(extracted_cmd, MMIOCommand::Commit as u8);

        // Extract register_id (bits [15:8])
        let extracted_id = ((ctrl >> 8) & 0xFF) as u8;
        assert_eq!(extracted_id, 42);
    }

    #[test]
    fn test_shadow_register_mmio_status_layout() {
        let mut reg = create_mock_shadow_register();

        unsafe {
            // Complex status:
            // bit 0 = busy (1)
            // bit 1 = error (0)
            // bit 2 = locked (1)
            // bits [7:3] = state (0b00011 = Committed)
            // bits [15:8] = version (5)
            // bits [31:16] = checksum (0xABCD)

            let status: u32 =
                1 |                        // busy
                (1 << 2) |                 // locked
                (3 << 3) |                 // state = Committed
                (5 << 8) |                 // version
                (0xABCD << 16);            // checksum

            reg.status = status;

            assert!(reg.is_busy());
            assert!(!reg.has_error());
            assert_eq!(reg.get_state(), RegisterState::Committed);
            assert_eq!(reg.get_version(), 5);
        }
    }

    #[test]
    fn test_shadow_mmio_controller_initialization() {
        use crate::shadow_register::ShadowRegisterBank;
        use crate::fuse_manager::FuseManager;

        let shadow_bank = Box::new(ShadowRegisterBank::new());
        let fuse_manager = Box::new(FuseManager::new());

        let shadow_ptr = Box::into_raw(shadow_bank);
        let fuse_ptr = Box::into_raw(fuse_manager);

        unsafe {
            let controller = ShadowMMIOController::new(shadow_ptr, fuse_ptr);

            assert_eq!(controller.mmio as usize, SHADOW_REG_BASE);
            assert_eq!(controller.shadow_bank, shadow_ptr);
            assert_eq!(controller.fuse_manager, fuse_ptr);

            // Cleanup
            let _ = Box::from_raw(shadow_ptr);
            let _ = Box::from_raw(fuse_ptr);
        }
    }

    #[test]
    fn test_mmio_command_all_values() {
        // Ensure all 11 commands have unique values
        let commands = [
            MMIOCommand::Nop as u8,
            MMIOCommand::Read as u8,
            MMIOCommand::Write as u8,
            MMIOCommand::Commit as u8,
            MMIOCommand::Rollback as u8,
            MMIOCommand::Lock as u8,
            MMIOCommand::Unlock as u8,
            MMIOCommand::Verify as u8,
            MMIOCommand::LoadFuse as u8,
            MMIOCommand::CommitFuse as u8,
            MMIOCommand::Sync as u8,
        ];

        // Check all are sequential from 0x00 to 0x0A
        for (i, &cmd) in commands.iter().enumerate() {
            assert_eq!(cmd, i as u8);
        }
    }

    #[test]
    fn test_shadow_register_mmio_volatile_semantics() {
        let mut reg = create_mock_shadow_register();

        unsafe {
            // Multiple writes to data register
            reg.write_data(0x1111);
            reg.write_data(0x2222);
            reg.write_data(0x3333);

            // Last write should be visible
            assert_eq!(reg.read_data(), 0x3333);

            // Multiple writes to control register
            reg.write_control(0xAAAA);
            reg.write_control(0xBBBB);

            assert_eq!(reg.read_control(), 0xBBBB);
        }
    }

    #[test]
    fn test_shadow_register_mmio_state_extraction() {
        let mut reg = create_mock_shadow_register();

        unsafe {
            // Test all RegisterState values
            let states = [
                (RegisterState::Uninitialized, 0),
                (RegisterState::Loaded, 1),
                (RegisterState::Modified, 2),
                (RegisterState::Committed, 3),
                (RegisterState::Locked, 4),
            ];

            for (expected_state, state_val) in states {
                // Set state bits [7:3]
                reg.status = state_val << 3;
                let actual_state = reg.get_state();
                assert_eq!(actual_state, expected_state);
            }
        }
    }

    #[test]
    fn test_shadow_register_mmio_version_range() {
        let mut reg = create_mock_shadow_register();

        unsafe {
            // Test version boundaries
            for version in [0, 1, 42, 127, 255] {
                reg.status = (version as u32) << 8;
                let read_version = reg.get_version();
                assert_eq!(read_version, version);
            }
        }
    }
}
