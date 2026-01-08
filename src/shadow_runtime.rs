/// Shadow Register Runtime Integration
/// Complete runtime system combining all shadow register components

use crate::ecc_handler::{ECCManager, ECCStrategy};
use crate::fuse_manager::{FuseManager, FuseMode};
use crate::shadow_mmio::ShadowMMIOController;
use crate::shadow_register::ShadowRegisterBank;
use crate::sync_manager::{SyncDirection, SyncManager, SyncPolicy};
use crate::version_control::{get_timestamp, VersionedShadowRegister};

/// Shadow Register System Runtime
pub struct ShadowRegisterRuntime {
    /// Shadow register bank
    shadow_bank: ShadowRegisterBank,
    /// Fuse manager
    fuse_manager: FuseManager,
    /// Sync manager
    sync_manager: SyncManager,
    /// ECC manager
    ecc_manager: ECCManager,
    /// MMIO controller
    mmio_controller: Option<ShadowMMIOController>,
}

impl ShadowRegisterRuntime {
    /// Create a new shadow register runtime
    pub const fn new() -> Self {
        Self {
            shadow_bank: ShadowRegisterBank::new(),
            fuse_manager: FuseManager::new(),
            sync_manager: SyncManager::new(),
            ecc_manager: ECCManager::new(ECCStrategy::Hamming),
            mmio_controller: None,
        }
    }

    /// Initialize the runtime with MMIO support
    pub unsafe fn init(&mut self) {
        let shadow_ptr = &mut self.shadow_bank as *mut ShadowRegisterBank;
        let fuse_ptr = &mut self.fuse_manager as *mut FuseManager;

        self.mmio_controller = Some(ShadowMMIOController::new(shadow_ptr, fuse_ptr));
    }

    /// Register a new fuse-backed shadow register
    pub fn register_fuse(
        &mut self,
        register_id: u32,
        fuse_addr: u64,
        mode: FuseMode,
    ) -> Result<(), &'static str> {
        // Add fuse to manager
        self.fuse_manager.add_fuse(fuse_addr, mode)?;

        // Add corresponding shadow register
        self.shadow_bank.add_register(register_id, fuse_addr)?;

        Ok(())
    }

    /// Load all fuses into shadow registers
    pub unsafe fn load_from_fuses(&mut self) -> Result<usize, &'static str> {
        self.fuse_manager.load_all()
    }

    /// Commit all shadow registers to fuses
    pub unsafe fn commit_to_fuses(&mut self) -> Result<usize, &'static str> {
        self.fuse_manager.commit_all()
    }

    /// Read a shadow register
    pub fn read(&self, register_id: u32) -> Result<u64, &'static str> {
        if let Some(reg) = self.shadow_bank.get_register(register_id) {
            // Verify integrity
            if !reg.verify() {
                return Err("Register checksum verification failed");
            }

            Ok(reg.read())
        } else {
            Err("Register not found")
        }
    }

    /// Write to a shadow register
    pub fn write(&mut self, register_id: u32, value: u64) -> Result<(), &'static str> {
        if let Some(reg) = self.shadow_bank.get_register_mut(register_id) {
            // Encode with ECC
            let (_encoded_value, ecc) = self.ecc_manager.encode_u64(value);

            // Write to register
            reg.write(value)?;

            Ok(())
        } else {
            Err("Register not found")
        }
    }

    /// Commit a shadow register
    pub fn commit(&mut self, register_id: u32) -> Result<(), &'static str> {
        if let Some(reg) = self.shadow_bank.get_register_mut(register_id) {
            reg.commit()
        } else {
            Err("Register not found")
        }
    }

    /// Synchronize registers with fuses
    pub unsafe fn sync(
        &mut self,
        direction: SyncDirection,
        policy: SyncPolicy,
    ) -> Result<usize, &'static str> {
        let result = self
            .sync_manager
            .sync_all(&mut self.fuse_manager, direction, policy);

        Ok(result.synced_count)
    }

    /// Verify all registers
    pub fn verify_all(&self) -> bool {
        self.shadow_bank.verify_all()
    }

    /// Get ECC error statistics
    pub fn get_ecc_stats(&self) -> (u32, u32) {
        self.ecc_manager.get_total_errors()
    }

    /// Get shadow bank
    #[inline(always)]
    pub fn get_shadow_bank(&self) -> &ShadowRegisterBank {
        &self.shadow_bank
    }

    /// Get mutable shadow bank
    #[inline(always)]
    pub fn get_shadow_bank_mut(&mut self) -> &mut ShadowRegisterBank {
        &mut self.shadow_bank
    }

    /// Get fuse manager
    #[inline(always)]
    pub fn get_fuse_manager(&self) -> &FuseManager {
        &self.fuse_manager
    }

    /// Get mutable fuse manager
    #[inline(always)]
    pub fn get_fuse_manager_mut(&mut self) -> &mut FuseManager {
        &mut self.fuse_manager
    }

    /// Get MMIO controller
    #[inline(always)]
    pub fn get_mmio_controller(&self) -> Option<&ShadowMMIOController> {
        self.mmio_controller.as_ref()
    }

    /// Get mutable MMIO controller
    #[inline(always)]
    pub fn get_mmio_controller_mut(&mut self) -> Option<&mut ShadowMMIOController> {
        self.mmio_controller.as_mut()
    }
}

