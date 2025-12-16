//! Renderização dos planos A, B e Window do VDP (Tilemaps).
//!
//! O VDP possui três planos de tilemap:
//! - Plano A: Tilemap principal (base address configurável)
//! - Plano B: Tilemap secundário (base address configurável)  
//! - Window: Tilemap de janela (sobreposição fixa/scroll)
//!
//! Cada entrada do tilemap é de 2 bytes com formato:
//! Bit 15:    Prioridade (0 = abaixo de sprites, 1 = acima de sprites)
//! Bits 14-13: Paleta (0-3)
//! Bit 12:    Flip vertical
//! Bit 11:    Flip horizontal
//! Bits 10-0: Índice do tile (0-2047)

use crate::vdp::{
    cram::Cram,
    framebuffer::FrameBuffer,
    video_modes::{VdpVideoMode, VdpRenderMode},
    registers::VdpRegisters,
    vram::Vram,
    vsram::Vsram,
};

/// Tipos de plano do VDP
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaneType {
    A,      // Plano A (normalmente fundo principal)
    B,      // Plano B (normalmente fundo secundário)
    Window, // Plano de janela (sobreposição)
}

/// Estrutura que representa um plano completo
#[derive(Clone, Debug)]
pub struct Plane {
    pub plane_type: PlaneType,
    pub name_table_addr: u32,    // Endereço base da tabela de nomes na VRAM
    pub width_tiles: usize,      // Largura em tiles (32, 64, 128)
    pub height_tiles: usize,     // Altura em tiles (32, 64, 128)
    pub width_pixels: usize,     // Largura em pixels
    pub height_pixels: usize,    // Altura em pixels
    pub enabled: bool,           // Plano habilitado?
    pub priority: u8,            // Prioridade do plano (0-3)
    pub scroll_x: i32,           // Scroll horizontal
    pub scroll_y: i32,           // Scroll vertical
    pub window_x: i32,           // Posição X da janela (se for window plane)
    pub window_y: i32,           // Posição Y da janela (se for window plane)
    pub window_hsplit: bool,     // Janela dividida horizontalmente?
    pub window_vsplit: bool,     // Janela dividida verticalmente?
}

/// Entrada de tile no tilemap (2 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileEntry {
    pub tile_index: u16,     // Índice do tile (0-2047)
    pub palette: u8,         // Número da paleta (0-3)
    pub priority: bool,      // Prioridade do tile
    pub flip_horizontal: bool, // Flip horizontal
    pub flip_vertical: bool, // Flip vertical
    pub hscroll_priority: bool, // Prioridade para hscroll (modo row scroll)
    pub vscroll_priority: bool, // Prioridade para vscroll (modo column scroll)
}

impl TileEntry {
    /// Decodifica uma entrada de tile de 2 bytes
    pub fn from_word(word: u16) -> Self {
        Self {
            tile_index: word & 0x07FF,
            palette: ((word >> 13) & 0x03) as u8,
            priority: (word & 0x8000) != 0,
            flip_horizontal: (word & 0x0800) != 0,
            flip_vertical: (word & 0x1000) != 0,
            hscroll_priority: (word & 0x0800) != 0, // Reusa bit de flip?
            vscroll_priority: (word & 0x1000) != 0, // Reusa bit de flip?
        }
    }

    /// Codifica a entrada de volta para 16 bits
    pub fn to_word(&self) -> u16 {
        let mut word = self.tile_index & 0x07FF;
        word |= (self.palette as u16 & 0x03) << 13;
        if self.priority { word |= 0x8000; }
        if self.flip_horizontal { word |= 0x0800; }
        if self.flip_vertical { word |= 0x1000; }
        word
    }

    /// Retorna o índice de cor de um pixel dentro do tile (4bpp)
    pub fn get_pixel_color_4bpp(&self, vram: &Vram, x: u8, y: u8) -> u8 {
        // Ajusta coordenadas baseado no flip
        let px = if self.flip_horizontal { 7 - x } else { x };
        let py = if self.flip_vertical { 7 - y } else { y };
        
        // Cada tile 4bpp tem 32 bytes (8x8 pixels, 4 bits por pixel)
        let tile_base = (self.tile_index as usize) * 32;
        
        // Cada linha de tile tem 4 bytes (8 pixels, 4 bits cada)
        let row_offset = (py as usize) * 4;
        let byte_index = tile_base + row_offset + (px as usize / 2);
        
        if byte_index < vram.data.len() {
            let byte = vram.data[byte_index];
            if px % 2 == 0 {
                // Pixel baixo: bits 4-7
                (byte >> 4) & 0x0F
            } else {
                // Pixel alto: bits 0-3
                byte & 0x0F
            }
        } else {
            0
        }
    }

