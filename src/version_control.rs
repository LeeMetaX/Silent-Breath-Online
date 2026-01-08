/// Register Version Control and Rollback System
/// Provides temporal management of shadow register values

use crate::shadow_register::ShadowRegister;
use core::sync::atomic::{AtomicU32, Ordering};

/// Maximum number of versions to keep in history
pub const MAX_VERSION_HISTORY: usize = 16;

/// Version Entry - represents a snapshot of register state
#[derive(Clone, Copy)]
#[repr(C, align(16))]
pub struct VersionEntry {
    /// Version number
    version: u32,
    /// Register value at this version
    value: u64,
    /// Timestamp (in cycles or ticks)
    timestamp: u64,
    /// Checksum for integrity
    checksum: u32,
    /// Valid flag
    valid: bool,
}

impl VersionEntry {
    /// Create a new version entry
    pub const fn new() -> Self {
        Self {
            version: 0,
            value: 0,
            timestamp: 0,
            checksum: 0,
            valid: false,
        }
    }

    /// Create a version entry from current state
    pub fn from_state(version: u32, value: u64, timestamp: u64) -> Self {
        let checksum = Self::calculate_checksum(value, timestamp);
        Self {
            version,
            value,
            timestamp,
            checksum,
            valid: true,
        }
    }

    /// Verify version entry integrity
    pub fn verify(&self) -> bool {
        if !self.valid {
            return false;
        }

        let calculated = Self::calculate_checksum(self.value, self.timestamp);
        calculated == self.checksum
    }

    /// Calculate checksum for version entry
    fn calculate_checksum(value: u64, timestamp: u64) -> u32 {
        let combined = value ^ timestamp;
        ((combined >> 32) as u32) ^ (combined as u32)
    }

    /// Check if entry is valid
    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Get version number
    #[inline(always)]
    pub fn get_version(&self) -> u32 {
        self.version
    }

    /// Get value
    #[inline(always)]
    pub fn get_value(&self) -> u64 {
        self.value
    }

    /// Get timestamp
    #[inline(always)]
    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// Version History - circular buffer of version entries
pub struct VersionHistory {
    /// Circular buffer of version entries
    entries: [VersionEntry; MAX_VERSION_HISTORY],
    /// Current write position
    head: AtomicU32,
    /// Number of valid entries
    count: AtomicU32,
    /// Global version counter
    version_counter: AtomicU32,
}

impl VersionHistory {
    /// Create a new version history
    pub const fn new() -> Self {
        const INIT: VersionEntry = VersionEntry::new();
        Self {
            entries: [INIT; MAX_VERSION_HISTORY],
            head: AtomicU32::new(0),
            count: AtomicU32::new(0),
            version_counter: AtomicU32::new(0),
        }
    }

    /// Add a new version to history
    pub fn push(&mut self, value: u64, timestamp: u64) -> u32 {
        // Get next version number
        let version = self.version_counter.fetch_add(1, Ordering::AcqRel);

        // Get current head position
        let head = self.head.load(Ordering::Acquire) as usize;

        // Create new entry
        self.entries[head] = VersionEntry::from_state(version, value, timestamp);

        // Advance head (circular)
        let next_head = (head + 1) % MAX_VERSION_HISTORY;
        self.head.store(next_head as u32, Ordering::Release);

        // Update count (saturate at MAX_VERSION_HISTORY)
        let count = self.count.load(Ordering::Acquire);
        if count < MAX_VERSION_HISTORY as u32 {
            self.count.fetch_add(1, Ordering::AcqRel);
        }

        version
    }

    /// Get version entry by version number
    pub fn get(&self, version: u32) -> Option<&VersionEntry> {
        let count = self.count.load(Ordering::Acquire) as usize;
        let head = self.head.load(Ordering::Acquire) as usize;

        // Search backwards from head
        for i in 0..count {
            let index = (head + MAX_VERSION_HISTORY - 1 - i) % MAX_VERSION_HISTORY;
            let entry = &self.entries[index];

            if entry.is_valid() && entry.get_version() == version {
                return Some(entry);
            }
        }

        None
    }

    /// Get most recent version
    pub fn get_latest(&self) -> Option<&VersionEntry> {
        let count = self.count.load(Ordering::Acquire);
        if count == 0 {
            return None;
        }

        let head = self.head.load(Ordering::Acquire) as usize;
        let latest_index = (head + MAX_VERSION_HISTORY - 1) % MAX_VERSION_HISTORY;

        Some(&self.entries[latest_index])
    }

