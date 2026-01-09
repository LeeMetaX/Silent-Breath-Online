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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_version_entry_initialization() {
        let entry = VersionEntry::new();
        assert_eq!(entry.version, 0);
        assert_eq!(entry.value, 0);
        assert_eq!(entry.timestamp, 0);
        assert_eq!(entry.checksum, 0);
        assert!(!entry.is_valid());
    }

    #[test]
    fn test_version_entry_from_state() {
        let version = 1;
        let value = 0xABCDEF01;
        let timestamp = 1000;

        let entry = VersionEntry::from_state(version, value, timestamp);

        assert_eq!(entry.get_version(), version);
        assert_eq!(entry.get_value(), value);
        assert_eq!(entry.get_timestamp(), timestamp);
        assert!(entry.is_valid());
        assert!(entry.verify());
    }

    #[test]
    fn test_version_entry_checksum_validation() {
        let entry = VersionEntry::from_state(1, 0x12345678, 2000);

        // Valid entry should verify
        assert!(entry.verify());

        // Create invalid entry manually
        let mut invalid_entry = entry;
        invalid_entry.checksum = 0xFFFFFFFF;

        // Should fail verification
        assert!(!invalid_entry.verify());
    }

    #[test]
    fn test_version_history_initialization() {
        let history = VersionHistory::new();
        assert_eq!(history.count(), 0);
        assert_eq!(history.current_version(), 0);
        assert!(history.get_latest().is_none());
    }

    #[test]
    fn test_version_history_push_and_get() {
        let mut history = VersionHistory::new();

        // Push first version
        let v1 = history.push(0x1111, 100);
        assert_eq!(v1, 0);
        assert_eq!(history.count(), 1);

        // Push second version
        let v2 = history.push(0x2222, 200);
        assert_eq!(v2, 1);
        assert_eq!(history.count(), 2);

        // Retrieve versions
        let entry1 = history.get(0).unwrap();
        assert_eq!(entry1.get_value(), 0x1111);
        assert_eq!(entry1.get_timestamp(), 100);

        let entry2 = history.get(1).unwrap();
        assert_eq!(entry2.get_value(), 0x2222);
        assert_eq!(entry2.get_timestamp(), 200);
    }

    #[test]
    fn test_version_history_circular_buffer_wraparound() {
        let mut history = VersionHistory::new();

        // Fill buffer with 16 versions
        for i in 0..16 {
            let version = history.push(0x1000 + i as u64, i as u64 * 100);
            assert_eq!(version, i);
        }

        assert_eq!(history.count(), 16);

        // Add 17th version - should wrap around
        let v17 = history.push(0x2000, 1700);
        assert_eq!(v17, 16);
        assert_eq!(history.count(), 16); // Still 16 (saturated)

        // First version (0) should be overwritten
        assert!(history.get(0).is_none());

        // Version 16 should be accessible
        assert!(history.get(16).is_some());
        assert_eq!(history.get(16).unwrap().get_value(), 0x2000);
    }

    #[test]
    fn test_version_history_get_latest() {
        let mut history = VersionHistory::new();

        // No versions yet
        assert!(history.get_latest().is_none());

        // Add versions
        history.push(0xAAAA, 100);
        history.push(0xBBBB, 200);
        history.push(0xCCCC, 300);

        // Latest should be the last one added
        let latest = history.get_latest().unwrap();
        assert_eq!(latest.get_value(), 0xCCCC);
        assert_eq!(latest.get_timestamp(), 300);
        assert_eq!(latest.get_version(), 2);
    }

    #[test]
    fn test_version_history_get_by_offset() {
        let mut history = VersionHistory::new();

        // Add 5 versions
        for i in 0..5 {
            history.push(0x1000 + i as u64, i as u64 * 100);
        }

        // Offset 0 = latest (version 4)
        let entry0 = history.get_by_offset(0).unwrap();
        assert_eq!(entry0.get_version(), 4);
        assert_eq!(entry0.get_value(), 0x1004);

        // Offset 1 = previous (version 3)
        let entry1 = history.get_by_offset(1).unwrap();
        assert_eq!(entry1.get_version(), 3);
        assert_eq!(entry1.get_value(), 0x1003);

        // Offset 4 = oldest (version 0)
        let entry4 = history.get_by_offset(4).unwrap();
        assert_eq!(entry4.get_version(), 0);
        assert_eq!(entry4.get_value(), 0x1000);

        // Offset 5 = out of range
        assert!(history.get_by_offset(5).is_none());
    }

    #[test]
    fn test_versioned_shadow_register_write_and_rollback() {
        let mut vreg = VersionedShadowRegister::new(1, 0x1000);

        // Write first version
        let v1 = vreg.write_versioned(0xAAAA, 100).unwrap();
        assert_eq!(v1, 0);
        vreg.get_register_mut().commit().unwrap();

        // Write second version
        let v2 = vreg.write_versioned(0xBBBB, 200).unwrap();
        assert_eq!(v2, 1);
        vreg.get_register_mut().commit().unwrap();

        // Write third version
        let v3 = vreg.write_versioned(0xCCCC, 300).unwrap();
        assert_eq!(v3, 2);
        vreg.get_register_mut().commit().unwrap();

        // Current value should be 0xCCCC
        assert_eq!(vreg.get_register().read(), 0xCCCC);

        // Rollback to version 1
        assert!(vreg.rollback_to_version(1).is_ok());
        assert_eq!(vreg.get_register().read(), 0xBBBB);

        // Rollback to version 0
        assert!(vreg.rollback_to_version(0).is_ok());
        assert_eq!(vreg.get_register().read(), 0xAAAA);
    }

    #[test]
    fn test_versioned_shadow_register_rollback_by_offset() {
        let mut vreg = VersionedShadowRegister::new(2, 0x2000);

        // Write multiple versions
        vreg.write_versioned(0x1111, 100).unwrap();
        vreg.get_register_mut().commit().unwrap();
        vreg.write_versioned(0x2222, 200).unwrap();
        vreg.get_register_mut().commit().unwrap();
        vreg.write_versioned(0x3333, 300).unwrap();
        vreg.get_register_mut().commit().unwrap();
        vreg.write_versioned(0x4444, 400).unwrap();
        vreg.get_register_mut().commit().unwrap();

        // Current value is 0x4444 (offset 0)
        assert_eq!(vreg.get_register().read(), 0x4444);

        // Rollback by offset 1 (to 0x3333)
        assert!(vreg.rollback_by_offset(1).is_ok());
        assert_eq!(vreg.get_register().read(), 0x3333);

        // Rollback by offset 2 (to 0x2222) from latest
        // Note: We need to consider latest is still version 3 in history
        vreg.write_versioned(0x5555, 500).unwrap(); // Write new version
        vreg.get_register_mut().commit().unwrap();
        assert!(vreg.rollback_by_offset(2).is_ok());
        assert_eq!(vreg.get_register().read(), 0x3333);
    }

    #[test]
    fn test_versioned_shadow_register_rollback_errors() {
        let mut vreg = VersionedShadowRegister::new(3, 0x3000);

        // Rollback to non-existent version
        assert!(vreg.rollback_to_version(999).is_err());

        // Rollback by invalid offset
        assert!(vreg.rollback_by_offset(100).is_err());
    }

    #[test]
    fn test_version_overflow_after_16_versions() {
        let mut vreg = VersionedShadowRegister::new(4, 0x4000);

        // Add 20 versions to test overflow behavior
        for i in 0..20 {
            vreg.write_versioned(0x1000 + i as u64, i as u64 * 100)
                .unwrap();
        }

        let history = vreg.get_history();

        // Should have exactly 16 versions (buffer size)
        assert_eq!(history.count(), 16);

        // Latest version should be version 19
        let latest = history.get_latest().unwrap();
        assert_eq!(latest.get_version(), 19);

        // First 4 versions (0-3) should be gone (overwritten)
        assert!(history.get(0).is_none());
        assert!(history.get(1).is_none());
        assert!(history.get(2).is_none());
        assert!(history.get(3).is_none());

        // Versions 4-19 should still exist
        assert!(history.get(4).is_some());
        assert!(history.get(19).is_some());
    }

    #[test]
    fn test_version_history_verify_all() {
        let mut history = VersionHistory::new();

        // Add valid versions
        history.push(0xAAAA, 100);
        history.push(0xBBBB, 200);
        history.push(0xCCCC, 300);

        // All versions should verify
        assert!(history.verify_all());
    }

    #[test]
    fn test_versioned_shadow_register_diff_versions() {
        let mut vreg = VersionedShadowRegister::new(5, 0x5000);

        // Write versions
        vreg.write_versioned(0xAAAA, 100).unwrap();
        vreg.write_versioned(0xBBBB, 200).unwrap();
        vreg.write_versioned(0xCCCC, 300).unwrap();

        // Compare versions 0 and 2
        let diff = vreg.diff_versions(0, 2);
        assert!(diff.is_some());
        let (val1, val2) = diff.unwrap();
        assert_eq!(val1, 0xAAAA);
        assert_eq!(val2, 0xCCCC);

        // Compare with non-existent version
        assert!(vreg.diff_versions(0, 999).is_none());
    }

    #[test]
    fn test_versioned_shadow_register_get_all_versions() {
        let mut vreg = VersionedShadowRegister::new(6, 0x6000);

        // Write 5 versions
        for i in 0..5 {
            vreg.write_versioned(0x1000 + i as u64, i as u64 * 100)
                .unwrap();
        }

        // Get all version numbers
        let versions = vreg.get_all_versions();
        assert_eq!(versions.len(), 5);
        assert_eq!(versions, vec![4, 3, 2, 1, 0]); // Latest to oldest
    }

    #[test]
    fn test_version_history_clear() {
        let mut history = VersionHistory::new();

        // Add versions
        history.push(0xAAAA, 100);
        history.push(0xBBBB, 200);
        assert_eq!(history.count(), 2);

        // Clear history
        history.clear();
        assert_eq!(history.count(), 0);
        assert!(history.get_latest().is_none());

        // Version counter should NOT reset
        assert_eq!(history.current_version(), 2);

        // Add new version - should continue from version 2
        let next_version = history.push(0xCCCC, 300);
        assert_eq!(next_version, 2);
    }

    #[test]
    fn test_global_timestamp() {
        let ts1 = get_timestamp();
        let ts2 = get_timestamp();
        let ts3 = get_timestamp();

        // Timestamps should increment
        assert!(ts2 > ts1);
        assert!(ts3 > ts2);
    }
}