    /// Retorna o índice de cor de um pixel dentro do tile (8bpp)
    pub fn get_pixel_color_8bpp(&self, vram: &Vram, x: u8, y: u8) -> u8 {
        let px = if self.flip_horizontal { 7 - x } else { x };
        let py = if self.flip_vertical { 7 - y } else { y };
        
        // Cada tile 8bpp tem 64 bytes (8x8 pixels, 8 bits por pixel)
        let tile_base = (self.tile_index as usize) * 64;
        let pixel_index = tile_base + (py as usize) * 8 + (px as usize);
        
        if pixel_index < vram.data.len() {
            vram.data[pixel_index]
        } else {
            0
        }
    }
}

impl Plane {
    /// Cria um novo plano com base nos registradores do VDP
    pub fn new(plane_type: PlaneType, regs: &VdpRegisters, mode: &VdpVideoMode) -> Self {
        // Obter endereço base da tabela de nomes
        let name_table_addr = match plane_type {
            PlaneType::A => regs.get_plane_a_address() as u32,
            PlaneType::B => regs.get_plane_b_address() as u32,
            PlaneType::Window => regs.get_window_address() as u32,
        };

        // Obter tamanho do plano (em tiles)
        let (width_tiles, height_tiles) = regs.get_plane_size(plane_type);
        
        // Calcular tamanho em pixels
        let width_pixels = width_tiles * 8;
        let height_pixels = height_tiles * 8;

        // Configurações específicas da janela
        let (window_x, window_y, window_hsplit, window_vsplit) = match plane_type {
            PlaneType::Window => {
                let window_pos = regs.get_window_position();
                (window_pos.0 as i32, window_pos.1 as i32, false, false)
            }
            _ => (0, 0, false, false),
        };

        // Determinar se o plano está habilitado
        let enabled = match plane_type {
            PlaneType::Window => name_table_addr != 0, // Janela habilitada se endereço != 0
            _ => true, // Planos A e B sempre habilitados se display estiver ativo
        };

        Self {
            plane_type,
            name_table_addr,
            width_tiles,
            height_tiles,
            width_pixels,
            height_pixels,
            enabled,
            priority: match plane_type {
                PlaneType::A => 0,
                PlaneType::B => 1,
                PlaneType::Window => 2,
            },
            scroll_x: 0,
            scroll_y: 0,
            window_x,
            window_y,
            window_hsplit,
            window_vsplit,
        }
    }

    /// Atualiza scroll do plano
    pub fn update_scroll(&mut self, regs: &VdpRegisters) {
        match self.plane_type {
            PlaneType::A => {
                self.scroll_x = regs.get_hscroll_a() as i32;
                self.scroll_y = regs.get_vscroll_a() as i32;
            }
            PlaneType::B => {
                self.scroll_x = regs.get_hscroll_b() as i32;
                self.scroll_y = regs.get_vscroll_b() as i32;
            }
            PlaneType::Window => {
                // Janela tem scroll fixo ou definido por registrador
                self.scroll_x = 0;
                self.scroll_y = 0;
            }
        }
    }

    /// Lê uma entrada de tile do tilemap
    pub fn read_tile_entry(&self, vram: &Vram, tile_x: usize, tile_y: usize) -> Option<TileEntry> {
        if tile_x >= self.width_tiles || tile_y >= self.height_tiles {
            return None;
        }

        let entry_addr = self.name_table_addr as usize + (tile_y * self.width_tiles + tile_x) * 2;
        if entry_addr + 1 < vram.data.len() {
            let word = (vram.data[entry_addr + 1] as u16) << 8 | vram.data[entry_addr] as u16;
            Some(TileEntry::from_word(word))
        } else {
            None
        }
    }

