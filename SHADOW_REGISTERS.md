# Shadow Register Management System

## Overview

A comprehensive Rust application for managing hardware fuse shadow registers with advanced features including:

- **Shadow Register Management**: Atomic register operations with CRC32 integrity checking
- **Hardware Fuse Control**: OTP/MTP/EEPROM fuse programming and verification
- **Synchronization**: Bidirectional sync between shadow registers and fuses
- **Error Correction**: Hamming and Reed-Solomon ECC for data integrity
- **MMIO Interface**: Memory-mapped I/O for direct hardware access
- **Version Control**: Full rollback capability with 16-version history

## Architecture

### Core Components

#### 1. Shadow Register (`shadow_register.rs`)

Manages in-memory copies of hardware fuse values:

```rust
pub struct ShadowRegister {
    id: u32,              // Unique identifier
    value: AtomicU64,     // Current value
    shadow_value: AtomicU64, // Staged value for atomic updates
    state: AtomicU32,     // Register state (Uninitialized/Loaded/Modified/Committed/Locked)
    version: AtomicU32,   // Version counter
    checksum: AtomicU32,  // CRC32 for integrity
}
```

**Features:**
- Atomic read/write operations
- Staged writes with commit/rollback
- CRC32 checksum verification
- Lock/unlock capability
- Version tracking

**Register States:**
- `Uninitialized`: Register not yet loaded
- `Loaded`: Loaded from fuse but not modified
- `Modified`: Shadow value changed, not committed
- `Committed`: Changes applied to active register
- `Locked`: Read-only, cannot be modified
- `Error`: Integrity check failed

#### 2. Fuse Manager (`fuse_manager.rs`)

Controls hardware fuse programming and reading:

```rust
pub struct HardwareFuse {
    address: u64,         // Physical MMIO address
    mode: FuseMode,       // OTP/MTP/EEPROM
    state: FuseState,     // Virgin/Programming/Programmed/Blown
    value: u64,           // Current fuse value
    ecc: u16,             // Error correction code
}
```

**Fuse Modes:**
- `OTP` (One-Time Programmable): Can only be written once
- `MTP` (Multiple-Time Programmable): Can be rewritten multiple times
- `EEPROM`: Electrically erasable, unlimited writes

**Fuse States:**
- `Virgin`: Unprogrammed (all zeros)
- `Programming`: Write in progress
- `Programmed`: Successfully written
- `Blown`: Permanently locked

**Operations:**
- `read_from_hardware()`: Load fuse value via MMIO
- `program_to_hardware()`: Write value to fuse
- `blow()`: Permanently lock fuse
- `verify()`: Check ECC integrity

#### 3. Synchronization Manager (`sync_manager.rs`)

Handles bidirectional sync between shadow registers and fuses:

```rust
pub enum SyncDirection {
    FuseToShadow,       // Load from fuse
    ShadowToFuse,       // Commit to fuse
    ShadowToActive,     // Update active register
    ActiveToShadow,     // Stage changes
    Bidirectional,      // Reconcile conflicts
}

pub enum SyncPolicy {
    ForceOverwrite,     // Always overwrite destination
    InitializeOnly,     // Only if uninitialized
    VersionChecked,     // Match version numbers
    ConflictResolve,    // Newest wins
}
```

**Features:**
- Conflict detection and resolution
- Batch synchronization
- Atomic operations
- Status tracking

#### 4. ECC Handler (`ecc_handler.rs`)

Provides error detection and correction:

**Hamming ECC (72,64):**
- 64 data bits + 8 parity bits
- Single-bit error correction
- Double-bit error detection
- O(1) encode/decode

**Reed-Solomon ECC:**
- Multi-bit error correction
- Configurable block size
- Galois field arithmetic
- Burst error resilience

**Error Types:**
- `NoError`: Data is clean
- `SingleBit`: Correctable error
- `DoubleBit`: Detectable but not correctable
- `MultiBit`: Catastrophic failure

#### 5. MMIO Interface (`shadow_mmio.rs`)

Memory-mapped I/O for hardware access:

```rust
// MMIO Base Addresses
const SHADOW_REG_BASE: usize = 0x5000_0000;
const FUSE_CTRL_BASE: usize = 0x5100_0000;
const SYNC_CTRL_BASE: usize = 0x5200_0000;

// Control Register Layout
// [7:0]   = Command (Read/Write/Commit/Rollback/Lock/etc)
// [15:8]  = Register ID
// [23:16] = Status
// [31:24] = Reserved
```

