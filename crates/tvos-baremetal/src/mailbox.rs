//! VideoCore IV Mailbox interface for Framebuffer allocation and display configuration

const MMIO_BASE: usize = 0x3F000000;
const MBOX_READ: *mut u32 = (MMIO_BASE + 0x0000B880) as *mut u32;
const MBOX_STATUS: *mut u32 = (MMIO_BASE + 0x0000B898) as *mut u32;
const MBOX_WRITE: *mut u32 = (MMIO_BASE + 0x0000B8A0) as *mut u32;

const MBOX_FULL: u32 = 0x80000000;
const MBOX_EMPTY: u32 = 0x40000000;
const MBOX_CHANNEL_PROPERTY: u32 = 8;

#[repr(C, align(16))]
pub struct MailboxBuffer<const N: usize>(pub [u32; N]);

pub unsafe fn mailbox_call(buffer: &mut [u32]) -> bool {
    let addr = buffer.as_ptr() as u32;
    let data = (addr & !0xF) | MBOX_CHANNEL_PROPERTY;

    // Wait until mailbox is not full
    while (MBOX_STATUS.read_volatile() & MBOX_FULL) != 0 {
        core::hint::spin_loop();
    }
    MBOX_WRITE.write_volatile(data);

    // Wait for response
    loop {
        while (MBOX_STATUS.read_volatile() & MBOX_EMPTY) != 0 {
            core::hint::spin_loop();
        }
        if MBOX_READ.read_volatile() == data {
            return buffer[1] == 0x80000000; // Success code
        }
    }
}

#[allow(dead_code)]
pub struct FramebufferConfig {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub fb_ptr: *mut u32,
}

pub unsafe fn init_framebuffer(width: u32, height: u32) -> Option<FramebufferConfig> {
    let mut msg = MailboxBuffer([
        35 * 4,     // Buffer size in bytes
        0,          // Request code
        // Tag: Set Physical Display
        0x00048003, 8, 8, width, height,
        // Tag: Set Virtual Display
        0x00048004, 8, 8, width, height,
        // Tag: Set Depth (32-bit ARGB)
        0x00048005, 4, 4, 32,
        // Tag: Set Pixel Order (RGB)
        0x00048006, 4, 4, 1,
        // Tag: Allocate Buffer
        0x00040001, 8, 8, 4096, 0,
        // Tag: Get Pitch
        0x00040008, 4, 4, 0,
        0, // End tag
    ]);

    if mailbox_call(&mut msg.0) {
        let fb_ptr = (msg.0[28] & 0x3FFFFFFF) as *mut u32;
        let pitch = msg.0[33];
        Some(FramebufferConfig {
            width,
            height,
            pitch,
            fb_ptr,
        })
    } else {
        None
    }
}
