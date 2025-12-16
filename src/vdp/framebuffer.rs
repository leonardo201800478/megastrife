//! Framebuffer do VDP - Buffer de renderização em RGBA
//!
//! Representa a imagem completa renderizada pelo VDP, com suporte para:
//! - Vários modos de resolução do Mega Drive (320x224, 320x240, 256x224, etc.)
//! - Operações de desenho eficientes
//! - Composição de layers (planos, sprites)
//! - Efeitos de transparência e blend
//! - Serialização/deserialização para save states

#[derive(Clone)]
pub struct FrameBuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>, // Formato ARGB: 0xAARRGGBB (alpha, red, green, blue)
    pub dirty: bool,      // Flag para otimização (buffer alterado desde último render)
}

impl FrameBuffer {
    // Resoluções padrão do Mega Drive
    pub const WIDTH_256: usize = 256;
    pub const WIDTH_320: usize = 320;
    pub const HEIGHT_224: usize = 224;
    pub const HEIGHT_240: usize = 240;
    pub const HEIGHT_256: usize = 256;
    
    /// Cria um novo framebuffer preto com tamanho especificado
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0xFF000000; width * height], // Preto com alpha máximo
            dirty: true,
        }
    }
    
    /// Cria um framebuffer com tamanho padrão do Mega Drive (320x224)
    pub fn new_standard() -> Self {
        Self::new(Self::WIDTH_320, Self::HEIGHT_224)
    }
    
    /// Cria um framebuffer a partir de dados existentes
    pub fn from_pixels(width: usize, height: usize, pixels: Vec<u32>) -> Option<Self> {
        if pixels.len() == width * height {
            Some(Self {
                width,
                height,
                pixels,
                dirty: true,
            })
        } else {
            None
        }
    }
    
    // =====================================================
    // OPERAÇÕES BÁSICAS
    // =====================================================
    
    /// Limpa o framebuffer com uma cor sólida
    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color | 0xFF000000); // Garante alpha máximo
        self.dirty = true;
    }
    
    /// Limpa com preto transparente (alpha 0)
    pub fn clear_transparent(&mut self) {
        self.pixels.fill(0x00000000);
        self.dirty = true;
    }
    
    /// Define um pixel individual com bounds checking
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) -> bool {
        if x < self.width && y < self.height {
            let index = y * self.width + x;
            self.pixels[index] = color;
            self.dirty = true;
            true
        } else {
            false
        }
    }
    
    /// Define um pixel sem bounds checking (mais rápido)
    pub unsafe fn set_pixel_unchecked(&mut self, x: usize, y: usize, color: u32) {
        *self.pixels.get_unchecked_mut(y * self.width + x) = color;
        self.dirty = true;
    }
    
    /// Obtém a cor de um pixel
    pub fn get_pixel(&self, x: usize, y: usize) -> Option<u32> {
        if x < self.width && y < self.height {
            Some(self.pixels[y * self.width + x])
        } else {
            None
        }
    }
    
    /// Obtém um pixel sem bounds checking (mais rápido)
    pub unsafe fn get_pixel_unchecked(&self, x: usize, y: usize) -> u32 {
        *self.pixels.get_unchecked(y * self.width + x)
    }
    
    // =====================================================
    // OPERAÇÕES DE DESENHO
    // =====================================================
    
    /// Desenha uma linha horizontal
    pub fn draw_horizontal_line(&mut self, x1: usize, x2: usize, y: usize, color: u32) {
        if y >= self.height { return; }
        
        let start = x1.min(x2).clamp(0, self.width - 1);
        let end = x1.max(x2).clamp(0, self.width - 1);
        let row_start = y * self.width;
        
        for x in start..=end {
            self.pixels[row_start + x] = color;
        }
        self.dirty = true;
    }
    
    /// Desenha uma linha vertical
    pub fn draw_vertical_line(&mut self, x: usize, y1: usize, y2: usize, color: u32) {
        if x >= self.width { return; }
        
        let start = y1.min(y2).clamp(0, self.height - 1);
        let end = y1.max(y2).clamp(0, self.height - 1);
        
        for y in start..=end {
            self.pixels[y * self.width + x] = color;
        }
        self.dirty = true;
    }
    
    /// Desenha um retângulo (borda apenas)
    pub fn draw_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: u32) {
        if x >= self.width || y >= self.height { return; }
        
        let x2 = (x + width).min(self.width - 1);
        let y2 = (y + height).min(self.height - 1);
        
        self.draw_horizontal_line(x, x2, y, color);
        self.draw_horizontal_line(x, x2, y2, color);
        self.draw_vertical_line(x, y, y2, color);
        self.draw_vertical_line(x2, y, y2, color);
    }
    
    /// Desenha um retângulo preenchido
    pub fn fill_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: u32) {
        if x >= self.width || y >= self.height { return; }
        
        let x2 = (x + width).min(self.width - 1);
        let y2 = (y + height).min(self.height - 1);
        
        for row in y..=y2 {
            let row_start = row * self.width;
            for col in x..=x2 {
                self.pixels[row_start + col] = color;
            }
        }
        self.dirty = true;
    }
    
    /// Copia um retângulo de outro framebuffer
    pub fn blit(&mut self, src: &FrameBuffer, src_x: usize, src_y: usize, 
                width: usize, height: usize, dest_x: usize, dest_y: usize) {
        if dest_x >= self.width || dest_y >= self.height { return; }
        if src_x >= src.width || src_y >= src.height { return; }
        
        let copy_width = width.min(self.width - dest_x).min(src.width - src_x);
        let copy_height = height.min(self.height - dest_y).min(src.height - src_y);
        
        for y in 0..copy_height {
            let src_row = (src_y + y) * src.width;
            let dst_row = (dest_y + y) * self.width;
            
            for x in 0..copy_width {
                self.pixels[dst_row + dest_x + x] = src.pixels[src_row + src_x + x];
            }
        }
        self.dirty = true;
    }
    
    /// Copia com transparência (pixels com alpha = 0 são ignorados)
    pub fn blit_transparent(&mut self, src: &FrameBuffer, src_x: usize, src_y: usize,
                           width: usize, height: usize, dest_x: usize, dest_y: usize) {
        if dest_x >= self.width || dest_y >= self.height { return; }
        if src_x >= src.width || src_y >= src.height { return; }
        
        let copy_width = width.min(self.width - dest_x).min(src.width - src_x);
        let copy_height = height.min(self.height - dest_y).min(src.height - src_y);
        
        for y in 0..copy_height {
            let src_row = (src_y + y) * src.width;
            let dst_row = (dest_y + y) * self.width;
            
            for x in 0..copy_width {
                let src_pixel = src.pixels[src_row + src_x + x];
                if (src_pixel >> 24) != 0 { // Alpha != 0
                    self.pixels[dst_row + dest_x + x] = src_pixel;
                }
            }
        }
        self.dirty = true;
    }
    
    /// Preenche uma região com um padrão de tile
    pub fn fill_with_tile(&mut self, tile: &[u32], tile_width: usize, tile_height: usize,
                          x: usize, y: usize, width: usize, height: usize) {
        if x >= self.width || y >= self.height || tile.len() != tile_width * tile_height {
            return;
        }
        
        let tile_x_end = x + width;
        let tile_y_end = y + height;
        
        for ty in y..tile_y_end {
            if ty >= self.height { break; }
            let tile_y = (ty - y) % tile_height;
            let tile_row = tile_y * tile_width;
            let dst_row = ty * self.width;
            
            for tx in x..tile_x_end {
                if tx >= self.width { break; }
                let tile_x = (tx - x) % tile_width;
                self.pixels[dst_row + tx] = tile[tile_row + tile_x];
            }
        }
        self.dirty = true;
    }
    
    // =====================================================
    // OPERAÇÕES DE COR
    // =====================================================
    
    /// Aplica uma paleta de cores ao framebuffer
    pub fn apply_palette(&mut self, palette: &[u32; 256]) {
        for pixel in &mut self.pixels {
            let index = (*pixel & 0xFF) as usize;
            if index < 256 {
                *pixel = palette[index];
            }
        }
        self.dirty = true;
    }
    
    /// Converte para escala de cinza
    pub fn convert_to_grayscale(&mut self) {
        for pixel in &mut self.pixels {
            let a = (*pixel >> 24) & 0xFF;
            let r = (*pixel >> 16) & 0xFF;
            let g = (*pixel >> 8) & 0xFF;
            let b = *pixel & 0xFF;
            
            // Luminosidade: 0.299*R + 0.587*G + 0.114*B
            let gray = ((r as f32 * 0.299) + (g as f32 * 0.587) + (b as f32 * 0.114)) as u32;
            *pixel = (a << 24) | (gray << 16) | (gray << 8) | gray;
        }
        self.dirty = true;
    }
    
    /// Aplica brilho/contraste
    pub fn adjust_brightness_contrast(&mut self, brightness: f32, contrast: f32) {
        let contrast_factor = (259.0 * (contrast + 255.0)) / (255.0 * (259.0 - contrast));
        
        for pixel in &mut self.pixels {
            let a = (*pixel >> 24) & 0xFF;
            let r = (*pixel >> 16) & 0xFF;
            let g = (*pixel >> 8) & 0xFF;
            let b = *pixel & 0xFF;
            
            let adjust = |c: u32| -> u32 {
                let c_f32 = c as f32;
                let adjusted = contrast_factor * (c_f32 - 128.0) + 128.0 + brightness;
                adjusted.clamp(0.0, 255.0) as u32
            };
            
            let new_r = adjust(r);
            let new_g = adjust(g);
            let new_b = adjust(b);
            
            *pixel = (a << 24) | (new_r << 16) | (new_g << 8) | new_b;
        }
        self.dirty = true;
    }
    
    /// Aplica um filtro de scanlines (para efeito de monitor CRT)
    pub fn apply_scanlines(&mut self, scanline_strength: f32) {
        let scanline_value = (scanline_strength.clamp(0.0, 1.0) * 255.0) as u32;
        
        for y in 0..self.height {
            if y % 2 == 0 { // Linhas pares ficam mais escuras
                let row_start = y * self.width;
                for x in 0..self.width {
                    let pixel = &mut self.pixels[row_start + x];
                    
                    let a = (*pixel >> 24) & 0xFF;
                    let r = (*pixel >> 16) & 0xFF;
                    let g = (*pixel >> 8) & 0xFF;
                    let b = *pixel & 0xFF;
                    
                    let darken = |c: u32| -> u32 {
                        ((c as f32 * (255.0 - scanline_value as f32) / 255.0) as u32).clamp(0, 255)
                    };
                    
                    let new_r = darken(r);
                    let new_g = darken(g);
                    let new_b = darken(b);
                    
                    *pixel = (a << 24) | (new_r << 16) | (new_g << 8) | new_b;
                }
            }
        }
        self.dirty = true;
    }
    
    // =====================================================
    // OPERAÇÕES DE CONVERSÃO
    // =====================================================
    
    /// Converte para vetor de bytes RGBA
    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
        for &pixel in &self.pixels {
            bytes.push((pixel >> 16) as u8); // R
            bytes.push((pixel >> 8) as u8);  // G
            bytes.push(pixel as u8);         // B
            bytes.push((pixel >> 24) as u8); // A
        }
        bytes
    }
    
    /// Converte de bytes RGBA
    pub fn from_rgba_bytes(width: usize, height: usize, bytes: &[u8]) -> Option<Self> {
        if bytes.len() != width * height * 4 {
            return None;
        }
        
        let mut pixels = Vec::with_capacity(width * height);
        for chunk in bytes.chunks_exact(4) {
            let r = chunk[0] as u32;
            let g = chunk[1] as u32;
            let b = chunk[2] as u32;
            let a = chunk[3] as u32;
            pixels.push((a << 24) | (r << 16) | (g << 8) | b);
        }
        
        Some(Self {
            width,
            height,
            pixels,
            dirty: true,
        })
    }
    
    /// Converte para vetor de u32 (formato ARGB)
    pub fn to_argb_vec(&self) -> Vec<u32> {
        self.pixels.clone()
    }
    
    /// Cria uma miniatura (thumbnail) do framebuffer
    pub fn create_thumbnail(&self, thumb_width: usize, thumb_height: usize) -> FrameBuffer {
        let mut thumbnail = FrameBuffer::new(thumb_width, thumb_height);
        
        if self.width == 0 || self.height == 0 {
            return thumbnail;
        }
        
        let x_ratio = self.width as f32 / thumb_width as f32;
        let y_ratio = self.height as f32 / thumb_height as f32;
        
        for y in 0..thumb_height {
            let src_y = (y as f32 * y_ratio) as usize;
            let src_row = src_y * self.width;
            let dst_row = y * thumb_width;
            
            for x in 0..thumb_width {
                let src_x = (x as f32 * x_ratio) as usize;
                thumbnail.pixels[dst_row + x] = self.pixels[src_row + src_x];
            }
        }
        
        thumbnail
    }
    
    // =====================================================
    // OPERAÇÕES DE UTILIDADE
    // =====================================================
    
    /// Retorna se o framebuffer está marcado como sujo (alterado)
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// Marca o framebuffer como limpo (renderizado)
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
    
    /// Retorna uma referência imutável aos pixels
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }
    
    /// Retorna uma referência mutável aos pixels
    pub fn pixels_mut(&mut self) -> &mut [u32] {
        self.dirty = true;
        &mut self.pixels
    }
    
    /// Retorna o tamanho em pixels
    pub fn size(&self) -> usize {
        self.width * self.height
    }
    
    /// Redimensiona o framebuffer (mantém o conteúdo o melhor possível)
    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        if new_width == self.width && new_height == self.height {
            return;
        }
        
        let mut new_pixels = vec![0xFF000000; new_width * new_height];
        
        let copy_width = self.width.min(new_width);
        let copy_height = self.height.min(new_height);
        
        for y in 0..copy_height {
            let src_row = y * self.width;
            let dst_row = y * new_width;
            new_pixels[dst_row..dst_row + copy_width]
                .copy_from_slice(&self.pixels[src_row..src_row + copy_width]);
        }
        
        self.width = new_width;
        self.height = new_height;
        self.pixels = new_pixels;
        self.dirty = true;
    }
    
    /// Inverte verticalmente (útil para sistemas com coordenadas Y invertidas)
    pub fn flip_vertical(&mut self) {
        let row_size = self.width;
        let half_height = self.height / 2;
        
        for y in 0..half_height {
            let top_row = y * row_size;
            let bottom_row = (self.height - 1 - y) * row_size;
            
            for x in 0..row_size {
                self.pixels.swap(top_row + x, bottom_row + x);
            }
        }
        self.dirty = true;
    }
    
    /// Inverte horizontalmente
    pub fn flip_horizontal(&mut self) {
        let row_size = self.width;
        let half_width = self.width / 2;
        
        for y in 0..self.height {
            let row_start = y * row_size;
            
            for x in 0..half_width {
                let left_idx = row_start + x;
                let right_idx = row_start + (self.width - 1 - x);
                self.pixels.swap(left_idx, right_idx);
            }
        }
        self.dirty = true;
    }
    
    /// Rotaciona 90 graus no sentido horário
    pub fn rotate_90_cw(&mut self) {
        let mut new_pixels = vec![0xFF000000; self.width * self.height];
        
        for y in 0..self.height {
            for x in 0..self.width {
                let src_idx = y * self.width + x;
                let dst_idx = x * self.height + (self.height - 1 - y);
                new_pixels[dst_idx] = self.pixels[src_idx];
            }
        }
        
        std::mem::swap(&mut self.width, &mut self.height);
        self.pixels = new_pixels;
        self.dirty = true;
    }
}

