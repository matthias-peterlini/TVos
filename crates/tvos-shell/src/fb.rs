//! Linux framebuffer (/dev/fb0) output. Works with fbdev, simpledrm and vc4 fbdev emulation.
//!
//! Video memory is uncached and slow to write, so `present` converts each row in RAM,
//! skips rows that did not change since the last frame, and copies whole rows at once.

use std::fs::OpenOptions;
use std::io;
use std::os::unix::io::AsRawFd;

const FBIOGET_VSCREENINFO: u32 = 0x4600;
const FBIOGET_FSCREENINFO: u32 = 0x4602;

pub struct Fb {
    pub width: usize,
    pub height: usize,
    bpp: usize,
    stride: usize,
    map: *mut u8,
    red_shift: u32,
    green_shift: u32,
    blue_shift: u32,
    /// Last frame sent to the screen (ARGB), used to skip unchanged rows
    prev: Vec<u32>,
    row: Vec<u8>,
    _file: std::fs::File,
}

impl Fb {
    pub fn open() -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open("/dev/fb0")?;
        let fd = file.as_raw_fd();
        let mut var = [0u32; 40];
        let mut fix = [0u8; 80];
        unsafe {
            if libc::ioctl(fd, FBIOGET_VSCREENINFO as _, var.as_mut_ptr()) < 0
                || libc::ioctl(fd, FBIOGET_FSCREENINFO as _, fix.as_mut_ptr()) < 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        let width = var[0] as usize;
        let height = var[1] as usize;
        let bpp = var[6] as usize;
        let stride = u32::from_ne_bytes(fix[48..52].try_into().unwrap()) as usize;
        let smem_len = u32::from_ne_bytes(fix[24..28].try_into().unwrap()) as usize;
        let len = smem_len.max(stride * height);
        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if map == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            width,
            height,
            bpp,
            stride,
            map: map as *mut u8,
            red_shift: var[8],
            green_shift: var[11],
            blue_shift: var[14],
            // 0x01000000 never occurs in a rendered frame (alpha is always 0xFF), so the first present writes everything
            prev: vec![0x0100_0000; width * height],
            row: vec![0; width * 4],
            _file: file,
        })
    }

    /// Forget what is on screen so the next `present` rewrites every row.
    pub fn invalidate(&mut self) {
        self.prev.fill(0x0100_0000);
    }

    /// Copy an ARGB32 buffer (same size as the framebuffer) to the screen. Returns rows written.
    pub fn present(&mut self, buf: &[u32]) -> usize {
        let w = self.width;
        let bytes_pp = self.bpp / 8;
        let native_xrgb = self.bpp == 32 && (self.red_shift, self.green_shift, self.blue_shift) == (16, 8, 0);
        let mut written = 0;
        for y in 0..self.height {
            let src = &buf[y * w..(y + 1) * w];
            if self.prev[y * w..(y + 1) * w] == *src {
                continue;
            }
            self.prev[y * w..(y + 1) * w].copy_from_slice(src);
            let (ptr, len): (*const u8, usize) = if native_xrgb {
                // Same byte layout as our buffer: copy straight from it
                (src.as_ptr() as *const u8, w * 4)
            } else {
                let out = &mut self.row;
                match self.bpp {
                    32 | 24 => {
                        for (x, p) in src.iter().enumerate() {
                            let v = ((p >> 16 & 0xFF) << self.red_shift)
                                | ((p >> 8 & 0xFF) << self.green_shift)
                                | ((p & 0xFF) << self.blue_shift);
                            out[x * bytes_pp..x * bytes_pp + bytes_pp].copy_from_slice(&v.to_le_bytes()[..bytes_pp]);
                        }
                    }
                    16 => {
                        for (x, p) in src.iter().enumerate() {
                            let v = (((p >> 19) & 0x1F) << 11 | ((p >> 10) & 0x3F) << 5 | ((p >> 3) & 0x1F)) as u16;
                            out[x * 2..x * 2 + 2].copy_from_slice(&v.to_le_bytes());
                        }
                    }
                    _ => return 0,
                }
                (out.as_ptr(), w * bytes_pp)
            };
            unsafe { std::ptr::copy_nonoverlapping(ptr, self.map.add(y * self.stride), len) };
            written += 1;
        }
        written
    }
}