    /// Renderiza o plano completo para o framebuffer
    pub fn render(
        &self,
        framebuffer: &mut FrameBuffer,
        vram: &Vram,
        cram: &Cram,
        vsram: &Vsram,
        regs: &VdpRegisters,
        mode: &VdpVideoMode,
        render_priority: bool, // true = renderiza tiles com prioridade, false = sem prioridade
    ) {
        if !self.enabled || !mode.display_enabled() {
            return;
        }

        let screen_width = framebuffer.width;
        let screen_height = framebuffer.height;

        // Modo de janela: renderiza apenas na região da janela
        let (render_start_x, render_end_x, render_start_y, render_end_y) = 
            match self.plane_type {
                PlaneType::Window => {
                    let (wx, wy) = regs.get_window_position();
                    if wx == 0 && wy == 0 {
                        // Janela desabilitada
                        return;
                    }
                    
                    let (start_x, end_x) = if wx < 0 {
                        (0, (screen_width as i32 + wx) as usize)
                    } else {
                        (wx as usize, screen_width)
                    };
                    
                    let (start_y, end_y) = if wy < 0 {
                        (0, (screen_height as i32 + wy) as usize)
                    } else {
                        (wy as usize, screen_height)
                    };
                    
                    (start_x, end_x, start_y, end_y)
                }
                _ => (0, screen_width, 0, screen_height),
            };

        // Renderizar cada pixel da tela
        for y in render_start_y..render_end_y {
            for x in render_start_x..render_end_x {
                // Aplicar scroll baseado no tipo de plano
                let (world_x, world_y) = self.apply_scroll(x, y, vsram, regs, mode);
                
                // Calcular coordenadas de tile
                let tile_x = ((world_x as usize) / 8) % self.width_tiles;
                let tile_y = ((world_y as usize) / 8) % self.height_tiles;
                
                // Obter entrada do tile
                if let Some(tile_entry) = self.read_tile_entry(vram, tile_x, tile_y) {
                    // Verificar se devemos renderizar esta prioridade
                    if tile_entry.priority != render_priority {
                        continue;
                    }
                    
                    // Coordenadas dentro do tile
                    let pixel_x = ((world_x as usize) % 8) as u8;
                    let pixel_y = ((world_y as usize) % 8) as u8;
                    
                    // Obter índice de cor do pixel
                    let color_index = if regs.high_color_mode() {
                        tile_entry.get_pixel_color_8bpp(vram, pixel_x, pixel_y)
                    } else {
                        tile_entry.get_pixel_color_4bpp(vram, pixel_x, pixel_y)
                    };
                    
                    // Pixel transparente (índice 0)
                    if color_index == 0 {
                        continue;
                    }
                    
                    // Converter para cor final
                    let palette_offset = (tile_entry.palette as usize) * 16;
                    let color = self.get_final_color(
                        cram, 
                        color_index as usize + palette_offset,
                        mode,
                        regs,
                    );
                    
                    // Desenhar pixel no framebuffer
                    if x < screen_width && y < screen_height {
                        let current_color = framebuffer.get_pixel(x, y).unwrap_or(0);
                        
                        // Aplicar prioridade: só desenha se pixel atual for transparente ou prioridade menor
                        let should_draw = match self.plane_type {
                            PlaneType::Window => {
                                // Janela sempre sobrepõe (a não ser que configurado diferente)
                                let alpha = (color >> 24) & 0xFF;
                                alpha > 0
                            }
                            _ => {
                                let current_alpha = (current_color >> 24) & 0xFF;
                                current_alpha == 0 || self.priority < ((current_color >> 28) & 0x0F) as u8
                            }
                        };
                        
                        if should_draw {
                            framebuffer.set_pixel(x, y, color);
                        }
                    }
                }
            }
        }
    }

    /// Aplica scroll às coordenadas da tela
    fn apply_scroll(
        &self,
        x: usize,
        y: usize,
        vsram: &Vsram,
        regs: &VdpRegisters,
        mode: &VdpVideoMode,
    ) -> (i32, i32) {
        let mut world_x = x as i32 + self.scroll_x;
        let mut world_y = y as i32 + self.scroll_y;

        // Aplicar scroll por linha (row scroll) se habilitado
        if regs.row_scroll_enabled() && self.plane_type == PlaneType::A {
            let row_scroll = vsram.get_row_scroll(y as u16);
            world_x += row_scroll as i32;
        }

        // Aplicar scroll por coluna (column scroll) se habilitado
        if regs.column_scroll_enabled() && self.plane_type == PlaneType::B {
            let column_scroll = vsram.get_column_scroll(x as u16);
            world_y += column_scroll as i32;
        }

        // Wrap-around baseado no tamanho do plano
        world_x = world_x.rem_euclid(self.width_pixels as i32);
        world_y = world_y.rem_euclid(self.height_pixels as i32);

        (world_x, world_y)
    }