    /// Get version by relative offset (0 = latest, 1 = previous, etc.)
    pub fn get_by_offset(&self, offset: usize) -> Option<&VersionEntry> {
        let count = self.count.load(Ordering::Acquire) as usize;
        if offset >= count {
            return None;
        }

        let head = self.head.load(Ordering::Acquire) as usize;
        let index = (head + MAX_VERSION_HISTORY - 1 - offset) % MAX_VERSION_HISTORY;

        Some(&self.entries[index])
    }

    /// Get total version count
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Acquire) as usize
    }

    /// Get current version number
    #[inline(always)]
    pub fn current_version(&self) -> u32 {
        self.version_counter.load(Ordering::Acquire)
    }

    /// Clear all version history
    pub fn clear(&mut self) {
        self.head.store(0, Ordering::Release);
        self.count.store(0, Ordering::Release);
        // Note: version_counter is NOT reset to maintain uniqueness
    }

    /// Verify all entries in history
    pub fn verify_all(&self) -> bool {
        let count = self.count.load(Ordering::Acquire) as usize;

        for i in 0..count {
            if !self.entries[i].verify() {
                return false;
            }
        }

        true
    }
}

/// Versioned Shadow Register - Shadow register with version control
pub struct VersionedShadowRegister {
    /// Base shadow register
    register: ShadowRegister,
    /// Version history
    history: VersionHistory,
}

impl VersionedShadowRegister {
    /// Create a new versioned shadow register
    pub const fn new(id: u32, fuse_addr: u64) -> Self {
        Self {
            register: ShadowRegister::new(id, fuse_addr),
            history: VersionHistory::new(),
        }
    }

    /// Write to register and record version
    pub fn write_versioned(&mut self, value: u64, timestamp: u64) -> Result<u32, &'static str> {
        // Write to shadow register
        self.register.write(value)?;

        // Add to version history
        let version = self.history.push(value, timestamp);

        Ok(version)
    }

    /// Rollback to specific version
    pub fn rollback_to_version(&mut self, version: u32) -> Result<(), &'static str> {
        // Find version in history
        if let Some(entry) = self.history.get(version) {
            if !entry.verify() {
                return Err("Version entry corrupted");
            }

            // Rollback register
            self.register.write(entry.get_value())?;
            self.register.commit()?;

            Ok(())
        } else {
            Err("Version not found in history")
        }
    }

    /// Rollback by offset (0 = latest, 1 = previous, etc.)
    pub fn rollback_by_offset(&mut self, offset: usize) -> Result<(), &'static str> {
        if let Some(entry) = self.history.get_by_offset(offset) {
            if !entry.verify() {
                return Err("Version entry corrupted");
            }

            self.register.write(entry.get_value())?;
            self.register.commit()?;

            Ok(())
        } else {
            Err("Version offset out of range")
        }
    }

    /// Get register
    #[inline(always)]
    pub fn get_register(&self) -> &ShadowRegister {
        &self.register
    }

    /// Get mutable register
    #[inline(always)]
    pub fn get_register_mut(&mut self) -> &mut ShadowRegister {
        &mut self.register
    }

    /// Get version history
    #[inline(always)]
    pub fn get_history(&self) -> &VersionHistory {
        &self.history
    }

    /// Get mutable version history
    #[inline(always)]
    pub fn get_history_mut(&mut self) -> &mut VersionHistory {
        &mut self.history
    }

    /// Compare two versions
    pub fn diff_versions(&self, version1: u32, version2: u32) -> Option<(u64, u64)> {
        let entry1 = self.history.get(version1)?;
        let entry2 = self.history.get(version2)?;

        Some((entry1.get_value(), entry2.get_value()))
    }

    /// Get all version numbers
    pub fn get_all_versions(&self) -> Vec<u32> {
        let count = self.history.count();
        let mut versions = Vec::with_capacity(count);

        for i in 0..count {
            if let Some(entry) = self.history.get_by_offset(i) {
                if entry.is_valid() {
                    versions.push(entry.get_version());
                }
            }
        }

        versions
    }
}

/// Global timestamp counter (for versioning)
static GLOBAL_TIMESTAMP: AtomicU32 = AtomicU32::new(0);

/// Get current timestamp
#[inline(always)]
pub fn get_timestamp() -> u64 {
    GLOBAL_TIMESTAMP.fetch_add(1, Ordering::Relaxed) as u64
}

// Vec implementation for no_std
extern crate alloc;
use alloc::vec::Vec;
