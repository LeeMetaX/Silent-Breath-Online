; ============================================================================
; Silent-Breath-Online :: GRUB-bootable Multiboot2 stub
;
; GRUB hands control to `_start` in 32-bit protected mode with paging off,
; eax = 0x36d76289 (multiboot2 magic), ebx = physical address of boot info.
;
; This stub:
;   1. Validates the multiboot2 magic.
;   2. Zeroes BSS so page tables / stack start clean.
;   3. Builds an identity-mapped 1 GiB page hierarchy with 2 MiB pages.
;   4. Enables PAE, sets EFER.LME, enables CR0.PG -> long mode active.
;   5. Loads a flat 64-bit GDT and far-jumps to 64-bit code.
;   6. Calls Rust `kernel_main(multiboot_info_phys: u64)`.
; ============================================================================

bits 32

global _start
extern kernel_main
extern __bss_start
extern __bss_end

MULTIBOOT2_MAGIC          equ 0xe85250d6
MULTIBOOT2_BOOTLOADER_MAGIC equ 0x36d76289
MULTIBOOT2_ARCH_I386      equ 0

; ----------------------------------------------------------------------------
; Multiboot2 header (must live in the first 32 KiB of the file, 8-byte aligned)
; ----------------------------------------------------------------------------
section .multiboot_header
align 8
header_start:
    dd MULTIBOOT2_MAGIC
    dd MULTIBOOT2_ARCH_I386
    dd header_end - header_start
    dd 0x100000000 - (MULTIBOOT2_MAGIC + MULTIBOOT2_ARCH_I386 + (header_end - header_start))

    ; End tag (type=0, flags=0, size=8)
    align 8
    dw 0
    dw 0
    dd 8
header_end:

; ----------------------------------------------------------------------------
; BSS: page tables and boot stack
; ----------------------------------------------------------------------------
section .bss
align 4096
p4_table:
    resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096

align 16
stack_bottom:
    resb 16384            ; 16 KiB boot stack
stack_top:

; ----------------------------------------------------------------------------
; Read-only data: 64-bit GDT
; ----------------------------------------------------------------------------
section .rodata
align 8
gdt64:
    dq 0                                              ; null
.code_seg: equ $ - gdt64
    dq (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53)  ; exec | code/data | present | 64-bit
.data_seg: equ $ - gdt64
    dq (1 << 44) | (1 << 47) | (1 << 41)              ; code/data | present | writable
.pointer:
    dw $ - gdt64 - 1
    dq gdt64

; ----------------------------------------------------------------------------
; 32-bit boot code
; ----------------------------------------------------------------------------
section .text
_start:
    cli

    ; Verify multiboot2 bootloader magic.
    cmp eax, MULTIBOOT2_BOOTLOADER_MAGIC
    jne .hang32

    ; Preserve multiboot info pointer across BSS zero (ebp survives stosd).
    mov ebp, ebx

    ; Zero BSS: page tables and stack memory must start at 0.
    mov edi, __bss_start
    mov ecx, __bss_end
    sub ecx, edi
    add ecx, 3
    shr ecx, 2                ; round up to dwords
    xor eax, eax
    rep stosd

    ; Restore multiboot info ptr (BSS storage is now zeroed).
    mov [multiboot_info_addr], ebp

    ; Set up boot stack.
    mov esp, stack_top

    ; ------------------------------------------------------------------
    ; Build identity-mapped page tables: PML4[0] -> PDPT[0] -> PD (1 GiB)
    ; ------------------------------------------------------------------
    mov eax, p3_table
    or  eax, 0b11             ; PRESENT | WRITABLE
    mov [p4_table], eax

    mov eax, p2_table
    or  eax, 0b11             ; PRESENT | WRITABLE
    mov [p3_table], eax

    xor ecx, ecx              ; index 0..511
.fill_pd:
    mov eax, 0x200000         ; 2 MiB
    mul ecx                   ; eax = ecx * 2 MiB (edx clobbered, ok)
    or  eax, 0b10000011       ; PRESENT | WRITABLE | PAGE_SIZE (huge)
    mov [p2_table + ecx*8], eax
    inc ecx
    cmp ecx, 512
    jne .fill_pd

    ; Load CR3 with PML4 physical address.
    mov eax, p4_table
    mov cr3, eax

    ; Enable PAE (bit 5), OSFXSR (bit 9), OSXMMEXCPT (bit 10).
    ; OSFXSR is required before any SSE instructions; the x86_64 SysV ABI
    ; (and therefore rustc-generated code) uses XMM registers freely.
    mov eax, cr4
    or  eax, (1 << 5) | (1 << 9) | (1 << 10)
    mov cr4, eax

    ; Enable long mode (EFER.LME).
    mov ecx, 0xC0000080
    rdmsr
    or  eax, 1 << 8
    wrmsr

    ; Enable paging (CR0.PG) and write-protect (CR0.WP).
    mov eax, cr0
    or  eax, (1 << 31) | (1 << 16)
    mov cr0, eax

    ; CPU is now in IA-32e compatibility mode. Load 64-bit GDT and jump.
    lgdt [gdt64.pointer]
    jmp gdt64.code_seg:long_mode_start

.hang32:
    hlt
    jmp .hang32

; ----------------------------------------------------------------------------
; 64-bit code
; ----------------------------------------------------------------------------
bits 64
long_mode_start:
    ; Reload data segment registers with the data selector (0 also legal in
    ; 64-bit mode, but a real selector is friendlier to debuggers).
    mov ax, gdt64.data_seg
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Re-establish the stack with a 64-bit pointer.
    mov rsp, stack_top

    ; Load multiboot info physical address (zero-extended via .bss reservation).
    mov rdi, [multiboot_info_addr]

    call kernel_main

.hang64:
    cli
    hlt
    jmp .hang64

; ----------------------------------------------------------------------------
; Storage for the multiboot info pointer that survives BSS zeroing.
; ----------------------------------------------------------------------------
section .bss
align 8
multiboot_info_addr:
    resq 1