impl Default for FrameBuffer {
    fn default() -> Self {
        Self::new_standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framebuffer_creation() {
        let fb = FrameBuffer::new(320, 224);
        assert_eq!(fb.width, 320);
        assert_eq!(fb.height, 224);
        assert_eq!(fb.pixels.len(), 320 * 224);
        assert!(fb.is_dirty());
        
        let fb_std = FrameBuffer::new_standard();
        assert_eq!(fb_std.width, 320);
        assert_eq!(fb_std.height, 224);
    }

    #[test]
    fn test_pixel_operations() {
        let mut fb = FrameBuffer::new(10, 10);
        
        // Teste set/get pixel
        assert!(fb.set_pixel(5, 5, 0xFF00FF00));
        assert_eq!(fb.get_pixel(5, 5), Some(0xFF00FF00));
        
        // Teste fora dos limites
        assert!(!fb.set_pixel(10, 5, 0xFF0000FF));
        assert_eq!(fb.get_pixel(10, 5), None);
        
        // Teste clear
        fb.clear(0xFF123456);
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF123456));
        assert_eq!(fb.get_pixel(9, 9), Some(0xFF123456));
    }

    #[test]
    fn test_drawing_operations() {
        let mut fb = FrameBuffer::new(10, 10);
        fb.clear(0xFF000000);
        
        // Teste linha horizontal
        fb.draw_horizontal_line(2, 7, 3, 0xFFFF0000);
        for x in 2..=7 {
            assert_eq!(fb.get_pixel(x, 3), Some(0xFFFF0000));
        }
        
        // Teste linha vertical
        fb.draw_vertical_line(5, 1, 8, 0xFF00FF00);
        for y in 1..=8 {
            assert_eq!(fb.get_pixel(5, y), Some(0xFF00FF00));
        }
        
        // Teste retângulo
        fb.draw_rect(1, 1, 3, 3, 0xFF0000FF);
        assert_eq!(fb.get_pixel(1, 1), Some(0xFF0000FF));
        assert_eq!(fb.get_pixel(3, 1), Some(0xFF0000FF));
        assert_eq!(fb.get_pixel(1, 3), Some(0xFF0000FF));
        assert_eq!(fb.get_pixel(3, 3), Some(0xFF0000FF));
        assert_eq!(fb.get_pixel(2, 2), Some(0xFF000000)); // Centro não preenchido
    }

    #[test]
    fn test_blit_operations() {
        let mut src = FrameBuffer::new(5, 5);
        src.fill_rect(0, 0, 5, 5, 0xFFFF0000);
        src.set_pixel(2, 2, 0xFF00FF00);
        
        let mut dst = FrameBuffer::new(10, 10);
        dst.clear(0xFF000000);
        
        // Blit normal
        dst.blit(&src, 0, 0, 5, 5, 0, 0);
        assert_eq!(dst.get_pixel(0, 0), Some(0xFFFF0000));
        assert_eq!(dst.get_pixel(2, 2), Some(0xFF00FF00));
        
        // Blit parcial
        dst.clear(0xFF000000);
        dst.blit(&src, 1, 1, 3, 3, 5, 5);
        assert_eq!(dst.get_pixel(5, 5), Some(0xFFFF0000));
        assert_eq!(dst.get_pixel(6, 6), Some(0xFF00FF00));
    }

    #[test]
    fn test_color_operations() {
        let mut fb = FrameBuffer::new(3, 3);
        
        // Preencher com cores de teste
        for y in 0..3 {
            for x in 0..3 {
                let color = ((y * 3 + x) * 10) as u32;
                fb.set_pixel(x, y, 0xFF000000 | (color << 16) | (color << 8) | color);
            }
        }
        
        // Teste conversão para escala de cinza
        let mut gray_fb = fb.clone();
        gray_fb.convert_to_grayscale();
        
        // Verificar que R=G=B após conversão
        for y in 0..3 {
            for x in 0..3 {
                let pixel = gray_fb.get_pixel(x, y).unwrap();
                let r = (pixel >> 16) & 0xFF;
                let g = (pixel >> 8) & 0xFF;
                let b = pixel & 0xFF;
                assert_eq!(r, g);
                assert_eq!(g, b);
            }
        }
    }

    #[test]
    fn test_conversion_operations() {
        let fb = FrameBuffer::new(2, 2);
        let rgba = fb.to_rgba_bytes();
        
        assert_eq!(rgba.len(), 16); // 2*2*4 = 16 bytes
        
        // Teste conversão de volta
        let fb2 = FrameBuffer::from_rgba_bytes(2, 2, &rgba).unwrap();
        assert_eq!(fb.width, fb2.width);
        assert_eq!(fb.height, fb2.height);
        assert_eq!(fb.pixels.len(), fb2.pixels.len());
    }

    #[test]
    fn test_resize_and_transform() {
        let mut fb = FrameBuffer::new(4, 4);
        
        // Preencher com padrão
        for y in 0..4 {
            for x in 0..4 {
                let value = if (x + y) % 2 == 0 { 0xFFFFFFFF } else { 0xFF000000 };
                fb.set_pixel(x, y, value);
            }
        }
        
        // Teste redimensionamento
        fb.resize(8, 8);
        assert_eq!(fb.width, 8);
        assert_eq!(fb.height, 8);
        assert_eq!(fb.pixels.len(), 64);
        
        // Verificar que os primeiros 4x4 pixels foram preservados
        for y in 0..4 {
            for x in 0..4 {
                let expected = if (x + y) % 2 == 0 { 0xFFFFFFFF } else { 0xFF000000 };
                assert_eq!(fb.get_pixel(x, y), Some(expected));
            }
        }
    }

    #[test]
    fn test_thumbnail() {
        let fb = FrameBuffer::new(100, 100);
        let thumbnail = fb.create_thumbnail(10, 10);
        
        assert_eq!(thumbnail.width, 10);
        assert_eq!(thumbnail.height, 10);
        assert_eq!(thumbnail.pixels.len(), 100);
    }
}