# i9-12900K Bare-Metal Firmware ABI Specification v0.1

## 1. Target Architecture

**Processor:** Intel Core i9-12900K (Alder Lake)
**Architecture:** x86-64 (64-bit)
**Hybrid Design:**
- 8 P-cores (Performance, Golden Cove, cores 0-7, logical 0-15 with HT)
- 8 E-cores (Efficiency, Gracemont, cores 8-15, logical 16-23)

**ISA Features:**
- Base: x86-64-v3 (AVX2, BMI2, FMA)
- SIMD: SSE4.2, AVX, AVX2
- Extensions: AES-NI, PCLMULQDQ, RDRAND, RDSEED
- **AVX-512: Disabled** (incompatible when E-cores active)

---

## 2. Calling Convention

### 2.1 Function Parameters

Based on System V AMD64 ABI with optimizations:

**Integer/Pointer Parameters (in order):**
1. `rdi` (1st argument)
2. `rsi` (2nd argument)
3. `rdx` (3rd argument)
4. `rcx` (4th argument)
5. `r8` (5th argument)
6. `r9` (6th argument)
7. Stack (7th+ arguments, right-to-left)

**Floating-Point Parameters:**
1. `xmm0` through `xmm7` (first 8 FP args)
2. Stack (9th+ FP arguments)

**Vector Parameters (AVX2):**
1. `ymm0` through `ymm7` (256-bit vectors)

### 2.2 Return Values

**Integer/Pointer:** `rax` (primary), `rdx` (secondary for 128-bit)
**Floating-Point:** `xmm0` (primary), `xmm1` (secondary)
**Vector:** `ymm0` (256-bit AVX2)

### 2.3 Register Preservation

**Caller-Saved (Volatile):**
- General: `rax`, `rcx`, `rdx`, `rsi`, `rdi`, `r8`-`r11`
- FP/SIMD: `xmm0`-`xmm15`, `ymm0`-`ymm15`

**Callee-Saved (Non-Volatile):**
- General: `rbx`, `rbp`, `r12`-`r15`
- Special: `rsp` (stack pointer)

### 2.4 Stack Frame

```
High Address
┌─────────────────┐
│  Return Address │  ← Pushed by CALL
├─────────────────┤
│  Saved RBP      │  ← rbp (frame pointer)
├─────────────────┤
│  Local Vars     │
├─────────────────┤
│  Spilled Regs   │
├─────────────────┤
│  Arg 7+         │  ← Additional arguments
└─────────────────┘
Low Address (RSP)
```

**Stack Alignment:** 16-byte aligned before `CALL` instruction

### 2.5 Red Zone

**Status:** DISABLED (`disable-redzone: true`)
**Reason:** Bare-metal environment may use interrupts/exceptions

---

## 3. Core Affinity Extensions

### 3.1 Core Type Hints

Custom calling convention attributes:

```rust
#[core_affinity(Performance)]  // Must run on P-cores (0-7)
pub fn performance_critical_function() { }

#[core_affinity(Efficiency)]   // Prefer E-cores (8-15)
pub fn background_task() { }

#[core_affinity(Any)]           // Thread Director decides
pub fn general_function() { }
```

### 3.2 Core Affinity ABI

**Extended Registers for Scheduling Hints:**
- `r15` (reserved): Core affinity hint (when calling scheduler)
  - `0x0000`: Any core
  - `0x0001`: P-core required
  - `0x0002`: E-core preferred
  - `0x0003`: P-core with HT
  - `0x00FF`: Let Thread Director decide

---

## 4. System Call Interface

### 4.1 Syscall Convention

**Instruction:** `syscall` (AMD64 fast syscall)

**Registers:**
- `rax`: System call number
- `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`: Arguments 1-6
- `rcx`: Return address (saved by CPU)
- `r11`: RFLAGS (saved by CPU)

**Return:**
- `rax`: Return value (or negative errno)

### 4.2 System Call Numbers

```
0x00: exit
0x01: read_msr
0x02: write_msr
0x03: get_core_type
0x04: set_core_affinity
0x05: cache_flush
0x06: cache_invalidate
0x10: performance_counter_read
0x11: performance_counter_start
0x12: performance_counter_stop
```

---

## 5. Exception Handling

### 5.1 Exception Frame

```rust
#[repr(C)]
pub struct ExceptionStackFrame {
    pub instruction_pointer: u64,
    pub code_segment: u64,
    pub cpu_flags: u64,
    pub stack_pointer: u64,
    pub stack_segment: u64,
}
```

