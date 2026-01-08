/// Register Synchronization Manager
/// Handles synchronization between shadow registers, fuses, and active hardware

use crate::fuse_manager::FuseManager;
use crate::shadow_register::{RegisterState, ShadowRegister};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Synchronization Direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    /// Fuse → Shadow Register
    FuseToShadow,
    /// Shadow Register → Fuse
    ShadowToFuse,
    /// Shadow Register → Active Register (in-memory)
    ShadowToActive,
    /// Active Register → Shadow Register
    ActiveToShadow,
    /// Bidirectional sync (reconcile differences)
    Bidirectional,
}

/// Synchronization Policy
#[derive(Debug, Clone, Copy)]
pub enum SyncPolicy {
    /// Always overwrite destination
    ForceOverwrite,
    /// Only sync if destination is uninitialized
    InitializeOnly,
    /// Only sync if versions match
    VersionChecked,
    /// Use conflict resolution (newest wins)
    ConflictResolve,
}

/// Synchronization Status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SyncStatus {
    Idle = 0x00,
    InProgress = 0x01,
    Success = 0x02,
    Failed = 0x03,
    Conflict = 0x04,
}

/// Synchronization Result
pub struct SyncResult {
    pub status: SyncStatus,
    pub synced_count: usize,
    pub failed_count: usize,
    pub conflict_count: usize,
}

/// Register Synchronization Manager
pub struct SyncManager {
    /// Current sync status
    status: AtomicU32,
    /// Sync in progress flag
    syncing: AtomicBool,
    /// Total syncs performed
    sync_count: AtomicU32,
}

impl SyncManager {
    /// Create a new sync manager
    pub const fn new() -> Self {
        Self {
            status: AtomicU32::new(SyncStatus::Idle as u32),
            syncing: AtomicBool::new(false),
            sync_count: AtomicU32::new(0),
        }
    }

    /// Synchronize a single register
    pub unsafe fn sync_register(
        &self,
        fuse_manager: &mut FuseManager,
        register_id: u32,
        direction: SyncDirection,
        policy: SyncPolicy,
    ) -> Result<(), &'static str> {
        // Check if already syncing
        if self.syncing.swap(true, Ordering::AcqRel) {
            return Err("Sync already in progress");
        }

        self.status
            .store(SyncStatus::InProgress as u32, Ordering::Release);

        let result = match direction {
            SyncDirection::FuseToShadow => self.sync_fuse_to_shadow(fuse_manager, register_id, policy),
            SyncDirection::ShadowToFuse => self.sync_shadow_to_fuse(fuse_manager, register_id, policy),
            SyncDirection::ShadowToActive => Ok(()), // Handled by commit()
            SyncDirection::ActiveToShadow => Ok(()), // Handled by write()
            SyncDirection::Bidirectional => {
                self.sync_bidirectional(fuse_manager, register_id, policy)
            }
        };

        // Update status
        match result {
            Ok(_) => {
                self.status
                    .store(SyncStatus::Success as u32, Ordering::Release);
                self.sync_count.fetch_add(1, Ordering::AcqRel);
            }
            Err(_) => {
                self.status
                    .store(SyncStatus::Failed as u32, Ordering::Release);
            }
        }

        self.syncing.store(false, Ordering::Release);

