//! Sprite Engine do VDP (Mega Drive / Sega Genesis)
//!
//! Implementa o sistema completo de sprites do VDP, incluindo:
//! - Sprite Attribute Table (SAT) na VRAM
//! - Até 80 sprites por quadro (mas apenas 20 por linha)
//! - Sprites de tamanhos variáveis (1x1 a 4x4 tiles)
//! - Prioridade, flip horizontal/vertical, seleção de paleta
//! - Link list para ordenação e limitação de sprites por linha
//! - Detecção de overflow e colisão de sprites

use crate::vdp::{
    cram::Cram,
    framebuffer::FrameBuffer,
    modes::VdpVideoMode,
    registers::VdpRegisters,
    vram::Vram,
};

// Constantes do sistema de sprites
pub const MAX_SPRITES_PER_FRAME: usize = 80;      // Máximo de sprites por quadro
pub const MAX_SPRITES_PER_LINE: usize = 20;       // Máximo de sprites por linha
pub const SPRITE_ENTRY_SIZE: usize = 8;           // Bytes por entrada de sprite
pub const TILE_SIZE: usize = 8;                   // Tamanho do tile em pixels

/// Tamanhos possíveis de sprites
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteSize {
    Size1x1,  // 8x8 pixels
    Size1x2,  // 8x16 pixels
    Size1x3,  // 8x24 pixels
    Size1x4,  // 8x32 pixels
    Size2x1,  // 16x8 pixels
    Size2x2,  // 16x16 pixels
    Size2x3,  // 16x24 pixels
    Size2x4,  // 16x32 pixels
    Size3x1,  // 24x8 pixels
    Size3x2,  // 24x16 pixels
    Size3x3,  // 24x24 pixels
    Size3x4,  // 24x32 pixels
    Size4x1,  // 32x8 pixels
    Size4x2,  // 32x16 pixels
    Size4x3,  // 32x24 pixels
    Size4x4,  // 32x32 pixels
}

impl SpriteSize {
    /// Retorna largura e altura em tiles
    pub fn dimensions(&self) -> (u8, u8) {
        match self {
            SpriteSize::Size1x1 => (1, 1),
            SpriteSize::Size1x2 => (1, 2),
            SpriteSize::Size1x3 => (1, 3),
            SpriteSize::Size1x4 => (1, 4),
            SpriteSize::Size2x1 => (2, 1),
            SpriteSize::Size2x2 => (2, 2),
            SpriteSize::Size2x3 => (2, 3),
            SpriteSize::Size2x4 => (2, 4),
            SpriteSize::Size3x1 => (3, 1),
            SpriteSize::Size3x2 => (3, 2),
            SpriteSize::Size3x3 => (3, 3),
            SpriteSize::Size3x4 => (3, 4),
            SpriteSize::Size4x1 => (4, 1),
            SpriteSize::Size4x2 => (4, 2),
            SpriteSize::Size4x3 => (4, 3),
            SpriteSize::Size4x4 => (4, 4),
        }
    }
    
    /// Retorna largura e altura em pixels
    pub fn pixel_dimensions(&self) -> (usize, usize) {
        let (w, h) = self.dimensions();
        (w as usize * TILE_SIZE, h as usize * TILE_SIZE)
    }
    
    /// Converte de código do hardware (2 bits para largura, 2 bits para altura)
    pub fn from_hardware_code(width_code: u8, height_code: u8) -> Option<Self> {
        match (width_code, height_code) {
            (0, 0) => Some(SpriteSize::Size1x1),
            (0, 1) => Some(SpriteSize::Size1x2),
            (0, 2) => Some(SpriteSize::Size1x3),
            (0, 3) => Some(SpriteSize::Size1x4),
            (1, 0) => Some(SpriteSize::Size2x1),
            (1, 1) => Some(SpriteSize::Size2x2),
            (1, 2) => Some(SpriteSize::Size2x3),
            (1, 3) => Some(SpriteSize::Size2x4),
            (2, 0) => Some(SpriteSize::Size3x1),
            (2, 1) => Some(SpriteSize::Size3x2),
            (2, 2) => Some(SpriteSize::Size3x3),
            (2, 3) => Some(SpriteSize::Size3x4),
            (3, 0) => Some(SpriteSize::Size4x1),
            (3, 1) => Some(SpriteSize::Size4x2),
            (3, 2) => Some(SpriteSize::Size4x3),
            (3, 3) => Some(SpriteSize::Size4x4),
            _ => None,
        }
    }
    
