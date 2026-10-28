#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(include_str!("boot.S"));

mod mailbox;
mod uart;

use mailbox::init_framebuffer;
use uart::Uart;

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    unsafe {
        Uart::init();
    }

    Uart::puts("\n==================================================\n");
    Uart::puts("  TVos Bare-Metal Kernel v0.1.0-alpha             \n");
    Uart::puts("  Target: Raspberry Pi 3B (BCM2837 ARMv8 64-bit) \n");
    Uart::puts("  UI Theme: login.ticly.it Glassmorphism          \n");
    Uart::puts("==================================================\n\n");

    Uart::puts("[BOOT] Inizializzazione Framebuffer HDMI via Mailbox VideoCore...\n");

    let fb = unsafe { init_framebuffer(1920, 1080) };

    match fb {
        Some(config) => {
            Uart::puts("[OK] Framebuffer HDMI inizializzato con successo: 1920x1080x32bpp\n");

            // Fill background with #07090e (Theme::BG_BASE)
            let base_color = 0xFF07090E;
            let total_pixels = (config.width * config.height) as usize;
            unsafe {
                for i in 0..total_pixels {
                    config.fb_ptr.add(i).write_volatile(base_color);
                }
            }
            Uart::puts("[OK] Schermata di avvio TVos visualizzata su TV.\n");
        }
        None => {
            Uart::puts("[ERRORE] Fallita allocazione Framebuffer VideoCore IV!\n");
        }
    }

    Uart::puts("[IDLE] Loop principale kernel attivo.\n");

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    Uart::puts("\n[PANIC KERNEL] Si e' verificato un panic non gestito!\n");
    loop {
        core::hint::spin_loop();
    }
}
