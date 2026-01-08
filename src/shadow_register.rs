/// Shadow Register Management System
/// Comprehensive hardware fuse shadow register implementation

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Register State - tracks the lifecycle of a shadow register
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RegisterState {
    /// Register is uninitialized
    Uninitialized = 0x00,
    /// Register is loaded from fuse
    Loaded = 0x01,
    /// Register has been modified but not committed
    Modified = 0x02,
    /// Register is committed to hardware
    Committed = 0x03,
    /// Register is locked (cannot be modified)
    Locked = 0x04,
    /// Register has detected error
    Error = 0xFF,
}

impl From<u8> for RegisterState {
    fn from(val: u8) -> Self {
        match val {
            0x00 => RegisterState::Uninitialized,
            0x01 => RegisterState::Loaded,
            0x02 => RegisterState::Modified,
            0x03 => RegisterState::Committed,
            0x04 => RegisterState::Locked,
            _ => RegisterState::Error,
        }
    }
}

/// Shadow Register - holds a copy of hardware fuse data
#[repr(C, align(64))]
pub struct ShadowRegister {
    /// Register ID (unique identifier)
    id: u32,
    /// Current register value
    value: AtomicU64,
    /// Shadow copy for atomic updates
    shadow_value: AtomicU64,
    /// Register state
    state: AtomicU32,
    /// Version counter for rollback
    version: AtomicU32,
    /// CRC32 checksum for error detection
    checksum: AtomicU32,
    /// Physical fuse address
    fuse_addr: u64,
    /// Write protection flag
    write_protected: bool,
    /// Backup value for rollback
    backup_value: u64,
}

impl ShadowRegister {
    /// Create a new shadow register
    pub const fn new(id: u32, fuse_addr: u64) -> Self {
        Self {
            id,
            value: AtomicU64::new(0),
            shadow_value: AtomicU64::new(0),
            state: AtomicU32::new(RegisterState::Uninitialized as u32),
            version: AtomicU32::new(0),
            checksum: AtomicU32::new(0),
            fuse_addr,
            write_protected: false,
            backup_value: 0,
        }
    }

    /// Get current register value
    #[inline(always)]
    pub fn read(&self) -> u64 {
        self.value.load(Ordering::Acquire)
    }

    /// Write to shadow register (staged write)
    #[inline]
    pub fn write(&self, new_value: u64) -> Result<(), &'static str> {
        // Check write protection
        if self.write_protected {
            return Err("Register is write-protected");
        }

        // Check if locked
        let current_state = RegisterState::from(self.state.load(Ordering::Acquire) as u8);
        if current_state == RegisterState::Locked {
            return Err("Register is locked");
        }

        // Write to shadow value
        self.shadow_value.store(new_value, Ordering::Release);

        // Update state to Modified
        self.state.store(RegisterState::Modified as u32, Ordering::Release);

        // Increment version
        self.version.fetch_add(1, Ordering::AcqRel);

