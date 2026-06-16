/*
 * Silent-Breath-Online :: GRUB stub linker script
 *
 * GRUB loads the kernel image at 1 MiB. The Multiboot2 header must live in
 * the first 32 KiB of the file, so we place .multiboot_header at the very
 * start. Code, rodata, data, and bss follow with 4 KiB alignment.
 */

ENTRY(_start)

SECTIONS
{
    . = 1M;

    .boot ALIGN(4K) :
    {
        KEEP(*(.multiboot_header))
    }

    .text ALIGN(4K) :
    {
        *(.text .text.*)
    }

    .rodata ALIGN(4K) :
    {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K) :
    {
        *(.data .data.*)
    }

    .bss ALIGN(4K) :
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(8);
        __bss_end = .;
    }

    /DISCARD/ :
    {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.note .note.*)
        *(.comment)
    }
}