**Commands:**
- `Read`: Read shadow register
- `Write`: Write shadow register
- `Commit`: Commit to active
- `Rollback`: Revert changes
- `Lock/Unlock`: Access control
- `Verify`: Check integrity
- `LoadFuse`: Load from fuse
- `CommitFuse`: Program to fuse
- `Sync`: Synchronize

**Features:**
- Spin-wait for real-time guarantees
- Batch operations
- Error reporting
- Status monitoring

#### 6. Version Control (`version_control.rs`)

Temporal management with rollback:

```rust
pub struct VersionHistory {
    entries: [VersionEntry; 16],  // Circular buffer
    head: AtomicU32,              // Write position
    version_counter: AtomicU32,   // Global counter
}

pub struct VersionEntry {
    version: u32,      // Version number
    value: u64,        // Register value at version
    timestamp: u64,    // When it was created
    checksum: u32,     // Integrity check
}
```

**Features:**
- 16-version circular buffer
- Timestamp tracking
- Checksum verification
- Diff between versions
- Rollback by version or offset

#### 7. Shadow Runtime (`shadow_runtime.rs`)

Complete integration of all components:

```rust
pub struct ShadowRegisterRuntime {
    shadow_bank: ShadowRegisterBank,
    fuse_manager: FuseManager,
    sync_manager: SyncManager,
    ecc_manager: ECCManager,
    mmio_controller: ShadowMMIOController,
}
```

**API:**
- `register_fuse()`: Register new fuse-backed register
- `read()`: Read with integrity check
- `write()`: Write with ECC encoding
- `commit()`: Commit to active
- `sync()`: Synchronize with fuses
- `verify_all()`: Bulk integrity check

## Usage Examples

### Basic Shadow Register Operations

```rust
use silent_breath_mmio::*;

unsafe {
    // Create runtime
    let runtime = shadow_runtime_init();

    // Register a fuse at address 0x6000_0000
    shadow_runtime_register_fuse(runtime, 0, 0x6000_0000, 0); // OTP mode

    // Write value to shadow register
    shadow_runtime_write(runtime, 0, 0x12345678);

    // Commit to active register
    shadow_runtime_commit(runtime, 0);

    // Read back
    let mut value = 0u64;
    shadow_runtime_read(runtime, 0, &mut value);

    // Commit to hardware fuse
    shadow_runtime_commit_to_fuses(runtime);

    // Verify all registers
    shadow_runtime_verify_all(runtime);
}
```

### Version Control with Rollback

```rust
use silent_breath_mmio::*;

let mut runtime = VersionedShadowRuntime::new();

// Add a register
runtime.add_register(0, 0x6000_0000).unwrap();

// Write multiple versions
let v1 = runtime.write_versioned(0, 0x1111).unwrap(); // Version 0
let v2 = runtime.write_versioned(0, 0x2222).unwrap(); // Version 1
let v3 = runtime.write_versioned(0, 0x3333).unwrap(); // Version 2

// Rollback to version 1
runtime.rollback_to_version(0, v2).unwrap();

// Or rollback by offset (0 = latest, 1 = previous, etc.)
runtime.rollback_by_offset(0, 1).unwrap();

// Get version history
if let Some(reg) = runtime.get_register(0) {
    let versions = reg.get_all_versions();
    println!("Available versions: {:?}", versions);
}
```

### Synchronization with Policies

```rust
use silent_breath_mmio::*;

unsafe {
    let mut runtime = ShadowRegisterRuntime::new();
    runtime.init();

    // Register fuses
    runtime.register_fuse(0, 0x6000_0000, FuseMode::MTP).unwrap();
    runtime.register_fuse(1, 0x6000_0008, FuseMode::MTP).unwrap();

    // Load all fuses into shadow registers
    runtime.sync(SyncDirection::FuseToShadow, SyncPolicy::ForceOverwrite).unwrap();

    // Modify shadow registers
    runtime.write(0, 0xDEADBEEF).unwrap();
    runtime.commit(0).unwrap();

    // Sync back to fuses (only if uninitialized)
    runtime.sync(SyncDirection::ShadowToFuse, SyncPolicy::InitializeOnly).unwrap();

    // Bidirectional sync with conflict resolution
    runtime.sync(SyncDirection::Bidirectional, SyncPolicy::ConflictResolve).unwrap();
}
```

### MMIO Direct Access

