/// Hardware Fuse Management System
/// Controls fuse programming, reading, and verification

use crate::shadow_register::{ShadowRegister, ShadowRegisterBank};
use core::ptr::{read_volatile, write_volatile};

/// Fuse Programming State
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FuseState {
    /// Fuse is unprogrammed (virgin)
    Virgin = 0x00,
    /// Fuse programming in progress
    Programming = 0x01,
    /// Fuse is programmed and locked
    Programmed = 0x02,
    /// Fuse is blown (permanent)
    Blown = 0x03,
    /// Fuse has error
    Error = 0xFF,
}

/// Fuse Programming Mode
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum FuseMode {
    /// One-time programmable (OTP)
    OTP = 0x00,
    /// Multiple-time programmable (MTP)
    MTP = 0x01,
    /// Electrically erasable (EEPROM-like)
    EEPROM = 0x02,
}

/// Hardware Fuse Descriptor
#[repr(C, align(32))]
pub struct HardwareFuse {
    /// Fuse physical address
    address: u64,
    /// Fuse programming mode
    mode: FuseMode,
    /// Current fuse state
    state: FuseState,
    /// Fuse value (64-bit)
    value: u64,
    /// Fuse lock bit
    locked: bool,
    /// Redundancy count (for multi-bit fuses)
    redundancy: u8,
    /// Error correction code (ECC) bits
    ecc: u16,
}

impl HardwareFuse {
    /// Create a new hardware fuse descriptor
    pub const fn new(address: u64, mode: FuseMode) -> Self {
        Self {
            address,
            mode,
            state: FuseState::Virgin,
            value: 0,
            locked: false,
            redundancy: 1,
            ecc: 0,
        }
    }

    /// Read fuse value from hardware
    #[inline]
    pub unsafe fn read_from_hardware(&mut self) -> Result<u64, &'static str> {
        // Read from physical fuse address via MMIO
        let fuse_ptr = self.address as *const u64;
        let value = read_volatile(fuse_ptr);

        // Verify ECC if enabled
        if self.redundancy > 1 {
            let calculated_ecc = self.calculate_ecc(value);
            if calculated_ecc != self.ecc && self.ecc != 0 {
                self.state = FuseState::Error;
                return Err("ECC mismatch - fuse data corrupted");
            }
        }

        self.value = value;
        self.state = if value != 0 {
            FuseState::Programmed
        } else {
            FuseState::Virgin
        };

        Ok(value)
    }

    /// Program fuse value to hardware
    #[inline]
    pub unsafe fn program_to_hardware(&mut self, value: u64) -> Result<(), &'static str> {
        // Check if already locked
        if self.locked {
            return Err("Fuse is locked");
        }

        // Check if OTP and already programmed
        if matches!(self.mode, FuseMode::OTP) && self.state == FuseState::Programmed {
            return Err("OTP fuse already programmed");
        }

        // Set programming state
        self.state = FuseState::Programming;

        // Calculate ECC for redundancy
        if self.redundancy > 1 {
            self.ecc = self.calculate_ecc(value);
        }

        // Write to physical fuse address via MMIO
        let fuse_ptr = self.address as *mut u64;
        write_volatile(fuse_ptr, value);

        // Verify write
        let readback = read_volatile(fuse_ptr);
        if readback != value {
            self.state = FuseState::Error;
            return Err("Fuse programming verification failed");
        }

        self.value = value;
        self.state = FuseState::Programmed;

        Ok(())
    }

    /// Blow (permanently lock) the fuse
    #[inline]
    pub fn blow(&mut self) -> Result<(), &'static str> {
        if self.state != FuseState::Programmed {
            return Err("Can only blow programmed fuses");
        }

        self.state = FuseState::Blown;
        self.locked = true;

        Ok(())
    }

    /// Check if fuse is virgin (unprogrammed)
    #[inline(always)]
    pub fn is_virgin(&self) -> bool {
        self.state == FuseState::Virgin
    }

    /// Check if fuse is locked
    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Get fuse value
    #[inline(always)]
    pub fn get_value(&self) -> u64 {
        self.value
    }

    /// Get fuse state
    #[inline(always)]
    pub fn get_state(&self) -> FuseState {
        self.state
    }

    /// Calculate Hamming ECC for error detection/correction
    #[inline]
    fn calculate_ecc(&self, data: u64) -> u16 {
        let mut ecc: u16 = 0;
        let mut temp = data;

        // Simple parity-based ECC (8 parity bits)
        for i in 0..8 {
            let mut parity = 0;
            for j in 0..8 {
                if (temp >> (i * 8 + j)) & 1 == 1 {
                    parity ^= 1;
                }
            }
            ecc |= (parity as u16) << i;
        }

        ecc
    }
}