    /// Converte para código do hardware
    pub fn to_hardware_code(&self) -> (u8, u8) {
        match self {
            SpriteSize::Size1x1 => (0, 0),
            SpriteSize::Size1x2 => (0, 1),
            SpriteSize::Size1x3 => (0, 2),
            SpriteSize::Size1x4 => (0, 3),
            SpriteSize::Size2x1 => (1, 0),
            SpriteSize::Size2x2 => (1, 1),
            SpriteSize::Size2x3 => (1, 2),
            SpriteSize::Size2x4 => (1, 3),
            SpriteSize::Size3x1 => (2, 0),
            SpriteSize::Size3x2 => (2, 1),
            SpriteSize::Size3x3 => (2, 2),
            SpriteSize::Size3x4 => (2, 3),
            SpriteSize::Size4x1 => (3, 0),
            SpriteSize::Size4x2 => (3, 1),
            SpriteSize::Size4x3 => (3, 2),
            SpriteSize::Size4x4 => (3, 3),
        }
    }
}

/// Estrutura que representa um sprite individual
#[derive(Debug, Clone)]
pub struct Sprite {
    pub y: i16,                     // Posição Y (com offset -128)
    pub x: i16,                     // Posição X (com offset -128)
    pub size: SpriteSize,           // Tamanho do sprite
    pub link: u8,                   // Link para próximo sprite (0-79)
    pub tile_index: u16,            // Índice do tile na VRAM (0-2047)
    pub palette: u8,                // Paleta (0-3)
    pub priority: bool,             // Prioridade (0=baixa, 1=alta)
    pub flip_horizontal: bool,      // Flip horizontal
    pub flip_vertical: bool,        // Flip vertical
    pub valid: bool,                // Sprite válido (não vazio)
    pub visible: bool,              // Sprite atualmente visível
    pub high_color_mode: bool,      // Usa modo de alta cor (8bpp)?
}

impl Sprite {
    /// Cria um novo sprite vazio/inválido
    pub fn new() -> Self {
        Self {
            y: 0,
            x: 0,
            size: SpriteSize::Size1x1,
            link: 0,
            tile_index: 0,
            palette: 0,
            priority: false,
            flip_horizontal: false,
            flip_vertical: false,
            valid: false,
            visible: false,
            high_color_mode: false,
        }
    }
    
    /// Cria um sprite com parâmetros específicos
    pub fn with_params(
        x: i16,
        y: i16,
        size: SpriteSize,
        tile_index: u16,
        palette: u8,
        priority: bool,
        flip_h: bool,
        flip_v: bool,
        high_color_mode: bool,
    ) -> Self {
        Self {
            y,
            x,
            size,
            link: 0,
            tile_index,
            palette,
            priority,
            flip_horizontal: flip_h,
            flip_vertical: flip_v,
            valid: true,
            visible: true,
            high_color_mode,
        }
    }
    
