# MMIO Cache Coherency System

## Overview

This module implements a **Rust-based MMIO (Memory-Mapped I/O) Real-Time Cache Coherency System** that replaces traditional ROMs and firmware with a 4-state logic gating mechanism.

## Architecture

### 4-State Logic Gating (MESI Protocol)

- **M**odified: Cache line is dirty, exclusively held by one core
- **E**xclusive: Cache line is clean, exclusively held by one core
- **S**hared: Cache line exists in multiple cores, all copies are clean
- **I**nvalid: Cache line is invalid, must be fetched from L3 or another core

### Real-Time Traversal Flow

The system implements a 5-step cache coherency flow:

```
1. Core 1 reads data → stored in L1, L2, L3 (Shared state)
2. Core 2 reads same data → also Shared across all levels
3. Core 1 writes to data → invalidates Core 2's copy via L3
4. Core 2's cache line marked Invalid
5. Core 2 reads again → fetches from Core 1 or L3
```

## Components

### `cache_coherency.rs`
- `CacheLine`: 64-byte cache line with atomic state transitions
- `CacheState`: 4-state enum (Modified, Exclusive, Shared, Invalid)
- `L3Directory`: L3 cache directory for multi-core coherency

### `mmio.rs`
- `CoherencyRegister`: MMIO register interface for hardware access
- `MMIOCoherency`: Real-time MMIO accessor with spin-wait guarantees
- Direct memory-mapped access to cache coherency hardware

### `state_machine.rs`
- `StateTransitionTable`: Pre-computed O(1) state transition lookup
- `CoherencyStateMachine`: Implements the 5-step coherency flow
- Lock-free state transitions for real-time guarantees

### `runtime.rs`
- `CoreCacheController`: Per-core cache controller
- `CoherencyRuntime`: Multi-core coherency orchestrator
- FFI interface for firmware integration

## Features

- **No Standard Library** (`#![no_std]`): Suitable for bare-metal firmware
- **Lock-Free Operations**: Uses atomic operations for zero-latency transitions
- **Real-Time Guarantees**: Spin-wait loops ensure deterministic timing
- **MMIO Direct Access**: Memory-mapped I/O for hardware-level control
- **Zero-Cost Abstractions**: Compiled to optimal machine code

## Usage

```rust
use silent_breath_mmio::*;

unsafe {
    // Initialize runtime
    let runtime = mmio_coherency_init();

    // Execute coherency flow
    let address = 0x1000_0000;
    let result = mmio_coherency_execute(runtime, address);
}
```

## Build

```bash
cargo build --release
```

## Performance

- **O(1) State Transitions**: Pre-computed lookup table
- **Cache-Aligned Structures**: 64-byte alignment for cache efficiency
- **Atomic Operations**: Lock-free for maximum throughput
- **Inline Assembly Ready**: Can integrate RISC-V/ARM MMIO instructions
