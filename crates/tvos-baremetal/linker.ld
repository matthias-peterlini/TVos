ENTRY(_start)

SECTIONS
{
    /* Raspberry Pi 3B boots 64-bit kernel at physical address 0x80000 */
    . = 0x80000;

    .text.boot : {
        KEEP(*(.text.boot))
    }

    .text : {
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    . = ALIGN(16);
    __bss_start = .;
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }
    . = ALIGN(16);
    __bss_end = .;

    /* 4 MB Kernel Stack */
    . = ALIGN(16);
    . += 0x400000;
    _stack_top = .;
}