    /// Decodifica um sprite a partir de 8 bytes da VRAM (formato SAT)
    pub fn from_bytes(data: &[u8; 8], high_color_mode: bool) -> Self {
        // Formato SAT (Sprite Attribute Table):
        // Bytes 0-1: Posição Y (9 bits, complemento de 2 com offset -128)
        // Bytes 2-3: Size/Link (bits 0-1: altura, bits 2-3: largura, bits 8-14: link)
        // Bytes 4-5: Posição X (9 bits, complemento de 2 com offset -128)
        // Bytes 6-7: Atributos do tile
        
        let y_raw = ((data[0] as u16) << 8) | (data[1] as u16);
        let size_link = ((data[2] as u16) << 8) | (data[3] as u16);
        let x_raw = ((data[4] as u16) << 8) | (data[5] as u16);
        let attributes = ((data[6] as u16) << 8) | (data[7] as u16);
        
        // Decodificar posição Y (9 bits, complemento de 2)
        let y = if y_raw & 0x0100 != 0 {
            // Valor negativo (usando complemento de 2 para 9 bits)
            (y_raw | 0xFE00) as i16
        } else {
            y_raw as i16
        };
        
        // Ajustar offset -128
        let y = y.wrapping_sub(128);
        
        // Decodificar tamanho
        let width_code = ((size_link >> 2) & 0x03) as u8;
        let height_code = (size_link & 0x03) as u8;
        let size = SpriteSize::from_hardware_code(width_code, height_code)
            .unwrap_or(SpriteSize::Size1x1);
        
        // Decodificar link
        let link = ((size_link >> 8) & 0x7F) as u8;
        
        // Decodificar posição X (9 bits, complemento de 2)
        let x = if x_raw & 0x0100 != 0 {
            (x_raw | 0xFE00) as i16
        } else {
            x_raw as i16
        };
        
        // Ajustar offset -128
        let x = x.wrapping_sub(128);
        
        // Decodificar atributos
        let tile_index = attributes & 0x07FF;           // Bits 0-10
        let palette = ((attributes >> 13) & 0x03) as u8; // Bits 13-14
        let priority = (attributes & 0x8000) != 0;      // Bit 15
        let flip_horizontal = (attributes & 0x0800) != 0; // Bit 11
        let flip_vertical = (attributes & 0x1000) != 0;   // Bit 12
        
        // Determinar se o sprite é válido
        // Um sprite é considerado inválido se Y estiver fora da faixa visível e link = 0
        let valid = !(y < -128 || y >= 224 + 128) || link != 0;
        
        Self {
            y,
            x,
            size,
            link,
            tile_index,
            palette,
            priority,
            flip_horizontal,
            flip_vertical,
            valid,
            visible: true,
            high_color_mode,
        }
    }
    
    /// Codifica o sprite de volta para 8 bytes (formato SAT)
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        
        // Codificar posição Y (com offset +128)
        let y_adj = self.y.wrapping_add(128);
        let y_raw = if y_adj < 0 {
            (y_adj & 0x01FF) as u16 | 0xFE00
        } else {
            y_adj as u16 & 0x01FF
        };
        
        bytes[0] = (y_raw >> 8) as u8;
        bytes[1] = y_raw as u8;
        
        // Codificar tamanho e link
        let (width_code, height_code) = self.size.to_hardware_code();
        let size_link = ((self.link as u16 & 0x7F) << 8) |
                       ((width_code as u16 & 0x03) << 2) |
                       (height_code as u16 & 0x03);
        
        bytes[2] = (size_link >> 8) as u8;
        bytes[3] = size_link as u8;
        
        // Codificar posição X (com offset +128)
        let x_adj = self.x.wrapping_add(128);
        let x_raw = if x_adj < 0 {
            (x_adj & 0x01FF) as u16 | 0xFE00
        } else {
            x_adj as u16 & 0x01FF
        };
        
        bytes[4] = (x_raw >> 8) as u8;
        bytes[5] = x_raw as u8;
        
        // Codificar atributos
        let mut attributes = self.tile_index & 0x07FF;
        attributes |= (self.palette as u16 & 0x03) << 13;
        if self.priority { attributes |= 0x8000; }
        if self.flip_horizontal { attributes |= 0x0800; }
        if self.flip_vertical { attributes |= 0x1000; }
        
        bytes[6] = (attributes >> 8) as u8;
        bytes[7] = attributes as u8;
        