    /// Converte índice de cor para cor final (RGB)
    fn get_final_color(
        &self,
        cram: &Cram,
        color_index: usize,
        mode: &VdpVideoMode,
        regs: &VdpRegisters,
    ) -> u32 {
        let color_9bit = cram.read(color_index % 64);
        
        // Extrair componentes RGB (cada um 3 bits: 0-7)
        let r = ((color_9bit >> 0) & 0x07) as u32;
        let g = ((color_9bit >> 4) & 0x07) as u32;
        let b = ((color_9bit >> 8) & 0x07) as u32;
        
        // Converter para 8 bits por canal (0-255)
        // Fórmula: valor * 255 / 7, mas otimizado
        let r8 = (r * 36) as u32;  // 36 ≈ 255/7
        let g8 = (g * 36) as u32;
        let b8 = (b * 36) as u32;
        
        // Alpha: transparente se índice 0, senão opaco
        let alpha = if color_index == 0 { 0x00 } else { 0xFF };
        
        // Prioridade no alpha channel (bits 28-31)
        let priority_alpha = (self.priority as u32) << 28;
        
        (alpha << 24) | (r8 << 16) | (g8 << 8) | b8 | priority_alpha
    }

    /// Renderiza apenas uma região do plano (para otimização)
    pub fn render_region(
        &self,
        framebuffer: &mut FrameBuffer,
        vram: &Vram,
        cram: &Cram,
        vsram: &Vsram,
        regs: &VdpRegisters,
        mode: &VdpVideoMode,
        start_x: usize,
        start_y: usize,
        width: usize,
        height: usize,
        render_priority: bool,
    ) {
        // Similar a render(), mas apenas para uma região específica
        // Útil para atualizações parciais da tela
        let screen_width = framebuffer.width;
        let screen_height = framebuffer.height;
        
        let end_x = (start_x + width).min(screen_width);
        let end_y = (start_y + height).min(screen_height);
        
        for y in start_y..end_y {
            for x in start_x..end_x {
                // Mesma lógica de render(), mas sem duplicar código completo
                // Por brevidade, chamamos render() com região recortada
                // Na prática, seria melhor refatorar a lógica comum
                if x < screen_width && y < screen_height {
                    // TODO: Implementar renderização otimizada por região
                }
            }
        }
    }

    /// Retorna informações sobre o plano
    pub fn get_info(&self) -> String {
        format!(
            "Plane {:?}: {}x{} tiles ({}x{} pixels), addr: 0x{:04X}, enabled: {}, scroll: ({}, {})",
            self.plane_type,
            self.width_tiles,
            self.height_tiles,
            self.width_pixels,
            self.height_pixels,
            self.name_table_addr,
            self.enabled,
            self.scroll_x,
            self.scroll_y
        )
    }

    /// Verifica se um ponto (x, y) da tela está dentro deste plano
    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        match self.plane_type {
            PlaneType::Window => {
                let wx = x - self.window_x;
                let wy = y - self.window_y;
                wx >= 0 && wx < self.width_pixels as i32 &&
                wy >= 0 && wy < self.height_pixels as i32
            }
            _ => {
                // Para planos A e B, cobre toda a tela
                x >= 0 && x < self.width_pixels as i32 &&
                y >= 0 && y < self.height_pixels as i32
            }
        }
    }
}

/// Gerencia todos os planos do VDP
#[derive(Clone)]
pub struct PlaneManager {
    pub plane_a: Plane,
    pub plane_b: Plane,
    pub plane_window: Plane,
    pub plane_order: Vec<PlaneType>, // Ordem de renderização
}

impl PlaneManager {
    /// Cria um novo gerenciador de planos
    pub fn new(regs: &VdpRegisters, mode: &VdpVideoMode) -> Self {
        Self {
            plane_a: Plane::new(PlaneType::A, regs, mode),
            plane_b: Plane::new(PlaneType::B, regs, mode),
            plane_window: Plane::new(PlaneType::Window, regs, mode),
            plane_order: vec![PlaneType::B, PlaneType::A, PlaneType::Window],
        }
    }

    /// Atualiza todos os planos com base nos registradores
    pub fn update(&mut self, regs: &VdpRegisters, mode: &VdpVideoMode) {
        self.plane_a.update_scroll(regs);
        self.plane_b.update_scroll(regs);
        // Window não tem scroll no sentido tradicional
    }

