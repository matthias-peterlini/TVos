// AArch64 Bootloader Assembly for Raspberry Pi 3B (BCM2837)
.section ".text.boot"

.global _start

_start:
    // Read Multiprocessor Affinity Register (MPIDR_EL1)
    mrs     x0, mpidr_el1
    and     x0, x0, #0xFF
    cbz     x0, master_core

park_core:
    // Park secondary CPU cores (1, 2, 3) in low-power wait-for-event loop
    wfe
    b       park_core

master_core:
    // Initialize Stack Pointer
    ldr     x1, =_stack_top
    mov     sp, x1

    // Clear .bss segment with zeros
    ldr     x1, =__bss_start
    ldr     x2, =__bss_end
clear_bss:
    cmp     x1, x2
    b.ge    jump_rust
    str     xzr, [x1], #8
    b       clear_bss

jump_rust:
    // Call Rust kernel entry point
    bl      kernel_main

hang:
    wfe
    b       hang
