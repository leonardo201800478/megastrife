/// Representa o framebuffer do VDP — uma imagem completa renderizada em RGBA.
#[derive(Clone)]
pub struct FrameBuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>, // Cada pixel em formato 0xAARRGGBB
}

impl FrameBuffer {
    /// Cria um novo framebuffer limpo (preto).
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0x00000000; width * height],
        }
    }

    /// Limpa o framebuffer para uma cor uniforme.
    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    /// Define um pixel individual (com bounds check).
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            let index = y * self.width + x;
            self.pixels[index] = color;
        }
    }

    /// Retorna uma cópia do buffer como `Vec<u32>` (para o renderizador principal).
    pub fn as_vec(&self) -> Vec<u32> {
        self.pixels.clone()
    }
}