    /// Renderiza todos os planos na ordem correta
    pub fn render_all(
        &self,
        framebuffer: &mut FrameBuffer,
        vram: &Vram,
        cram: &Cram,
        vsram: &Vsram,
        regs: &VdpRegisters,
        mode: &VdpVideoMode,
    ) {
        // Primeiro limpar o framebuffer com a cor de fundo
        let bg_color_index = regs.get_background_color() as usize;
        let bg_color_9bit = cram.read(bg_color_index % 64);
        let (r, g, b) = {
            let r = ((bg_color_9bit >> 0) & 0x07) as u32 * 36;
            let g = ((bg_color_9bit >> 4) & 0x07) as u32 * 36;
            let b = ((bg_color_9bit >> 8) & 0x07) as u32 * 36;
            (r, g, b)
        };
        let bg_color = 0xFF000000 | (r << 16) | (g << 8) | b;
        framebuffer.clear(bg_color);

        // Renderizar planos na ordem especificada
        // Primeiro tiles sem prioridade, depois com prioridade
        for &plane_type in &self.plane_order {
            let plane = match plane_type {
                PlaneType::A => &self.plane_a,
                PlaneType::B => &self.plane_b,
                PlaneType::Window => &self.plane_window,
            };

            // Renderizar tiles sem prioridade primeiro
            plane.render(framebuffer, vram, cram, vsram, regs, mode, false);
            
            // Renderizar tiles com prioridade depois
            plane.render(framebuffer, vram, cram, vsram, regs, mode, true);
        }
    }