/// Complete system example with versioning
pub struct VersionedShadowRuntime {
    /// Versioned shadow registers
    registers: [VersionedShadowRegister; 64],
    /// Number of active registers
    count: usize,
    /// ECC manager
    ecc_manager: ECCManager,
}

impl VersionedShadowRuntime {
    /// Create a new versioned shadow runtime
    pub const fn new() -> Self {
        const INIT: VersionedShadowRegister = VersionedShadowRegister::new(0, 0);
        Self {
            registers: [INIT; 64],
            count: 0,
            ecc_manager: ECCManager::new(ECCStrategy::Hamming),
        }
    }

    /// Add a new versioned register
    pub fn add_register(&mut self, id: u32, fuse_addr: u64) -> Result<usize, &'static str> {
        if self.count >= 64 {
            return Err("Runtime is full");
        }

        let index = self.count;
        self.registers[index] = VersionedShadowRegister::new(id, fuse_addr);
        self.count += 1;

        Ok(index)
    }

    /// Write with versioning
    pub fn write_versioned(
        &mut self,
        index: usize,
        value: u64,
    ) -> Result<u32, &'static str> {
        if index >= self.count {
            return Err("Invalid register index");
        }

        let timestamp = get_timestamp();
        self.registers[index].write_versioned(value, timestamp)
    }

    /// Rollback to version
    pub fn rollback_to_version(
        &mut self,
        index: usize,
        version: u32,
    ) -> Result<(), &'static str> {
        if index >= self.count {
            return Err("Invalid register index");
        }

        self.registers[index].rollback_to_version(version)
    }

    /// Rollback by offset
    pub fn rollback_by_offset(&mut self, index: usize, offset: usize) -> Result<(), &'static str> {
        if index >= self.count {
            return Err("Invalid register index");
        }

        self.registers[index].rollback_by_offset(offset)
    }

    /// Get register
    pub fn get_register(&self, index: usize) -> Option<&VersionedShadowRegister> {
        if index < self.count {
            Some(&self.registers[index])
        } else {
            None
        }
    }

    /// Get mutable register
    pub fn get_register_mut(&mut self, index: usize) -> Option<&mut VersionedShadowRegister> {
        if index < self.count {
            Some(&mut self.registers[index])
        } else {
            None
        }
    }
}

/// FFI interface for C integration
#[no_mangle]
pub unsafe extern "C" fn shadow_runtime_init() -> *mut ShadowRegisterRuntime {
    let mut runtime = Box::leak(Box::new(ShadowRegisterRuntime::new()));
    runtime.init();
    runtime as *mut ShadowRegisterRuntime
}

#[no_mangle]
pub unsafe extern "C" fn shadow_runtime_register_fuse(
    runtime: *mut ShadowRegisterRuntime,
    register_id: u32,
    fuse_addr: u64,
    mode: u8,
) -> i32 {
    if runtime.is_null() {
        return -1;
    }

    let fuse_mode = match mode {
        0 => FuseMode::OTP,
        1 => FuseMode::MTP,
        2 => FuseMode::EEPROM,
        _ => return -1,
    };

    match (*runtime).register_fuse(register_id, fuse_addr, fuse_mode) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn shadow_runtime_read(
    runtime: *mut ShadowRegisterRuntime,
    register_id: u32,
    out_value: *mut u64,
) -> i32 {
    if runtime.is_null() || out_value.is_null() {
        return -1;
    }

    match (*runtime).read(register_id) {
        Ok(value) => {
            *out_value = value;
            0
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn shadow_runtime_write(
    runtime: *mut ShadowRegisterRuntime,
    register_id: u32,
    value: u64,
) -> i32 {
    if runtime.is_null() {
        return -1;
    }

    match (*runtime).write(register_id, value) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn shadow_runtime_commit(
    runtime: *mut ShadowRegisterRuntime,
    register_id: u32,
) -> i32 {
    if runtime.is_null() {
        return -1;
    }

    match (*runtime).commit(register_id) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn shadow_runtime_load_from_fuses(
    runtime: *mut ShadowRegisterRuntime,
) -> i32 {
    if runtime.is_null() {
        return -1;
    }

    match (*runtime).load_from_fuses() {
        Ok(count) => count as i32,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn shadow_runtime_commit_to_fuses(
    runtime: *mut ShadowRegisterRuntime,
) -> i32 {
    if runtime.is_null() {
        return -1;
    }

    match (*runtime).commit_to_fuses() {
        Ok(count) => count as i32,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn shadow_runtime_verify_all(runtime: *mut ShadowRegisterRuntime) -> i32 {
    if runtime.is_null() {
        return -1;
    }

    if (*runtime).verify_all() {
        0
    } else {
        -1
    }
}

extern crate alloc;
use alloc::boxed::Box;
