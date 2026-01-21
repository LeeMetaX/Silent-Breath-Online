/* Linker script for i9-12900K bare-metal kernel with Multiboot2 support */

ENTRY(_start)

SECTIONS
{
    /* Kernel starts at 1 MiB (standard for higher-half kernels) */
    . = 1M;

    /* Multiboot2 header must be in the first 32 KiB */
    .multiboot2 ALIGN(8) : {
        KEEP(*(.multiboot2))
    }

    /* Read-only sections */
    .text ALIGN(4K) : {
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
    }

    /* Read-write sections */
    .data ALIGN(4K) : {
        *(.data .data.*)
    }

    .bss ALIGN(4K) : {
        *(.bss .bss.*)
        *(COMMON)
    }

    /* Stack (16 KiB) */
    .stack ALIGN(4K) : {
        . += 16K;
        stack_top = .;
    }

    /* Discard sections we don't need */
    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
    }
}