/// Fuse Manager - manages all hardware fuses
pub struct FuseManager {
    /// Array of hardware fuses
    fuses: [HardwareFuse; 128],
    /// Number of active fuses
    count: usize,
    /// Shadow register bank for syncing
    shadow_bank: ShadowRegisterBank,
}

impl FuseManager {
    /// Create a new fuse manager
    pub const fn new() -> Self {
        const INIT: HardwareFuse = HardwareFuse::new(0, FuseMode::OTP);
        Self {
            fuses: [INIT; 128],
            count: 0,
            shadow_bank: ShadowRegisterBank::new(),
        }
    }

    /// Add a new fuse
    pub fn add_fuse(&mut self, address: u64, mode: FuseMode) -> Result<usize, &'static str> {
        if self.count >= 128 {
            return Err("Fuse manager is full");
        }

        let index = self.count;
        self.fuses[index] = HardwareFuse::new(address, mode);
        self.count += 1;

        // Create corresponding shadow register
        self.shadow_bank.add_register(index as u32, address)?;

        Ok(index)
    }

    /// Load fuse value into shadow register
    pub unsafe fn load_to_shadow(&mut self, fuse_index: usize) -> Result<(), &'static str> {
        if fuse_index >= self.count {
            return Err("Invalid fuse index");
        }

        // Read from hardware fuse
        let fuse = &mut self.fuses[fuse_index];
        let value = fuse.read_from_hardware()?;

        // Write to shadow register
        if let Some(shadow_reg) = self.shadow_bank.get_by_index_mut(fuse_index) {
            shadow_reg.write(value)?;
            shadow_reg.commit()?;
        }

        Ok(())
    }

    /// Commit shadow register to fuse
    pub unsafe fn commit_to_fuse(&mut self, fuse_index: usize) -> Result<(), &'static str> {
        if fuse_index >= self.count {
            return Err("Invalid fuse index");
        }

        // Get shadow register value
        let shadow_value = if let Some(shadow_reg) = self.shadow_bank.get_by_index(fuse_index) {
            shadow_reg.read()
        } else {
            return Err("Shadow register not found");
        };

        // Program to hardware fuse
        let fuse = &mut self.fuses[fuse_index];
        fuse.program_to_hardware(shadow_value)?;

        Ok(())
    }

    /// Load all fuses into shadow registers
    pub unsafe fn load_all(&mut self) -> Result<usize, &'static str> {
        let mut loaded = 0;

        for i in 0..self.count {
            if self.load_to_shadow(i).is_ok() {
                loaded += 1;
            }
        }

        Ok(loaded)
    }

    /// Commit all shadow registers to fuses
    pub unsafe fn commit_all(&mut self) -> Result<usize, &'static str> {
        let mut committed = 0;

        for i in 0..self.count {
            if self.commit_to_fuse(i).is_ok() {
                committed += 1;
            }
        }

        Ok(committed)
    }

    /// Verify all fuses against shadow registers
    pub fn verify_all(&self) -> bool {
        for i in 0..self.count {
            let fuse_value = self.fuses[i].get_value();

            if let Some(shadow_reg) = self.shadow_bank.get_by_index(i) {
                if shadow_reg.read() != fuse_value {
                    return false;
                }
                if !shadow_reg.verify() {
                    return false;
                }
            }
        }

        true
    }

    /// Get fuse count
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Get fuse by index
    #[inline(always)]
    pub fn get_fuse(&self, index: usize) -> Option<&HardwareFuse> {
        if index < self.count {
            Some(&self.fuses[index])
        } else {
            None
        }
    }

    /// Get mutable fuse by index
    #[inline(always)]
    pub fn get_fuse_mut(&mut self, index: usize) -> Option<&mut HardwareFuse> {
        if index < self.count {
            Some(&mut self.fuses[index])
        } else {
            None
        }
    }

    /// Get shadow register bank
    #[inline(always)]
    pub fn get_shadow_bank(&self) -> &ShadowRegisterBank {
        &self.shadow_bank
    }

    /// Get mutable shadow register bank
    #[inline(always)]
    pub fn get_shadow_bank_mut(&mut self) -> &mut ShadowRegisterBank {
        &mut self.shadow_bank
    }
}