        bytes
    }
    
    /// Retorna largura do sprite em pixels
    pub fn width_pixels(&self) -> usize {
        let (w, _) = self.size.pixel_dimensions();
        w
    }
    
    /// Retorna altura do sprite em pixels
    pub fn height_pixels(&self) -> usize {
        let (_, h) = self.size.pixel_dimensions();
        h
    }
    
    /// Verifica se o sprite está visível na linha Y especificada
    pub fn is_on_line(&self, line: i16) -> bool {
        if !self.valid || !self.visible {
            return false;
        }
        
        let sprite_top = self.y;
        let sprite_bottom = self.y + self.height_pixels() as i16;
        
        line >= sprite_top && line < sprite_bottom
    }
    
    /// Verifica se o sprite está completamente fora da tela
    pub fn is_offscreen(&self, screen_width: usize, screen_height: usize) -> bool {
        let right = self.x + self.width_pixels() as i16;
        let bottom = self.y + self.height_pixels() as i16;
        
        right <= 0 || self.x >= screen_width as i16 ||
        bottom <= 0 || self.y >= screen_height as i16
    }
    
    /// Retorna o índice de cor de um pixel específico do sprite
    pub fn get_pixel_color(
        &self,
        vram: &Vram,
        sprite_x: usize,
        sprite_y: usize,
    ) -> Option<(u8, u8)> { // Retorna (índice de cor, paleta)
        if sprite_x >= self.width_pixels() || sprite_y >= self.height_pixels() {
            return None;
        }
        
        // Calcular tile local dentro do sprite
        let tiles_wide = self.size.dimensions().0 as usize;
        let tile_x = sprite_x / TILE_SIZE;
        let tile_y = sprite_y / TILE_SIZE;
        
        // Coordenadas dentro do tile
        let mut pixel_x = sprite_x % TILE_SIZE;
        let mut pixel_y = sprite_y % TILE_SIZE;
        
        // Aplicar flip
        if self.flip_horizontal {
            pixel_x = TILE_SIZE - 1 - pixel_x;
        }
        if self.flip_vertical {
            pixel_y = TILE_SIZE - 1 - pixel_y;
        }
        
        // Calcular índice do tile na VRAM
        // Os tiles são organizados em linhas no sprite
        let tile_offset = tile_y * tiles_wide + tile_x;
        let absolute_tile_index = self.tile_index as usize + tile_offset;
        
        // Obter cor do pixel
        let color_index = if self.high_color_mode {
            // Modo 8bpp (256 cores)
            vram.read_tile_pixel_8bpp(absolute_tile_index, pixel_x, pixel_y)
        } else {
            // Modo 4bpp (16 cores por paleta)
            vram.read_tile_pixel_4bpp(absolute_tile_index, pixel_x, pixel_y)
        };
        
        Some((color_index, self.palette))
    }
    
    /// Renderiza o sprite em um framebuffer
    pub fn render(
        &self,
        framebuffer: &mut FrameBuffer,
        vram: &Vram,
        cram: &Cram,
        line: Option<u16>, // Se especificado, renderiza apenas esta linha
    ) {
        if !self.valid || !self.visible || self.is_offscreen(framebuffer.width, framebuffer.height) {
            return;
        }
        
        let screen_width = framebuffer.width as i16;
        let screen_height = framebuffer.height as i16;
        
        // Determinar intervalo de linhas a renderizar
        let start_line = if let Some(l) = line {
            l as i16
        } else {
            self.y.max(0)
        };
        
        let end_line = if let Some(l) = line {
            (l + 1) as i16
        } else {
            (self.y + self.height_pixels() as i16).min(screen_height)
        };
        
        // Para cada linha do sprite...
        for screen_y in start_line..end_line {
            // Verificar se esta linha está dentro do sprite
            if !self.is_on_line(screen_y) {
                continue;
            }
            
            // Calcular linha local dentro do sprite
            let sprite_y = (screen_y - self.y) as usize;
            
            // Para cada coluna do sprite...
            for sprite_x in 0..self.width_pixels() {
                let screen_x = self.x + sprite_x as i16;
                
                // Verificar se está dentro da tela
                if screen_x < 0 || screen_x >= screen_width {
                    continue;
                }
                
                // Obter cor do pixel
                if let Some((color_index, palette)) = self.get_pixel_color(vram, sprite_x, sprite_y) {
                    // Ignorar pixel transparente (índice 0)
                    if color_index == 0 {
                        continue;
                    }
                    
                    // Calcular índice na CRAM
                    let cram_index = if self.high_color_mode {
                        color_index as usize
                    } else {
                        (palette as usize * 16) + color_index as usize
                    };
                    
                    // Obter cor da CRAM
                    let color_9bit = cram.read(cram_index % 64);
                    
                    // Converter para RGB888
                    let r = ((color_9bit >> 0) & 0x07) as u8 * 36;  // 36 ≈ 255/7
                    let g = ((color_9bit >> 4) & 0x07) as u8 * 36;
                    let b = ((color_9bit >> 8) & 0x07) as u8 * 36;
                    let color = (0xFF << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                    
                    // Desenhar pixel
                    framebuffer.set_pixel(screen_x as usize, screen_y as usize, color);
                }
            }
        }
    }
}

