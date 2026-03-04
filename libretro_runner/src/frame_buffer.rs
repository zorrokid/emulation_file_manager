use std::os::raw::c_void;

use crate::ffi::RetroPixelFormat;

pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    // Pixel data stored as Cairo ARgb32: 4 bytes per pixel, [B, G, R, A] in
    // memory on little-endian x86_64. This is the format Cairo's ImageSurface
    // expects, so we can hand this buffer directly to GTK for drawing.
    pub rgba_data: Vec<u8>,
    // Set to true whenever new pixel data arrives. The GTK draw callback
    // reads this to know whether to repaint.
    pub dirty: bool,
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            rgba_data: Vec::new(),
            dirty: false,
        }
    }

    /// Called from video_refresh_cb. Converts core pixel format to Cairo ARgb32.
    ///
    /// Cairo ARgb32 on little-endian x86_64 stores pixels as [B, G, R, A] in memory.
    /// XRGB8888 from the core is [B, G, R, 0x00] — same byte order, just set alpha to 0xFF.
    ///
    /// `pitch` is the number of bytes per row in the source buffer. It may be
    /// wider than `width * bytes_per_pixel` if the core pads rows for alignment.
    /// We use it as the row stride when indexing into the source data.
    pub fn update(
        &mut self,
        // Raw pointer to the pixel data the core just rendered.
        // *const c_void because the core gives us a type-erased pointer —
        // we cast it to *const u8 below to read individual bytes.
        data: *const c_void,
        width: u32,
        height: u32,
        pitch: usize,
        format: RetroPixelFormat,
    ) {
        self.width = width;
        self.height = height;

        let total = (width * height * 4) as usize;
        if self.rgba_data.len() != total {
            self.rgba_data.resize(total, 0);
        }

        // Reinterpret the void pointer as a byte pointer so we can read
        // individual bytes. This is safe as long as the core gave us valid
        // pixel data, which it guarantees by the libretro contract.
        let src = data as *const u8;

        for row in 0..height as usize {
            for col in 0..width as usize {
                // Destination offset in our RGBA output buffer: 4 bytes per pixel, row-major.
                let dst = (row * width as usize + col) * 4;
                unsafe {
                    match format {
                        RetroPixelFormat::Xrgb8888 => {
                            // Source layout: [B, G, R, 0x00] (4 bytes per pixel).
                            // pitch is used instead of width*4 in case the core
                            // adds padding bytes at the end of each row.
                            let s = row * pitch + col * 4;
                            self.rgba_data[dst] = *src.add(s);         // B
                            self.rgba_data[dst + 1] = *src.add(s + 1); // G
                            self.rgba_data[dst + 2] = *src.add(s + 2); // R
                            self.rgba_data[dst + 3] = 0xFF;            // A — set opaque
                        }
                        RetroPixelFormat::Rgb565 => {
                            // Source layout: 2 bytes per pixel, packed as:
                            // bits 15-11 = R (5 bits), 10-5 = G (6 bits), 4-0 = B (5 bits).
                            // We read two bytes and reassemble the 16-bit value.
                            let s = row * pitch + col * 2;
                            let lo = *src.add(s) as u16;
                            let hi = *src.add(s + 1) as u16;
                            let px = lo | (hi << 8);
                            let r = ((px >> 11) & 0x1F) as u8;
                            let g = ((px >> 5) & 0x3F) as u8;
                            let b = (px & 0x1F) as u8;
                            // Scale 5-bit (0-31) and 6-bit (0-63) values up to 8-bit (0-255).
                            // The `| (x >> n)` trick replicates the high bits into the low bits
                            // so the full range maps correctly (e.g. 0x1F → 0xFF, not 0xF8).
                            self.rgba_data[dst] = (b << 3) | (b >> 2);
                            self.rgba_data[dst + 1] = (g << 2) | (g >> 4);
                            self.rgba_data[dst + 2] = (r << 3) | (r >> 2);
                            self.rgba_data[dst + 3] = 0xFF;
                        }
                        RetroPixelFormat::Rgb1555 => {
                            // Source layout: 2 bytes per pixel, packed as:
                            // bit 15 = ignored, 14-10 = R, 9-5 = G, 4-0 = B (all 5 bits).
                            let s = row * pitch + col * 2;
                            let lo = *src.add(s) as u16;
                            let hi = *src.add(s + 1) as u16;
                            let px = lo | (hi << 8);
                            let r = ((px >> 10) & 0x1F) as u8;
                            let g = ((px >> 5) & 0x1F) as u8;
                            let b = (px & 0x1F) as u8;
                            self.rgba_data[dst] = (b << 3) | (b >> 2);
                            self.rgba_data[dst + 1] = (g << 3) | (g >> 2);
                            self.rgba_data[dst + 2] = (r << 3) | (r >> 2);
                            self.rgba_data[dst + 3] = 0xFF;
                        }
                    }
                }
            }
        }

        self.dirty = true;
    }
}
