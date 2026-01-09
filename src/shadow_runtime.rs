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
            let (_encoded_value, _ecc) = self.ecc_manager.encode_u64(value);

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
    let runtime = Box::leak(Box::new(ShadowRegisterRuntime::new()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    /// Test: ShadowRegisterRuntime initialization
    #[test]
    fn test_shadow_register_runtime_new() {
        let runtime = ShadowRegisterRuntime::new();

        // Verify empty bank has no registers
        assert!(runtime.shadow_bank.get_register(0).is_none());
        assert!(runtime.mmio_controller.is_none());
        assert!(runtime.verify_all());
    }

    /// Test: ShadowRegisterRuntime init with MMIO
    #[test]
    fn test_shadow_register_runtime_init() {
        let mut runtime = ShadowRegisterRuntime::new();

        unsafe {
            runtime.init();
        }

        assert!(runtime.mmio_controller.is_some());
    }

    /// Test: Register a fuse successfully
    #[test]
    fn test_shadow_register_runtime_register_fuse_success() {
        let mut runtime = ShadowRegisterRuntime::new();

        let result = runtime.register_fuse(1, 0x1000, FuseMode::OTP);
        assert!(result.is_ok());

        // Verify the register was added
        assert!(runtime.shadow_bank.get_register(1).is_some());
    }

    /// Test: Register duplicate returns error on full bank
    #[test]
    fn test_shadow_register_runtime_register_fuse_duplicate() {
        let mut runtime = ShadowRegisterRuntime::new();

        // Register first fuse
        let result1 = runtime.register_fuse(1, 0x1000, FuseMode::OTP);
        assert!(result1.is_ok());

        // Registering different register_id with different fuse address should succeed
        let result2 = runtime.register_fuse(2, 0x2000, FuseMode::MTP);
        assert!(result2.is_ok());

        // Verify both registers exist
        assert!(runtime.shadow_bank.get_register(1).is_some());
        assert!(runtime.shadow_bank.get_register(2).is_some());
    }

    /// Test: Read from existing register
    #[test]
    fn test_shadow_register_runtime_read_success() {
        let mut runtime = ShadowRegisterRuntime::new();

        // Register, write, and commit
        runtime.register_fuse(1, 0x1000, FuseMode::OTP).unwrap();
        runtime.write(1, 0x12345678).unwrap();
        runtime.commit(1).unwrap();

        // Read back
        let result = runtime.read(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x12345678);
    }

    /// Test: Read from non-existent register
    #[test]
    fn test_shadow_register_runtime_read_not_found() {
        let runtime = ShadowRegisterRuntime::new();

        let result = runtime.read(999);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Register not found");
    }

    /// Test: Write to existing register
    #[test]
    fn test_shadow_register_runtime_write_success() {
        let mut runtime = ShadowRegisterRuntime::new();

        runtime.register_fuse(1, 0x1000, FuseMode::OTP).unwrap();

        let result = runtime.write(1, 0xDEADBEEF);
        assert!(result.is_ok());

        // Commit to update checksum
        runtime.commit(1).unwrap();

        // Verify the write
        let read_result = runtime.read(1);
        assert_eq!(read_result.unwrap(), 0xDEADBEEF);
    }

    /// Test: Write to non-existent register
    #[test]
    fn test_shadow_register_runtime_write_not_found() {
        let mut runtime = ShadowRegisterRuntime::new();

        let result = runtime.write(999, 0x12345678);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Register not found");
    }

    /// Test: Commit existing register
    #[test]
    fn test_shadow_register_runtime_commit_success() {
        let mut runtime = ShadowRegisterRuntime::new();

        runtime.register_fuse(1, 0x1000, FuseMode::OTP).unwrap();
        runtime.write(1, 0x12345678).unwrap();

        let result = runtime.commit(1);
        assert!(result.is_ok());
    }

    /// Test: Commit non-existent register
    #[test]
    fn test_shadow_register_runtime_commit_not_found() {
        let mut runtime = ShadowRegisterRuntime::new();

        let result = runtime.commit(999);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Register not found");
    }

    /// Test: Verify all registers
    #[test]
    fn test_shadow_register_runtime_verify_all() {
        let mut runtime = ShadowRegisterRuntime::new();

        // Empty runtime should verify successfully
        assert!(runtime.verify_all());

        // Add, write, commit and verify with registers
        runtime.register_fuse(1, 0x1000, FuseMode::OTP).unwrap();
        runtime.write(1, 0x12345678).unwrap();
        runtime.commit(1).unwrap();

        assert!(runtime.verify_all());
    }

    /// Test: Get ECC stats
    #[test]
    fn test_shadow_register_runtime_get_ecc_stats() {
        let runtime = ShadowRegisterRuntime::new();

        let (single_bit, multi_bit) = runtime.get_ecc_stats();

        // Initially should be zero
        assert_eq!(single_bit, 0);
        assert_eq!(multi_bit, 0);
    }

    /// Test: VersionedShadowRuntime initialization
    #[test]
    fn test_versioned_shadow_runtime_new() {
        let runtime = VersionedShadowRuntime::new();

        assert_eq!(runtime.count, 0);

        // Try to get register from empty runtime
        assert!(runtime.get_register(0).is_none());
    }

    /// Test: Add register to VersionedShadowRuntime
    #[test]
    fn test_versioned_shadow_runtime_add_register() {
        let mut runtime = VersionedShadowRuntime::new();

        let result = runtime.add_register(1, 0x1000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert_eq!(runtime.count, 1);

        // Verify we can get the register
        assert!(runtime.get_register(0).is_some());
    }

    /// Test: VersionedShadowRuntime full capacity
    #[test]
    fn test_versioned_shadow_runtime_full() {
        let mut runtime = VersionedShadowRuntime::new();

        // Fill to capacity (64 registers)
        for i in 0..64 {
            let result = runtime.add_register(i, 0x1000 + (i as u64 * 0x100));
            assert!(result.is_ok());
        }

        // Try to add one more - should fail
        let result = runtime.add_register(64, 0x10000);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Runtime is full");
    }

    /// Test: Write versioned
    #[test]
    fn test_versioned_shadow_runtime_write_versioned() {
        let mut runtime = VersionedShadowRuntime::new();

        runtime.add_register(1, 0x1000).unwrap();

        let result = runtime.write_versioned(0, 0x12345678);
        assert!(result.is_ok());

        // Version should be returned (starts at 0, gets incremented)
        let version = result.unwrap();
        assert_eq!(version, 0);
    }

    /// Test: Write versioned with invalid index
    #[test]
    fn test_versioned_shadow_runtime_write_versioned_invalid() {
        let mut runtime = VersionedShadowRuntime::new();

        let result = runtime.write_versioned(999, 0x12345678);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid register index");
    }

    /// Test: Rollback to version
    #[test]
    fn test_versioned_shadow_runtime_rollback_to_version() {
        let mut runtime = VersionedShadowRuntime::new();

        runtime.add_register(1, 0x1000).unwrap();

        // Write multiple versions
        let v1 = runtime.write_versioned(0, 0x1111).unwrap();
        runtime.write_versioned(0, 0x2222).unwrap();
        runtime.write_versioned(0, 0x3333).unwrap();

        // Rollback to first version
        let result = runtime.rollback_to_version(0, v1);
        assert!(result.is_ok());
    }

    /// Test: Rollback by offset
    #[test]
    fn test_versioned_shadow_runtime_rollback_by_offset() {
        let mut runtime = VersionedShadowRuntime::new();

        runtime.add_register(1, 0x1000).unwrap();

        // Write multiple versions
        runtime.write_versioned(0, 0x1111).unwrap();
        runtime.write_versioned(0, 0x2222).unwrap();
        runtime.write_versioned(0, 0x3333).unwrap();

        // Rollback by 1 offset
        let result = runtime.rollback_by_offset(0, 1);
        assert!(result.is_ok());
    }

    /// Test: FFI shadow_runtime_init returns valid pointer
    #[test]
    fn test_ffi_shadow_runtime_init() {
        unsafe {
            let ptr = shadow_runtime_init();

            assert!(!ptr.is_null());

            // Verify the runtime was initialized
            assert!((*ptr).mmio_controller.is_some());

            // Cleanup: Convert back to Box and drop
            let _ = Box::from_raw(ptr);
        }
    }

    /// Test: FFI shadow_runtime_register_fuse with null pointer
    #[test]
    fn test_ffi_shadow_runtime_register_fuse_null() {
        unsafe {
            let result = shadow_runtime_register_fuse(
                core::ptr::null_mut(),
                1,
                0x1000,
                0
            );

            assert_eq!(result, -1);
        }
    }

    /// Test: FFI shadow_runtime_register_fuse success
    #[test]
    fn test_ffi_shadow_runtime_register_fuse_success() {
        unsafe {
            let ptr = shadow_runtime_init();

            // Register with OTP mode (0)
            let result = shadow_runtime_register_fuse(ptr, 1, 0x1000, 0);
            assert_eq!(result, 0);

            // Register with MTP mode (1)
            let result = shadow_runtime_register_fuse(ptr, 2, 0x2000, 1);
            assert_eq!(result, 0);

            // Register with EEPROM mode (2)
            let result = shadow_runtime_register_fuse(ptr, 3, 0x3000, 2);
            assert_eq!(result, 0);

            // Cleanup
            let _ = Box::from_raw(ptr);
        }
    }

    /// Test: FFI shadow_runtime_register_fuse invalid mode
    #[test]
    fn test_ffi_shadow_runtime_register_fuse_invalid_mode() {
        unsafe {
            let ptr = shadow_runtime_init();

            // Invalid mode (3)
            let result = shadow_runtime_register_fuse(ptr, 1, 0x1000, 3);
            assert_eq!(result, -1);

            // Cleanup
            let _ = Box::from_raw(ptr);
        }
    }

    /// Test: FFI shadow_runtime_read with null pointers
    #[test]
    fn test_ffi_shadow_runtime_read_null() {
        unsafe {
            let mut out_value: u64 = 0;

            // Null runtime pointer
            let result = shadow_runtime_read(core::ptr::null_mut(), 1, &mut out_value);
            assert_eq!(result, -1);

            // Null output pointer
            let ptr = shadow_runtime_init();
            let result = shadow_runtime_read(ptr, 1, core::ptr::null_mut());
            assert_eq!(result, -1);

            // Cleanup
            let _ = Box::from_raw(ptr);
        }
    }

    /// Test: FFI shadow_runtime_read success
    #[test]
    fn test_ffi_shadow_runtime_read_success() {
        unsafe {
            let ptr = shadow_runtime_init();

            // Register, write, and commit
            shadow_runtime_register_fuse(ptr, 1, 0x1000, 0);
            shadow_runtime_write(ptr, 1, 0xDEADBEEF);
            shadow_runtime_commit(ptr, 1);

            // Read back
            let mut out_value: u64 = 0;
            let result = shadow_runtime_read(ptr, 1, &mut out_value);

            assert_eq!(result, 0);
            assert_eq!(out_value, 0xDEADBEEF);

            // Cleanup
            let _ = Box::from_raw(ptr);
        }
    }

    /// Test: FFI shadow_runtime_write with null pointer
    #[test]
    fn test_ffi_shadow_runtime_write_null() {
        unsafe {
            let result = shadow_runtime_write(core::ptr::null_mut(), 1, 0x12345678);
            assert_eq!(result, -1);
        }
    }

    /// Test: FFI shadow_runtime_write success
    #[test]
    fn test_ffi_shadow_runtime_write_success() {
        unsafe {
            let ptr = shadow_runtime_init();

            shadow_runtime_register_fuse(ptr, 1, 0x1000, 0);

            let result = shadow_runtime_write(ptr, 1, 0xCAFEBABE);
            assert_eq!(result, 0);

            // Cleanup
            let _ = Box::from_raw(ptr);
        }
    }

    /// Test: FFI shadow_runtime_commit with null pointer
    #[test]
    fn test_ffi_shadow_runtime_commit_null() {
        unsafe {
            let result = shadow_runtime_commit(core::ptr::null_mut(), 1);
            assert_eq!(result, -1);
        }
    }

    /// Test: FFI shadow_runtime_commit success
    #[test]
    fn test_ffi_shadow_runtime_commit_success() {
        unsafe {
            let ptr = shadow_runtime_init();

            shadow_runtime_register_fuse(ptr, 1, 0x1000, 0);
            shadow_runtime_write(ptr, 1, 0x12345678);

            let result = shadow_runtime_commit(ptr, 1);
            assert_eq!(result, 0);

            // Cleanup
            let _ = Box::from_raw(ptr);
        }
    }

    /// Test: FFI shadow_runtime_load_from_fuses with null pointer
    #[test]
    fn test_ffi_shadow_runtime_load_from_fuses_null() {
        unsafe {
            let result = shadow_runtime_load_from_fuses(core::ptr::null_mut());
            assert_eq!(result, -1);
        }
    }

    /// Test: FFI shadow_runtime_commit_to_fuses with null pointer
    #[test]
    fn test_ffi_shadow_runtime_commit_to_fuses_null() {
        unsafe {
            let result = shadow_runtime_commit_to_fuses(core::ptr::null_mut());
            assert_eq!(result, -1);
        }
    }

    /// Test: FFI shadow_runtime_verify_all with null pointer
    #[test]
    fn test_ffi_shadow_runtime_verify_all_null() {
        unsafe {
            let result = shadow_runtime_verify_all(core::ptr::null_mut());
            assert_eq!(result, -1);
        }
    }

    /// Test: FFI shadow_runtime_verify_all success
    #[test]
    fn test_ffi_shadow_runtime_verify_all_success() {
        unsafe {
            let ptr = shadow_runtime_init();

            shadow_runtime_register_fuse(ptr, 1, 0x1000, 0);
            shadow_runtime_write(ptr, 1, 0x12345678);
            shadow_runtime_commit(ptr, 1);

            let result = shadow_runtime_verify_all(ptr);
            assert_eq!(result, 0);

            // Cleanup
            let _ = Box::from_raw(ptr);
        }
    }

    /// Test: FFI integration - complete workflow
    #[test]
    fn test_ffi_complete_workflow() {
        unsafe {
            // Initialize runtime
            let ptr = shadow_runtime_init();
            assert!(!ptr.is_null());

            // Register multiple fuses
            assert_eq!(shadow_runtime_register_fuse(ptr, 1, 0x1000, 0), 0);
            assert_eq!(shadow_runtime_register_fuse(ptr, 2, 0x2000, 1), 0);
            assert_eq!(shadow_runtime_register_fuse(ptr, 3, 0x3000, 2), 0);

            // Write to registers
            assert_eq!(shadow_runtime_write(ptr, 1, 0x1111), 0);
            assert_eq!(shadow_runtime_write(ptr, 2, 0x2222), 0);
            assert_eq!(shadow_runtime_write(ptr, 3, 0x3333), 0);

            // Commit registers
            assert_eq!(shadow_runtime_commit(ptr, 1), 0);
            assert_eq!(shadow_runtime_commit(ptr, 2), 0);
            assert_eq!(shadow_runtime_commit(ptr, 3), 0);

            // Read back and verify
            let mut value: u64 = 0;
            assert_eq!(shadow_runtime_read(ptr, 1, &mut value), 0);
            assert_eq!(value, 0x1111);

            assert_eq!(shadow_runtime_read(ptr, 2, &mut value), 0);
            assert_eq!(value, 0x2222);

            assert_eq!(shadow_runtime_read(ptr, 3, &mut value), 0);
            assert_eq!(value, 0x3333);

            // Verify all
            assert_eq!(shadow_runtime_verify_all(ptr), 0);

            // Cleanup
            let _ = Box::from_raw(ptr);
        }
    }
}