/// Tabela de Sprites (SAT - Sprite Attribute Table)
#[derive(Clone)]
pub struct SpriteTable {
    pub sprites: Vec<Sprite>,                   // Todos os sprites
    pub base_address: u32,                      // Endereço base da SAT na VRAM
    pub sprite_count: usize,                    // Número de sprites ativos
    pub active_sprites_per_line: Vec<usize>,    // Sprites ativos por linha
    pub overflow_line: Option<u16>,             // Linha onde ocorreu overflow
    pub collision_detected: bool,               // Colisão detectada
    pub high_color_mode: bool,                  // Modo de alta cor ativo
}

impl SpriteTable {
    /// Cria uma nova tabela de sprites
    pub fn new(base_address: u32, high_color_mode: bool) -> Self {
        Self {
            sprites: Vec::with_capacity(MAX_SPRITES_PER_FRAME),
            base_address,
            sprite_count: 0,
            active_sprites_per_line: vec![0; 512], // Suficiente para PAL/NTSC
            overflow_line: None,
            collision_detected: false,
            high_color_mode,
        }
    }
    
    /// Limpa a tabela de sprites
    pub fn clear(&mut self) {
        self.sprites.clear();
        self.sprite_count = 0;
        self.active_sprites_per_line.fill(0);
        self.overflow_line = None;
        self.collision_detected = false;
    }
    
    /// Carrega a tabela de sprites da VRAM
    pub fn load_from_vram(&mut self, vram: &Vram) {
        self.clear();
        
        let mut link_index = 0;
        let mut processed_sprites = 0;
        
        // Seguir a chain de sprites até encontrar link = 0 ou atingir limite
        while link_index < MAX_SPRITES_PER_FRAME && processed_sprites < MAX_SPRITES_PER_FRAME {
            let addr = self.base_address + (link_index as u32 * SPRITE_ENTRY_SIZE as u32);
            
            // Ler 8 bytes da VRAM
            let mut bytes = [0u8; 8];
            for i in 0..8 {
                bytes[i] = vram.read8(addr + i as u32);
            }
            
            // Decodificar sprite
            let sprite = Sprite::from_bytes(&bytes, self.high_color_mode);
            
            // Adicionar à lista
            self.sprites.push(sprite);
            self.sprite_count += 1;
            processed_sprites += 1;
            
            // Atualizar link para próximo sprite
            link_index = sprite.link as usize;
            
            // Se link = 0, terminamos a chain
            if sprite.link == 0 {
                break;
            }
        }
    }
    
    /// Atualiza a VRAM com a tabela de sprites atual
    pub fn save_to_vram(&self, vram: &mut Vram) {
        for (i, sprite) in self.sprites.iter().enumerate() {
            let addr = self.base_address + (i as u32 * SPRITE_ENTRY_SIZE as u32);
            let bytes = sprite.to_bytes();
            
            for (j, &byte) in bytes.iter().enumerate() {
                vram.write8(addr + j as u32, byte);
            }
        }
    }
    
    /// Calcula quais sprites estão ativos em cada linha
    pub fn calculate_active_sprites(&mut self, screen_height: u16) {
        self.active_sprites_per_line.fill(0);
        self.overflow_line = None;
        
        // Para cada linha da tela...
        for line in 0..screen_height as usize {
            let mut count = 0;
            
            // Contar sprites nesta linha
            for sprite in &self.sprites {
                if sprite.is_on_line(line as i16) {
                    count += 1;
                    
                    // Verificar overflow (mais de 20 sprites)
                    if count > MAX_SPRITES_PER_LINE && self.overflow_line.is_none() {
                        self.overflow_line = Some(line as u16);
                    }
                }
            }
            
            // Armazenar contagem (limitada a MAX_SPRITES_PER_LINE)
            self.active_sprites_per_line[line] = count.min(MAX_SPRITES_PER_LINE);
        }
    }
    
    /// Renderiza uma linha específica do framebuffer com sprites
    pub fn render_line(
        &self,
        line: u16,
        framebuffer: &mut FrameBuffer,
        vram: &Vram,
        cram: &Cram,
        high_priority_only: bool,
    ) {
        let mut sprites_rendered = 0;
        
        // Renderizar sprites nesta linha (até o limite)
        for sprite in &self.sprites {
            // Verificar se atingimos o limite por linha
            if sprites_rendered >= MAX_SPRITES_PER_LINE {
                break;
            }
            
            // Verificar se sprite está nesta linha e tem a prioridade correta
            if sprite.is_on_line(line as i16) &&
               (!high_priority_only || sprite.priority) {
                
                sprite.render(framebuffer, vram, cram, Some(line));
                sprites_rendered += 1;
            }
        }
    }
    