```rust
use silent_breath_mmio::*;

unsafe {
    let mut runtime = ShadowRegisterRuntime::new();
    runtime.init();

    if let Some(mmio) = runtime.get_mmio_controller_mut() {
        // Read via MMIO
        let value = mmio.mmio_read(0).unwrap();

        // Write via MMIO
        mmio.mmio_write(0, 0xCAFEBABE).unwrap();

        // Commit via MMIO
        mmio.mmio_commit(0).unwrap();

        // Lock register
        mmio.mmio_lock(0).unwrap();

        // Verify checksum
        let valid = mmio.mmio_verify(0).unwrap();

        // Batch operations
        let values = mmio.mmio_batch_read(&[0, 1, 2]).unwrap();
        mmio.mmio_batch_write(&[(0, 0x1111), (1, 0x2222)]).unwrap();
        mmio.mmio_batch_commit(&[0, 1, 2]).unwrap();
    }
}
```

### Error Correction

```rust
use silent_breath_mmio::*;

let ecc_manager = ECCManager::new(ECCStrategy::Hamming);

// Encode data
let data = 0xDEADBEEFCAFEBABE;
let (encoded, parity) = ecc_manager.encode_u64(data);

// Simulate bit flip
let corrupted = encoded ^ (1 << 10);

// Decode and correct
match ecc_manager.decode_u64(corrupted, parity) {
    Ok((corrected, syndrome)) => {
        match syndrome.error_type {
            ECCError::NoError => println!("No errors"),
            ECCError::SingleBit => println!("Corrected single-bit error at position {}", syndrome.error_position),
            ECCError::DoubleBit => println!("Double-bit error detected"),
            ECCError::MultiBit => println!("Multi-bit error - cannot correct"),
        }
    }
    Err(e) => println!("ECC decode failed: {}", e),
}

// Get error statistics
let (detected, corrected) = ecc_manager.get_total_errors();
println!("Errors detected: {}, corrected: {}", detected, corrected);
```

## Key Features

### 1. Real-Time Guarantees
- Lock-free atomic operations
- Spin-wait for deterministic timing
- O(1) state transitions
- Cache-aligned structures (64-byte)

### 2. Data Integrity
- CRC32 checksum on all registers
- Hamming ECC (single-bit correction)
- Reed-Solomon ECC (multi-bit correction)
- Automatic verification on read

### 3. Safety & Reliability
- Write protection and locking
- Staged commits with rollback
- Version history (16 snapshots)
- Conflict detection and resolution

### 4. Performance
- `#![no_std]` for bare-metal
- Zero-cost abstractions
- Inline assembly ready
- SIMD-friendly memory layout

### 5. Flexibility
- Multiple fuse modes (OTP/MTP/EEPROM)
- Configurable sync policies
- Batch operations
- FFI interface for C integration

## Build & Integration

```bash
# Build release version
cargo build --release

# Run tests (if std is available)
cargo test --features std

# Generate docs
cargo doc --open
```

### FFI Integration (C/C++)

```c
// Initialize runtime
void* runtime = shadow_runtime_init();

// Register a fuse
shadow_runtime_register_fuse(runtime, 0, 0x6000_0000, 0);

// Write value
shadow_runtime_write(runtime, 0, 0x12345678);

// Read value
uint64_t value;
shadow_runtime_read(runtime, 0, &value);

// Verify
if (shadow_runtime_verify_all(runtime) == 0) {
    printf("All registers valid\n");
}
```

## Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Register read | O(1) | Atomic load + CRC check |
| Register write | O(1) | Atomic store + version increment |
| Commit | O(1) | Atomic swap + checksum |
| Rollback | O(1) | Copy from backup |
| Version lookup | O(n) | n = history size (max 16) |
| ECC encode | O(1) | Hamming parity calculation |
| ECC decode | O(1) | Syndrome generation + correction |
| Fuse read | O(1) | MMIO volatile read + ECC |
| Fuse program | O(1) | MMIO volatile write + verify |
| Sync all | O(n) | n = number of registers |

## Memory Footprint

- `ShadowRegister`: 128 bytes (cache-aligned)
- `HardwareFuse`: 32 bytes
- `VersionEntry`: 32 bytes
- `VersionHistory`: 512 bytes (16 entries)
- `VersionedShadowRegister`: 640 bytes
- `ShadowRegisterBank`: 32 KB (256 registers)
- `FuseManager`: 4 KB (128 fuses)

## License

MIT OR Apache-2.0
