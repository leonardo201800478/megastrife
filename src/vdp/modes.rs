//! Implementações específicas dos modos de vídeo

use super::*;

impl VDP {
    /// Renderiza frame completo no Modo 5
    pub fn render_mode5_frame(&mut self) -> Result<()> {
        for line in 0..GENESIS_HEIGHT as u16 {
            self.render_mode5_scanline(line)?;
        }
        Ok(())
    }

    /// Renderiza frame completo no Modo 4
    pub fn render_mode4_frame(&mut self) -> Result<()> {
        for line in 0..GENESIS_HEIGHT as u16 {
            self.render_mode4_scanline(line)?;
        }
        Ok(())
    }
}

// Implementações específicas do Modo 4
impl VDP {
    fn render_mode4_scanline(&mut self, line: u16) -> Result<()> {
        // Similar ao Mode5 mas com resolução 256x224
        for x in 0..256 {
            let mut color: u32 = 0;

            // Lógica de renderização similar
            if self.layer_enable.background_a {
                color = self.render_background_pixel(x, line, 0, true)?;
            }

            let idx: usize = (line as usize * 256) + x as usize;
            if idx < self.framebuffer.pixels.len() {
                self.framebuffer.pixels[idx] = color;
            }
        }

        Ok(())
    }
}
