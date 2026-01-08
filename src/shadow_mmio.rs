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
    pub unsafe fn mmio_get_state(&self, register_id: u8) -> Result<RegisterState, &'static str> {
        let mmio = &*self.mmio;
        Ok(mmio.get_state())
    }

    /// Get register version via MMIO
    #[inline]
    pub unsafe fn mmio_get_version(&self, register_id: u8) -> Result<u8, &'static str> {
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
