//! Renderizador de vídeo

pub struct RenderBuffer {
    pub pixels: Vec<u32>,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum PixelFormat {
    ARGB8888,
    RGB565,
    Indexed8,
}

pub struct Renderer {
    format: PixelFormat,
}

impl RenderBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            pixels: vec![0; width * height],
            width,
            height,
        }
    }

    pub fn clear(&mut self, color: u32) {
        for pixel in &mut self.pixels {
            *pixel = color;
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x]
        } else {
            0
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = color;
        }
    }
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            format: PixelFormat::ARGB8888,
        }
    }

    pub fn set_format(&mut self, format: PixelFormat) {
        self.format = format;
    }

    pub fn convert_to_rgb(&self, buffer: &RenderBuffer) -> Vec<u8> {
        let mut result = Vec::with_capacity(buffer.pixels.len() * 3);

        for &pixel in &buffer.pixels {
            result.push(((pixel >> 16) & 0xFF) as u8); // R
            result.push(((pixel >> 8) & 0xFF) as u8); // G
            result.push((pixel & 0xFF) as u8); // B
        }

        result
    }
}