        Ok(())
    }

    /// Commit shadow value to active register
    #[inline]
    pub fn commit(&mut self) -> Result<(), &'static str> {
        let current_state = RegisterState::from(self.state.load(Ordering::Acquire) as u8);

        if current_state != RegisterState::Modified {
            return Err("No pending changes to commit");
        }

        // Backup current value for rollback
        self.backup_value = self.value.load(Ordering::Acquire);

        // Atomic commit
        let shadow_val = self.shadow_value.load(Ordering::Acquire);
        self.value.store(shadow_val, Ordering::Release);

        // Update checksum
        let crc = self.calculate_crc32(shadow_val);
        self.checksum.store(crc, Ordering::Release);

        // Update state
        self.state.store(RegisterState::Committed as u32, Ordering::Release);

        Ok(())
    }

    /// Rollback to previous value
    #[inline]
    pub fn rollback(&mut self) -> Result<(), &'static str> {
        // Restore backup value
        self.value.store(self.backup_value, Ordering::Release);
        self.shadow_value.store(self.backup_value, Ordering::Release);

        // Recalculate checksum
        let crc = self.calculate_crc32(self.backup_value);
        self.checksum.store(crc, Ordering::Release);

        // Decrement version
        self.version.fetch_sub(1, Ordering::AcqRel);

        // Update state
        self.state.store(RegisterState::Committed as u32, Ordering::Release);

        Ok(())
    }

    /// Verify register integrity using CRC32
    #[inline]
    pub fn verify(&self) -> bool {
        let current_value = self.value.load(Ordering::Acquire);
        let stored_crc = self.checksum.load(Ordering::Acquire);
        let calculated_crc = self.calculate_crc32(current_value);

        stored_crc == calculated_crc
    }

    /// Lock register (prevent modifications)
    #[inline]
    pub fn lock(&mut self) {
        self.state.store(RegisterState::Locked as u32, Ordering::Release);
        self.write_protected = true;
    }

    /// Unlock register
    #[inline]
    pub fn unlock(&mut self) {
        self.state.store(RegisterState::Committed as u32, Ordering::Release);
        self.write_protected = false;
    }

    /// Get current state
    #[inline(always)]
    pub fn get_state(&self) -> RegisterState {
        RegisterState::from(self.state.load(Ordering::Acquire) as u8)
    }

    /// Get version number
    #[inline(always)]
    pub fn get_version(&self) -> u32 {
        self.version.load(Ordering::Acquire)
    }

    /// Calculate CRC32 checksum
    #[inline]
    fn calculate_crc32(&self, value: u64) -> u32 {
        // Simple CRC32 implementation
        let mut crc: u32 = 0xFFFFFFFF;
        let bytes = value.to_le_bytes();

        for byte in bytes.iter() {
            crc ^= *byte as u32;
            for _ in 0..8 {
                if (crc & 1) != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
        }

        !crc
    }

    /// Get fuse address
    #[inline(always)]
    pub fn get_fuse_address(&self) -> u64 {
        self.fuse_addr
    }

    /// Get register ID
    #[inline(always)]
    pub fn get_id(&self) -> u32 {
        self.id
    }
}

/// Shadow Register Bank - manages multiple shadow registers
pub struct ShadowRegisterBank {
    /// Array of shadow registers
    registers: [ShadowRegister; 256],
    /// Number of active registers
    count: usize,
}

impl ShadowRegisterBank {
    /// Create a new shadow register bank
    pub const fn new() -> Self {
        const INIT: ShadowRegister = ShadowRegister::new(0, 0);
        Self {
            registers: [INIT; 256],
            count: 0,
        }
    }

    /// Add a new shadow register
    pub fn add_register(&mut self, id: u32, fuse_addr: u64) -> Result<usize, &'static str> {
        if self.count >= 256 {
            return Err("Register bank is full");
        }

        let index = self.count;
        self.registers[index] = ShadowRegister::new(id, fuse_addr);
        self.count += 1;

        Ok(index)
    }

    /// Get register by ID
    pub fn get_register(&self, id: u32) -> Option<&ShadowRegister> {
        self.registers[..self.count]
            .iter()
            .find(|reg| reg.get_id() == id)
    }

    /// Get mutable register by ID
    pub fn get_register_mut(&mut self, id: u32) -> Option<&mut ShadowRegister> {
        self.registers[..self.count]
            .iter_mut()
            .find(|reg| reg.get_id() == id)
    }

    /// Get register by index
    #[inline(always)]
    pub fn get_by_index(&self, index: usize) -> Option<&ShadowRegister> {
        if index < self.count {
            Some(&self.registers[index])
        } else {
            None
        }
    }

    /// Get mutable register by index
    #[inline(always)]
    pub fn get_by_index_mut(&mut self, index: usize) -> Option<&mut ShadowRegister> {
        if index < self.count {
            Some(&mut self.registers[index])
        } else {
            None
        }
    }

    /// Verify all registers
    pub fn verify_all(&self) -> bool {
        self.registers[..self.count]
            .iter()
            .all(|reg| reg.verify())
    }

    /// Commit all modified registers
    pub fn commit_all(&mut self) -> Result<usize, &'static str> {
        let mut committed = 0;

        for reg in &mut self.registers[..self.count] {
            if reg.get_state() == RegisterState::Modified {
                reg.commit()?;
                committed += 1;
            }
        }

        Ok(committed)
    }

    /// Get count of active registers
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.count
    }
}