        result
    }

    /// Sync from fuse to shadow register
    unsafe fn sync_fuse_to_shadow(
        &self,
        fuse_manager: &mut FuseManager,
        register_id: u32,
        policy: SyncPolicy,
    ) -> Result<(), &'static str> {
        let shadow_bank = fuse_manager.get_shadow_bank_mut();

        if let Some(shadow_reg) = shadow_bank.get_register_mut(register_id) {
            // Check policy
            match policy {
                SyncPolicy::InitializeOnly => {
                    if shadow_reg.get_state() != RegisterState::Uninitialized {
                        return Ok(()); // Skip if already initialized
                    }
                }
                SyncPolicy::VersionChecked => {
                    // Would need version info from fuse
                    // Skip for now
                }
                _ => {}
            }

            // Load from fuse
            fuse_manager.load_to_shadow(register_id as usize)?;
        }

        Ok(())
    }

    /// Sync from shadow register to fuse
    unsafe fn sync_shadow_to_fuse(
        &self,
        fuse_manager: &mut FuseManager,
        register_id: u32,
        policy: SyncPolicy,
    ) -> Result<(), &'static str> {
        let shadow_bank = fuse_manager.get_shadow_bank();

        if let Some(shadow_reg) = shadow_bank.get_register(register_id) {
            // Check policy
            match policy {
                SyncPolicy::InitializeOnly => {
                    if let Some(fuse) = fuse_manager.get_fuse(register_id as usize) {
                        if !fuse.is_virgin() {
                            return Ok(()); // Skip if fuse already programmed
                        }
                    }
                }
                _ => {}
            }

            // Commit to fuse
            fuse_manager.commit_to_fuse(register_id as usize)?;
        }

        Ok(())
    }

    /// Bidirectional sync with conflict resolution
    unsafe fn sync_bidirectional(
        &self,
        fuse_manager: &mut FuseManager,
        register_id: u32,
        policy: SyncPolicy,
    ) -> Result<(), &'static str> {
        // Read both values
        let fuse_value = if let Some(fuse) = fuse_manager.get_fuse_mut(register_id as usize) {
            fuse.read_from_hardware()?
        } else {
            return Err("Fuse not found");
        };

        let shadow_value = if let Some(shadow_reg) =
            fuse_manager.get_shadow_bank().get_register(register_id)
        {
            shadow_reg.read()
        } else {
            return Err("Shadow register not found");
        };

        // Check for conflicts
        if fuse_value != shadow_value {
            match policy {
                SyncPolicy::ForceOverwrite => {
                    // Shadow wins
                    fuse_manager.commit_to_fuse(register_id as usize)?;
                }
                SyncPolicy::ConflictResolve => {
                    // Use version to determine winner
                    if let Some(shadow_reg) =
                        fuse_manager.get_shadow_bank().get_register(register_id)
                    {
                        if shadow_reg.get_version() > 0 {
                            // Shadow is newer, commit to fuse
                            fuse_manager.commit_to_fuse(register_id as usize)?;
                        } else {
                            // Fuse is newer, load to shadow
                            fuse_manager.load_to_shadow(register_id as usize)?;
                        }
                    }
                }
                _ => {
                    self.status
                        .store(SyncStatus::Conflict as u32, Ordering::Release);
                    return Err("Sync conflict detected");
                }
            }
        }

        Ok(())
    }

    /// Synchronize all registers
    pub unsafe fn sync_all(
        &self,
        fuse_manager: &mut FuseManager,
        direction: SyncDirection,
        policy: SyncPolicy,
    ) -> SyncResult {
        let mut synced = 0;
        let mut failed = 0;
        let mut conflicts = 0;

        for i in 0..fuse_manager.count() {
            match self.sync_register(fuse_manager, i as u32, direction, policy) {
                Ok(_) => synced += 1,
                Err(_) => {
                    if self.status.load(Ordering::Acquire) == SyncStatus::Conflict as u32 {
                        conflicts += 1;
                    } else {
                        failed += 1;
                    }
                }
            }
        }

        let status = if failed == 0 && conflicts == 0 {
            SyncStatus::Success
        } else if synced == 0 {
            SyncStatus::Failed
        } else {
            SyncStatus::Conflict
        };

        SyncResult {
            status,
            synced_count: synced,
            failed_count: failed,
            conflict_count: conflicts,
        }
    }

    /// Check if sync is in progress
    #[inline(always)]
    pub fn is_syncing(&self) -> bool {
        self.syncing.load(Ordering::Acquire)
    }

    /// Get current sync status
    #[inline(always)]
    pub fn get_status(&self) -> SyncStatus {
        match self.status.load(Ordering::Acquire) {
            0 => SyncStatus::Idle,
            1 => SyncStatus::InProgress,
            2 => SyncStatus::Success,
            3 => SyncStatus::Failed,
            4 => SyncStatus::Conflict,
            _ => SyncStatus::Failed,
        }
    }

    /// Get total sync count
    #[inline(always)]
    pub fn get_sync_count(&self) -> u32 {
        self.sync_count.load(Ordering::Acquire)
    }

    /// Reset sync manager
    pub fn reset(&self) {
        self.status.store(SyncStatus::Idle as u32, Ordering::Release);
        self.syncing.store(false, Ordering::Release);
    }
}