### 5.2 Exception Handlers

**Calling Convention:**
- No parameters passed via registers
- Exception frame on stack
- Handler must use `iretq` to return

---

## 6. Memory Layout

### 6.1 Virtual Address Space

```
0x0000_0000_0000_0000 - 0x0000_7FFF_FFFF_FFFF : User Space (disabled in bare-metal)
0xFFFF_8000_0000_0000 - 0xFFFF_87FF_FFFF_FFFF : Direct Physical Mapping
0xFFFF_8800_0000_0000 - 0xFFFF_88FF_FFFF_FFFF : Kernel Heap
0xFFFF_8900_0000_0000 - 0xFFFF_8FFF_FFFF_FFFF : Reserved
0xFFFF_9000_0000_0000 - 0xFFFF_9FFF_FFFF_FFFF : MMIO Regions
  ├─ 0xFFFF_9000_4000_0000 : L3 Cache Control
  ├─ 0xFFFF_9000_4010_0000 : Coherency Control
  ├─ 0xFFFF_9000_5000_0000 : Shadow Registers
  └─ 0xFFFF_9000_6000_0000 : Hardware Fuses
0xFFFF_FFFF_8000_0000 - 0xFFFF_FFFF_FFFF_FFFF : Kernel Code & Data
```

### 6.2 Page Size

**Standard:** 4KB pages
**Optional:** 2MB huge pages for performance-critical regions

---

## 7. Data Types & Alignment

### 7.1 Primitive Types

| Type | Size | Alignment |
|------|------|-----------|
| `bool` | 1 byte | 1 byte |
| `i8`/`u8` | 1 byte | 1 byte |
| `i16`/`u16` | 2 bytes | 2 bytes |
| `i32`/`u32` | 4 bytes | 4 bytes |
| `i64`/`u64` | 8 bytes | 8 bytes |
| `i128`/`u128` | 16 bytes | 16 bytes |
| `f32` | 4 bytes | 4 bytes |
| `f64` | 8 bytes | 8 bytes |
| `*const T`/`*mut T` | 8 bytes | 8 bytes |

### 7.2 Struct Alignment

**Default:** Natural alignment (largest member)
**Cache-Line Aligned:** Use `#[repr(align(64))]` for cache optimization

---

## 8. Performance Optimizations

### 8.1 Cache Line Optimization

**P-Core L1d Cache Line:** 64 bytes
**E-Core L1d Cache Line:** 64 bytes
**L2 Cache Line:** 64 bytes
**L3 Cache Line:** 64 bytes

**Recommendation:** Align hot data structures to 64-byte boundaries

### 8.2 Branch Prediction Hints

```rust
#[likely]      // Hint: likely branch
#[unlikely]    // Hint: unlikely branch (cold path)
#[cold]        // Function rarely called
#[hot]         // Function frequently called
```

### 8.3 Prefetch Instructions

**Available:**
- `prefetcht0` - L1 cache
- `prefetcht1` - L2 cache
- `prefetcht2` - L3 cache
- `prefetchnta` - Non-temporal (bypass cache)

---

## 9. Thread Director Integration

### 9.1 Hardware Feedback Interface (HFI)

**MSRs:**
- `MSR_HWP_REQUEST` (0x774): Hardware P-state request
- `MSR_TURBO_RATIO_LIMIT` (0x1AD): Turbo limits

### 9.2 Core Type Detection

**CPUID Leaf 0x1A:**
- EAX[31:24]: Core type (0x20 = Atom/E-core, 0x40 = Core/P-core)

---

## 10. ABI Versioning

**Version:** 0.1.0
**Compatibility:** Forward-compatible within minor versions
**Breaking Changes:** Major version increment

---

## 11. Toolchain Support

**Compiler:** rustc 1.94.0-nightly+ with target `x86_64-i9-12900k`
**Linker:** LLD (LLVM linker)
**Object Format:** ELF64
**Boot Protocol:** UEFI or Multiboot2

---

## References

- [Intel® 64 and IA-32 Architectures Software Developer's Manual](https://www.intel.com/content/dam/develop/external/us/en/documents/335592-sdm-vol-4.pdf)
- [System V AMD64 ABI](https://wiki.osdev.org/System_V_ABI)
- [x86-64 Calling Conventions](https://en.wikipedia.org/wiki/X86_calling_conventions)
- [Intel Core i9-12900K Architecture](https://www.techpowerup.com/review/intel-core-i9-12900k-alder-lake-12th-gen/2.html)