    /// Renderiza todos os sprites no framebuffer
    pub fn render_all(
        &self,
        framebuffer: &mut FrameBuffer,
        vram: &Vram,
        cram: &Cram,
    ) {
        // Primeiro renderizar sprites sem prioridade
        for sprite in &self.sprites {
            if sprite.valid && sprite.visible && !sprite.priority {
                sprite.render(framebuffer, vram, cram, None);
            }
        }
        
        // Depois renderizar sprites com prioridade
        for sprite in &self.sprites {
            if sprite.valid && sprite.visible && sprite.priority {
                sprite.render(framebuffer, vram, cram, None);
            }
        }
    }
    
    /// Detecta colisões entre sprites
    pub fn detect_collisions(&mut self, screen_width: usize, screen_height: usize) -> bool {
        self.collision_detected = false;
        
        // Para cada par de sprites...
        for i in 0..self.sprites.len() {
            let sprite_a = &self.sprites[i];
            
            if !sprite_a.valid || !sprite_a.visible ||
               sprite_a.is_offscreen(screen_width, screen_height) {
                continue;
            }
            
            for j in (i + 1)..self.sprites.len() {
                let sprite_b = &self.sprites[j];
                
                if !sprite_b.valid || !sprite_b.visible ||
                   sprite_b.is_offscreen(screen_width, screen_height) {
                    continue;
                }
                
                // Verificar sobreposição de retângulos
                let a_left = sprite_a.x;
                let a_right = sprite_a.x + sprite_a.width_pixels() as i16;
                let a_top = sprite_a.y;
                let a_bottom = sprite_a.y + sprite_a.height_pixels() as i16;
                
                let b_left = sprite_b.x;
                let b_right = sprite_b.x + sprite_b.width_pixels() as i16;
                let b_top = sprite_b.y;
                let b_bottom = sprite_b.y + sprite_b.height_pixels() as i16;
                
                if a_left < b_right && a_right > b_left &&
                   a_top < b_bottom && a_bottom > b_top {
                    self.collision_detected = true;
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Adiciona um sprite à tabela
    pub fn add_sprite(&mut self, sprite: Sprite) -> bool {
        if self.sprite_count >= MAX_SPRITES_PER_FRAME {
            return false;
        }
        
        self.sprites.push(sprite);
        self.sprite_count += 1;
        true
    }
    
    /// Remove um sprite da tabela
    pub fn remove_sprite(&mut self, index: usize) -> bool {
        if index >= self.sprites.len() {
            return false;
        }
        
        self.sprites.remove(index);
        self.sprite_count -= 1;
        true
    }
    
    /// Atualiza um sprite existente
    pub fn update_sprite(&mut self, index: usize, sprite: Sprite) -> bool {
        if index >= self.sprites.len() {
            return false;
        }
        
        self.sprites[index] = sprite;
        true
    }
    
    /// Retorna informações de debug sobre a tabela de sprites
    pub fn debug_info(&self) -> String {
        let mut info = String::new();
        
        info.push_str(&format!("Sprite Table ({} sprites):\n", self.sprite_count));
        info.push_str(&format!("  Base Address: 0x{:04X}\n", self.base_address));
        info.push_str(&format!("  High Color Mode: {}\n", self.high_color_mode));
        
        if let Some(overflow_line) = self.overflow_line {
            info.push_str(&format!("  Overflow at line: {}\n", overflow_line));
        }
        
        if self.collision_detected {
            info.push_str("  Collision detected\n");
        }
        
        // Informações dos primeiros sprites
        for (i, sprite) in self.sprites.iter().take(5).enumerate() {
            let (w, h) = sprite.size.pixel_dimensions();
            info.push_str(&format!(
                "  Sprite {}: ({}, {}) {}x{}, tile: {}, link: {}\n",
                i, sprite.x, sprite.y, w, h, sprite.tile_index, sprite.link
            ));
        }
        
        if self.sprite_count > 5 {
            info.push_str(&format!("  ... and {} more sprites\n", self.sprite_count - 5));
        }
        
        info
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdp::framebuffer::FrameBuffer;

    fn create_test_vram() -> Vram {
        let mut vram = Vram::new();
        
        // Criar um tile simples (padrão xadrez)
        let mut tile = [0u8; 32];
        for i in 0..32 {
            tile[i] = if i % 2 == 0 { 0xF0 } else { 0x0F };
        }
        vram.write_tile_4bpp(0, &tile);
        
        vram
    }
    
    #[test]
    fn test_sprite_size() {
        let size = SpriteSize::Size2x2;
        assert_eq!(size.dimensions(), (2, 2));
        assert_eq!(size.pixel_dimensions(), (16, 16));
        
        let (w_code, h_code) = size.to_hardware_code();
        assert_eq!(w_code, 1);
        assert_eq!(h_code, 1);
        
        let decoded = SpriteSize::from_hardware_code(1, 1).unwrap();
        assert_eq!(decoded, SpriteSize::Size2x2);
    }
    
    #[test]
    fn test_sprite_creation() {
        let sprite = Sprite::with_params(
            64, 32,
            SpriteSize::Size2x2,
            100,
            1,
            true,
            false,
            true,
            false,
        );
        
        assert_eq!(sprite.x, 64);
        assert_eq!(sprite.y, 32);
        assert_eq!(sprite.tile_index, 100);
        assert_eq!(sprite.palette, 1);
        assert_eq!(sprite.priority, true);
        assert_eq!(sprite.flip_horizontal, false);
        assert_eq!(sprite.flip_vertical, true);
        assert_eq!(sprite.valid, true);
        assert_eq!(sprite.width_pixels(), 16);
        assert_eq!(sprite.height_pixels(), 16);
    }
    
    #[test]
    fn test_sprite_encoding_decoding() {
        let original = Sprite::with_params(
            64, 32,
            SpriteSize::Size2x2,
            100,
            1,
            true,
            false,
            true,
            false,
        );
        
        let bytes = original.to_bytes();
        let decoded = Sprite::from_bytes(&bytes, false);
        
        // Verificar propriedades importantes
        assert_eq!(decoded.x, original.x);
        assert_eq!(decoded.y, original.y);
        assert_eq!(decoded.tile_index, original.tile_index);
        assert_eq!(decoded.palette, original.palette);
        assert_eq!(decoded.priority, original.priority);
        assert_eq!(decoded.flip_horizontal, original.flip_horizontal);
        assert_eq!(decoded.flip_vertical, original.flip_vertical);
        assert_eq!(decoded.size, original.size);
    }
    
    #[test]
    fn test_sprite_visibility() {
        let sprite = Sprite::with_params(
            10, 20,
            SpriteSize::Size1x1,
            0,
            0,
            false,
            false,
            false,
            false,
        );
        
        // Sprite ocupa linhas 20-27
        assert!(sprite.is_on_line(20));
        assert!(sprite.is_on_line(25));
        assert!(!sprite.is_on_line(15));
        assert!(!sprite.is_on_line(30));
        
        // Teste offscreen
        let offscreen_sprite = Sprite::with_params(
            -100, -100,
            SpriteSize::Size1x1,
            0,
            0,
            false,
            false,
            false,
            false,
        );
        
        assert!(offscreen_sprite.is_offscreen(320, 224));
    }
    
    #[test]
    fn test_sprite_table() {
        let mut sprite_table = SpriteTable::new(0xF800, false);
        
        // Adicionar alguns sprites
        let sprite1 = Sprite::with_params(
            10, 20,
            SpriteSize::Size1x1,
            0,
            0,
            false,
            false,
            false,
            false,
        );
        
        let sprite2 = Sprite::with_params(
            50, 30,
            SpriteSize::Size2x2,
            10,
            1,
            true,
            false,
            false,
            false,
        );
        
        assert!(sprite_table.add_sprite(sprite1));
        assert!(sprite_table.add_sprite(sprite2));
        assert_eq!(sprite_table.sprite_count, 2);
        
        // Teste remoção
        assert!(sprite_table.remove_sprite(0));
        assert_eq!(sprite_table.sprite_count, 1);
        
        // Teste atualização
        let sprite3 = Sprite::with_params(
            100, 100,
            SpriteSize::Size1x2,
            20,
            2,
            false,
            true,
            true,
            false,
        );
        
        assert!(sprite_table.update_sprite(0, sprite3));
    }
    
    #[test]
    fn test_sprite_table_calculation() {
        let mut sprite_table = SpriteTable::new(0xF800, false);
        
        // Adicionar sprites em diferentes linhas
        for i in 0..10 {
            let sprite = Sprite::with_params(
                i as i16 * 20,
                i as i16 * 10,
                SpriteSize::Size1x1,
                i as u16,
                0,
                false,
                false,
                false,
                false,
            );
            sprite_table.add_sprite(sprite);
        }
        
        // Calcular sprites ativos
        sprite_table.calculate_active_sprites(224);
        
        // Verificar algumas linhas
        assert!(sprite_table.active_sprites_per_line[0] > 0); // Linha 0 tem sprite
        assert!(sprite_table.active_sprites_per_line[100] > 0); // Linha 100 tem sprite
    }
    
    #[test]
    fn test_sprite_overflow() {
        let mut sprite_table = SpriteTable::new(0xF800, false);
        
        // Adicionar mais de 20 sprites na mesma linha
        for _ in 0..25 {
            let sprite = Sprite::with_params(
                0, 50, // Todos na linha 50
                SpriteSize::Size1x1,
                0,
                0,
                false,
                false,
                false,
                false,
            );
            sprite_table.add_sprite(sprite);
        }
        
        sprite_table.calculate_active_sprites(224);
        
        // Deve detectar overflow na linha 50
        assert_eq!(sprite_table.overflow_line, Some(50));
        
        // Deve limitar a 20 sprites por linha
        assert_eq!(sprite_table.active_sprites_per_line[50], 20);
    }
    
    #[test]
    fn test_sprite_collision() {
        let mut sprite_table = SpriteTable::new(0xF800, false);
        
        // Dois sprites sobrepostos
        let sprite1 = Sprite::with_params(
            10, 10,
            SpriteSize::Size2x2,
            0,
            0,
            false,
            false,
            false,
            false,
        );
        
        let sprite2 = Sprite::with_params(
            15, 15,
            SpriteSize::Size2x2,
            0,
            0,
            false,
            false,
            false,
            false,
        );
        
        sprite_table.add_sprite(sprite1);
        sprite_table.add_sprite(sprite2);
        
        // Deve detectar colisão
        let collision = sprite_table.detect_collisions(320, 224);
        assert!(collision);
        assert!(sprite_table.collision_detected);
        
        // Sprites não sobrepostos
        let mut sprite_table2 = SpriteTable::new(0xF800, false);
        
        let sprite3 = Sprite::with_params(
            10, 10,
            SpriteSize::Size1x1,
            0,
            0,
            false,
            false,
            false,
            false,
        );
        
        let sprite4 = Sprite::with_params(
            100, 100,
            SpriteSize::Size1x1,
            0,
            0,
            false,
            false,
            false,
            false,
        );
        
        sprite_table2.add_sprite(sprite3);
        sprite_table2.add_sprite(sprite4);
        
        // Não deve detectar colisão
        let collision = sprite_table2.detect_collisions(320, 224);
        assert!(!collision);
        assert!(!sprite_table2.collision_detected);
    }
    
    #[test]
    fn test_sprite_rendering() {
        let vram = create_test_vram();
        let cram = Cram::new();
        
        let mut sprite_table = SpriteTable::new(0xF800, false);
        
        // Adicionar um sprite
        let sprite = Sprite::with_params(
            10, 10,
            SpriteSize::Size1x1,
            0,
            0,
            false,
            false,
            false,
            false,
        );
        
        sprite_table.add_sprite(sprite);
        
        // Criar framebuffer
        let mut framebuffer = FrameBuffer::new(64, 64);
        
        // Renderizar sprite
        sprite_table.render_all(&mut framebuffer, &vram, &cram);
        
        // Verificar que alguns pixels foram desenhados
        let has_pixels = framebuffer.pixels().iter().any(|&c| c != 0);
        assert!(has_pixels);
        
        // Verificar pixel específico (10,10) - deve ser desenhado
        let pixel = framebuffer.get_pixel(10, 10);
        assert!(pixel.is_some() && pixel.unwrap() != 0);
    }
}