//! BCM2837 Mini UART driver for early serial boot diagnostics

const MMIO_BASE: usize = 0x3F000000;
const AUX_ENABLES: *mut u32 = (MMIO_BASE + 0x00215004) as *mut u32;
const AUX_MU_IO_REG: *mut u32 = (MMIO_BASE + 0x00215040) as *mut u32;
const AUX_MU_LSR_REG: *mut u32 = (MMIO_BASE + 0x00215054) as *mut u32;
const AUX_MU_CNTL_REG: *mut u32 = (MMIO_BASE + 0x00215060) as *mut u32;
const AUX_MU_BAUD_REG: *mut u32 = (MMIO_BASE + 0x00215068) as *mut u32;

pub struct Uart;

impl Uart {
    pub unsafe fn init() {
        // Enable Mini UART
        AUX_ENABLES.write_volatile(AUX_ENABLES.read_volatile() | 1);
        AUX_MU_CNTL_REG.write_volatile(0); // Disable TX/RX during setup
        AUX_MU_BAUD_REG.write_volatile(270); // 115200 baud at 250MHz VPU clock
        AUX_MU_CNTL_REG.write_volatile(3); // Enable TX/RX
    }

    pub fn putc(c: u8) {
        unsafe {
            // Wait until transmitter has space
            while (AUX_MU_LSR_REG.read_volatile() & 0x20) == 0 {
                core::hint::spin_loop();
            }
            AUX_MU_IO_REG.write_volatile(c as u32);
        }
    }

    pub fn puts(s: &str) {
        for b in s.bytes() {
            if b == b'\n' {
                Self::putc(b'\r');
            }
            Self::putc(b);
        }
    }
}