    /// Retorna informações de todos os planos
    pub fn get_debug_info(&self) -> Vec<String> {
        vec![
            self.plane_a.get_info(),
            self.plane_b.get_info(),
            self.plane_window.get_info(),
            format!("Render order: {:?}", self.plane_order),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdp::video_modes::VdpVideoMode;
    use crate::vdp::registers::VdpRegisters;

    fn create_test_registers() -> VdpRegisters {
        let mut regs = VdpRegisters::new();
        
        // Configurar endereços dos planos
        regs.set_plane_a_address(0xC000);
        regs.set_plane_b_address(0xE000);
        regs.set_window_address(0xB000);
        
        // Configurar tamanho dos planos (64x32 tiles)
        regs.set_plane_size(PlaneType::A, (64, 32));
        regs.set_plane_size(PlaneType::B, (64, 32));
        regs.set_plane_size(PlaneType::Window, (32, 32));
        
        // Configurar posição da janela
        regs.set_window_position((16, 16));
        
        // Habilitar display
        regs.set(0x01, 0x40);
        
        regs
    }

    #[test]
    fn test_plane_creation() {
        let regs = create_test_registers();
        let mode = VdpVideoMode::new_default();
        
        let plane_a = Plane::new(PlaneType::A, &regs, &mode);
        let plane_b = Plane::new(PlaneType::B, &regs, &mode);
        let plane_window = Plane::new(PlaneType::Window, &regs, &mode);
        
        assert_eq!(plane_a.name_table_addr, 0xC000);
        assert_eq!(plane_b.name_table_addr, 0xE000);
        assert_eq!(plane_window.name_table_addr, 0xB000);
        
        assert_eq!(plane_a.width_tiles, 64);
        assert_eq!(plane_a.height_tiles, 32);
        assert_eq!(plane_a.width_pixels, 512);
        assert_eq!(plane_a.height_pixels, 256);
        
        assert!(plane_a.enabled);
        assert!(plane_b.enabled);
        assert!(plane_window.enabled);
    }

    #[test]
    fn test_tile_entry_decoding() {
        // Teste 1: Tile básico sem atributos especiais
        let word = 0x1234;
        let entry = TileEntry::from_word(word);
        
        assert_eq!(entry.tile_index, 0x0234);
        assert_eq!(entry.palette, 0);
        assert!(!entry.priority);
        assert!(!entry.flip_horizontal);
        assert!(!entry.flip_vertical);
        
        // Teste 2: Tile com todos os atributos
        let word = 0xFEDC;
        let entry = TileEntry::from_word(word);
        
        assert_eq!(entry.tile_index, 0x06DC);
        assert_eq!(entry.palette, 3);
        assert!(entry.priority);
        assert!(entry.flip_horizontal);
        assert!(entry.flip_vertical);
        
        // Teste codificação/decodificação roundtrip
        let original_word = 0xABCD;
        let entry = TileEntry::from_word(original_word);
        let encoded_word = entry.to_word();
        assert_eq!(original_word, encoded_word);
    }

    #[test]
    fn test_plane_scroll() {
        let mut regs = create_test_registers();
        let mode = VdpVideoMode::new_default();
        
        // Configurar scroll
        regs.set_hscroll_a(100);
        regs.set_vscroll_a(50);
        regs.set_hscroll_b(200);
        regs.set_vscroll_b(150);
        
        let mut plane_a = Plane::new(PlaneType::A, &regs, &mode);
        let mut plane_b = Plane::new(PlaneType::B, &regs, &mode);
        
        plane_a.update_scroll(&regs);
        plane_b.update_scroll(&regs);
        
        assert_eq!(plane_a.scroll_x, 100);
        assert_eq!(plane_a.scroll_y, 50);
        assert_eq!(plane_b.scroll_x, 200);
        assert_eq!(plane_b.scroll_y, 150);
    }

    #[test]
    fn test_plane_manager() {
        let regs = create_test_registers();
        let mode = VdpVideoMode::new_default();
        
        let manager = PlaneManager::new(&regs, &mode);
        
        assert_eq!(manager.plane_order.len(), 3);
        assert_eq!(manager.plane_order[0], PlaneType::B);
        assert_eq!(manager.plane_order[1], PlaneType::A);
        assert_eq!(manager.plane_order[2], PlaneType::Window);
        
        let info = manager.get_debug_info();
        assert_eq!(info.len(), 4);
        
        // Verificar que as informações dos planos estão presentes
        assert!(info[0].contains("Plane A"));
        assert!(info[1].contains("Plane B"));
        assert!(info[2].contains("Plane Window"));
        assert!(info[3].contains("Render order"));
    }

    #[test]
    fn test_plane_contains_point() {
        let regs = create_test_registers();
        let mode = VdpVideoMode::new_default();
        
        let plane = Plane::new(PlaneType::A, &regs, &mode);
        
        // Pontos dentro do plano
        assert!(plane.contains_point(0, 0));
        assert!(plane.contains_point(100, 100));
        assert!(plane.contains_point(511, 255));
        
        // Pontos fora do plano
        assert!(!plane.contains_point(-1, 0));
        assert!(!plane.contains_point(0, -1));
        assert!(!plane.contains_point(512, 0));
        assert!(!plane.contains_point(0, 256));
        
        // Teste específico para janela
        let plane_window = Plane::new(PlaneType::Window, &regs, &mode);
        assert!(plane_window.contains_point(16, 16)); // Dentro da janela
        assert!(!plane_window.contains_point(0, 0));  // Fora da janela
    }

    #[test]
    fn test_tile_color_extraction() {
        let mut vram = Vram::new();
        let tile_index = 0;
        
        // Criar um tile 4bpp simples: alternando 0x0 e 0xF
        let mut tile_data = [0u8; 32];
        for i in 0..32 {
            tile_data[i] = if i % 2 == 0 { 0xF0 } else { 0x0F };
        }
        vram.write_tile_4bpp(tile_index, &tile_data);
        
        let entry = TileEntry {
            tile_index: 0,
            palette: 0,
            priority: false,
            flip_horizontal: false,
            flip_vertical: false,
            hscroll_priority: false,
            vscroll_priority: false,
        };
        
        // Testar pixels específicos
        // Pixel (0,0): byte 0, bits 4-7 = 0xF
        let color = entry.get_pixel_color_4bpp(&vram, 0, 0);
        assert_eq!(color, 0xF);
        
        // Pixel (1,0): byte 0, bits 0-3 = 0x0
        let color = entry.get_pixel_color_4bpp(&vram, 1, 0);
        assert_eq!(color, 0x0);
        
        // Pixel (0,1): byte 4, bits 4-7 = 0xF
        let color = entry.get_pixel_color_4bpp(&vram, 0, 1);
        assert_eq!(color, 0xF);
        
        // Testar com flip horizontal
        let mut entry_flipped = entry.clone();
        entry_flipped.flip_horizontal = true;
        
        // Pixel (7,0) normal vs (0,0) com flip
        let color_normal = entry.get_pixel_color_4bpp(&vram, 7, 0);
        let color_flipped = entry_flipped.get_pixel_color_4bpp(&vram, 0, 0);
        assert_eq!(color_normal, color_flipped);
    }
}