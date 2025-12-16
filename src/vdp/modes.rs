//! Modos de vídeo do VDP (Mega Drive / Sega Genesis)
//!
//! Implementa todos os modos de exibição suportados pelo VDP:
//! - Modos de resolução (256x224, 320x224, 256x240, 320x240)
//! - Modos de interlace (non-interlace, interlace)
//! - Modos de cores (15-bit, 12-bit shadow/highlight)
//! - Modos especiais (224, 240, 256, 480 lines)
//!
//! Baseado nos registradores do VDP:
//! - R#0: Display enable, HBlank interrupt enable, etc.
//! - R#1: Display enable, VBlank interrupt enable, DMA enable, etc.
//! - R#0xC: H40 mode, interlace, shadow/highlight
//! - R#0xF: Auto-increment value
//! - R#0x10: Plane size, pattern size
//! - R#0x11: Window plane position
//! - R#0x12: Plane A/B base address
//! - R#0x13: Window base address
//! - R#0x15-17: DMA control
//! - R#0x19: Background color
//! - R#0x1A: Horizontal interrupt counter
//! - R#0x1B: Auto-increment value

use crate::vdp::registers::VdpRegisters;

bitflags! {
    /// Flags de modo de vídeo
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct VdpModeFlags: u16 {
        /// Modo H40 (40 células horizontais) vs H32 (32 células)
        const H40 = 1 << 0;
        /// Modo interlace (480 linhas) vs non-interlace (240 linhas)
        const INTERLACE = 1 << 1;
        /// Modo shadow/highlight (12-bit) vs normal (9-bit)
        const SHADOW_HIGHLIGHT = 1 << 2;
        /// Modo HBlank interrupt habilitado
        const HBLANK_INT = 1 << 3;
        /// Modo VBlank interrupt habilitado
        const VBLANK_INT = 1 << 4;
        /// DMA habilitado
        const DMA_ENABLED = 1 << 5;
        /// Display habilitado
        const DISPLAY_ENABLED = 1 << 6;
        /// Modo PAL (50Hz) vs NTSC (60Hz)
        const PAL = 1 << 7;
        /// Modo 30Hz (interlace frame rate)
        const INTERLACE_30HZ = 1 << 8;
        /// Window plane habilitado
        const WINDOW_ENABLED = 1 << 9;
        /// External interrupt habilitado
        const EXTERNAL_INT = 1 << 10;
    }
}

/// Estrutura completa de modo de vídeo
#[derive(Clone, Debug, PartialEq)]
pub struct VdpVideoMode {
    pub name: &'static str,
    pub flags: VdpModeFlags,
    
    // Resolução e timing
    pub visible_width: u16,
    pub visible_height: u16,
    pub total_width: u16,   // Incluindo blanking
    pub total_height: u16,  // Incluindo blanking
    
    // Cores
    pub color_depth: u8,    // Bits por canal (R/G/B)
    pub palette_size: u16,  // Número de cores disponíveis
    pub supports_shadow_highlight: bool,
    
    // Timing
    pub pixel_clock: f64,   // MHz
    pub refresh_rate: f64,  // Hz
    pub h_total_cycles: u16,
    pub v_total_lines: u16,
    
    // Endereçamento de memória
    pub plane_a_address: u16,
    pub plane_b_address: u16,
    pub window_address: u16,
    pub sprite_table_address: u16,
    pub hscroll_table_address: u16,
    
    // Configurações de tile
    pub tile_width: u8,
    pub tile_height: u8,
    pub plane_width_tiles: u16,
    pub plane_height_tiles: u16,
}

/// Tipos de resolução suportados
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VdpResolution {
    Res256x224,   // 32 células x 28 linhas
    Res320x224,   // 40 células x 28 linhas
    Res256x240,   // 32 células x 30 linhas
    Res320x240,   // 40 células x 30 linhas
    Res256x256,   // 32 células x 32 linhas (raro)
    Res320x256,   // 40 células x 32 linhas (raro)
    Res256x448,   // 32 células x 56 linhas (interlace)
    Res320x448,   // 40 células x 56 linhas (interlace)
    Res256x480,   // 32 células x 60 linhas (interlace)
    Res320x480,   // 40 células x 60 linhas (interlace)
}

/// Tipos de renderização
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VdpRenderMode {
    Normal,          // 9-bit color (512 cores)
    ShadowHighlight, // 12-bit color (4096 cores, shadow/highlight)
    RGB444,          // 12-bit RGB (4096 cores)
}

impl VdpVideoMode {
    /// Cria um novo modo de vídeo padrão (NTSC, 320x224, non-interlace)
    pub fn new_default() -> Self {
        Self::create_ntsc_320x224()
    }
    
    /// Cria modo NTSC 320x224 (modo mais comum)
    pub fn create_ntsc_320x224() -> Self {
        VdpVideoMode {
            name: "NTSC 320x224",
            flags: VdpModeFlags::H40 | VdpModeFlags::DISPLAY_ENABLED,
            
            visible_width: 320,
            visible_height: 224,
            total_width: 342,   // NTSC H40
            total_height: 262,  // NTSC total
            
            color_depth: 9,
            palette_size: 64,
            supports_shadow_highlight: false,
            
            pixel_clock: 7.67,  // ~7.67 MHz para NTSC H40
            refresh_rate: 59.94,
            h_total_cycles: 342,
            v_total_lines: 262,
            
            plane_a_address: 0xC000,
            plane_b_address: 0xE000,
            window_address: 0xB000,
            sprite_table_address: 0xF800,
            hscroll_table_address: 0xFC00,
            
            tile_width: 8,
            tile_height: 8,
            plane_width_tiles: 40,
            plane_height_tiles: 28,
        }
    }
    
    /// Cria modo NTSC 256x224
    pub fn create_ntsc_256x224() -> Self {
        VdpVideoMode {
            name: "NTSC 256x224",
            flags: VdpModeFlags::DISPLAY_ENABLED,
            
            visible_width: 256,
            visible_height: 224,
            total_width: 320,   // NTSC H32
            total_height: 262,
            
            color_depth: 9,
            palette_size: 64,
            supports_shadow_highlight: false,
            
            pixel_clock: 6.14,  // ~6.14 MHz para NTSC H32
            refresh_rate: 59.94,
            h_total_cycles: 320,
            v_total_lines: 262,
            
            plane_a_address: 0xC000,
            plane_b_address: 0xE000,
            window_address: 0xB000,
            sprite_table_address: 0xF800,
            hscroll_table_address: 0xFC00,
            
            tile_width: 8,
            tile_height: 8,
            plane_width_tiles: 32,
            plane_height_tiles: 28,
        }
    }
    
    /// Cria modo PAL 320x240
    pub fn create_pal_320x240() -> Self {
        VdpVideoMode {
            name: "PAL 320x240",
            flags: VdpModeFlags::H40 | VdpModeFlags::DISPLAY_ENABLED | VdpModeFlags::PAL,
            
            visible_width: 320,
            visible_height: 240,
            total_width: 424,   // PAL H40
            total_height: 313,  // PAL total
            
            color_depth: 9,
            palette_size: 64,
            supports_shadow_highlight: false,
            
            pixel_clock: 7.67,  // Mesmo clock, mas linhas diferentes
            refresh_rate: 50.0,
            h_total_cycles: 424,
            v_total_lines: 313,
            
            plane_a_address: 0xC000,
            plane_b_address: 0xE000,
            window_address: 0xB000,
            sprite_table_address: 0xF800,
            hscroll_table_address: 0xFC00,
            
            tile_width: 8,
            tile_height: 8,
            plane_width_tiles: 40,
            plane_height_tiles: 30,
        }
    }
    
    /// Cria modo interlace 320x448 (240 linhas * 2)
    pub fn create_interlace_320x448() -> Self {
        VdpVideoMode {
            name: "Interlace 320x448",
            flags: VdpModeFlags::H40 | VdpModeFlags::INTERLACE | VdpModeFlags::DISPLAY_ENABLED,
            
            visible_width: 320,
            visible_height: 448,  // 224*2
            total_width: 342,
            total_height: 525,    // 262*2 (aproximadamente)
            
            color_depth: 9,
            palette_size: 64,
            supports_shadow_highlight: false,
            
            pixel_clock: 7.67,
            refresh_rate: 59.94,  // Still 60Hz, but fields at 30Hz
            h_total_cycles: 342,
            v_total_lines: 525,
            
            plane_a_address: 0xC000,
            plane_b_address: 0xE000,
            window_address: 0xB000,
            sprite_table_address: 0xF800,
            hscroll_table_address: 0xFC00,
            
            tile_width: 8,
            tile_height: 8,
            plane_width_tiles: 40,
            plane_height_tiles: 56,  // 28*2
        }
    }
    
    /// Cria modo shadow/highlight (12-bit color)
    pub fn create_shadow_highlight_320x224() -> Self {
        VdpVideoMode {
            name: "Shadow/Highlight 320x224",
            flags: VdpModeFlags::H40 | VdpModeFlags::DISPLAY_ENABLED | VdpModeFlags::SHADOW_HIGHLIGHT,
            
            visible_width: 320,
            visible_height: 224,
            total_width: 342,
            total_height: 262,
            
            color_depth: 12,     // 4 bits por canal
            palette_size: 64,
            supports_shadow_highlight: true,
            
            pixel_clock: 7.67,
            refresh_rate: 59.94,
            h_total_cycles: 342,
            v_total_lines: 262,
            
            plane_a_address: 0xC000,
            plane_b_address: 0xE000,
            window_address: 0xB000,
            sprite_table_address: 0xF800,
            hscroll_table_address: 0xFC00,
            
            tile_width: 8,
            tile_height: 8,
            plane_width_tiles: 40,
            plane_height_tiles: 28,
        }
    }
    
    /// Obtém modo atual a partir dos registradores
    pub fn from_registers(regs: &VdpRegisters, is_pal: bool) -> Self {
        let r0 = regs.get(0x00);
        let r1 = regs.get(0x01);
        let r12 = regs.get(0x0C);
        
        let mut flags = VdpModeFlags::empty();
        
        // Display habilitado
        if (r1 & 0x40) != 0 {
            flags.insert(VdpModeFlags::DISPLAY_ENABLED);
        }
        
        // H40 mode (bit 6 do registrador 12)
        if (r12 & 0x01) != 0 {
            flags.insert(VdpModeFlags::H40);
        }
        
        // Interlace (bits 1-2 do registrador 12)
        let interlace_mode = (r12 >> 1) & 0x03;
        if interlace_mode >= 2 {
            flags.insert(VdpModeFlags::INTERLACE);
            if interlace_mode == 3 {
                flags.insert(VdpModeFlags::INTERLACE_30HZ);
            }
        }
        
        // Shadow/Highlight (bit 3 do registrador 12)
        if (r12 & 0x08) != 0 {
            flags.insert(VdpModeFlags::SHADOW_HIGHLIGHT);
        }
        
        // Interrupt enables
        if (r0 & 0x10) != 0 {
            flags.insert(VdpModeFlags::HBLANK_INT);
        }
        if (r1 & 0x20) != 0 {
            flags.insert(VdpModeFlags::VBLANK_INT);
        }
        
        // DMA enable
        if (r1 & 0x10) != 0 {
            flags.insert(VdpModeFlags::DMA_ENABLED);
        }
        
        // PAL/NTSC
        if is_pal {
            flags.insert(VdpModeFlags::PAL);
        }
        
        // External interrupt (bit 5 do registrador 0)
        if (r0 & 0x20) != 0 {
            flags.insert(VdpModeFlags::EXTERNAL_INT);
        }
        
        // Window enable (determinado por endereço não-zero)
        if regs.get_window_address() != 0 {
            flags.insert(VdpModeFlags::WINDOW_ENABLED);
        }
        
        // Determinar modo base
        let mut mode = if flags.contains(VdpModeFlags::H40) {
            if is_pal {
                Self::create_pal_320x240()
            } else {
                Self::create_ntsc_320x224()
            }
        } else {
            Self::create_ntsc_256x224()
        };
        
        // Aplicar flags
        mode.flags = flags;
        
        // Atualizar para interlace se necessário
        if flags.contains(VdpModeFlags::INTERLACE) {
            if flags.contains(VdpModeFlags::H40) {
                mode = Self::create_interlace_320x448();
                mode.flags = flags;
            } else {
                // Interlace H32
                mode.visible_height *= 2;
                mode.total_height = if is_pal { 626 } else { 525 };
                mode.plane_height_tiles *= 2;
                mode.name = if flags.contains(VdpModeFlags::H40) {
                    "Interlace 320x448"
                } else {
                    "Interlace 256x448"
                };
            }
            
            // Ajustar refresh rate para 30Hz se for interlaced frame
            if flags.contains(VdpModeFlags::INTERLACE_30HZ) {
                mode.refresh_rate /= 2.0;
            }
        }
        
        // Atualizar para shadow/highlight se necessário
        if flags.contains(VdpModeFlags::SHADOW_HIGHLIGHT) {
            mode.color_depth = 12;
            mode.supports_shadow_highlight = true;
            mode.name = if flags.contains(VdpModeFlags::H40) {
                "Shadow/Highlight 320x224"
            } else {
                "Shadow/Highlight 256x224"
            };
        }
        
        // Atualizar endereços dos planos
        mode.plane_a_address = regs.get_plane_a_address();
        mode.plane_b_address = regs.get_plane_b_address();
        mode.window_address = regs.get_window_address();
        mode.sprite_table_address = regs.get_sprite_table_address();
        mode.hscroll_table_address = regs.get_hscroll_address();
        
        // Atualizar tamanho do tile baseado no registrador
        let tile_size = regs.get_tile_size();
        mode.tile_width = if (tile_size & 0x01) != 0 { 8 } else { 8 }; // Sempre 8 no MD
        mode.tile_height = if (tile_size & 0x01) != 0 { 8 } else { 8 };
        
        // Atualizar tamanho do plano
        let plane_size = regs.get_plane_size();
        mode.plane_width_tiles = match (plane_size >> 2) & 0x03 {
            0 => 32,
            1 => 64,
            2 => 128,
            3 => 32, // Invalido, fallback para 32
            _ => 32,
        };
        
        mode.plane_height_tiles = match plane_size & 0x03 {
            0 => 32,
            1 => 64,
            2 => 128,
            3 => 32, // Invalido, fallback para 32
            _ => 32,
        };
        
        mode
    }
    
    // =====================================================
    // MÉTODOS DE CONSULTA
    // =====================================================
    
    /// Retorna a resolução visível
    pub fn resolution(&self) -> (u16, u16) {
        (self.visible_width, self.visible_height)
    }
    
    /// Retorna a resolução total (incluindo blanking)
    pub fn total_resolution(&self) -> (u16, u16) {
        (self.total_width, self.total_height)
    }
    
    /// Retorna true se for modo H40 (320 pixels)
    pub fn is_h40(&self) -> bool {
        self.flags.contains(VdpModeFlags::H40)
    }
    
    /// Retorna true se for modo H32 (256 pixels)
    pub fn is_h32(&self) -> bool {
        !self.flags.contains(VdpModeFlags::H40)
    }
    
    /// Retorna true se for modo interlace
    pub fn is_interlace(&self) -> bool {
        self.flags.contains(VdpModeFlags::INTERLACE)
    }
    
    /// Retorna true se for modo PAL
    pub fn is_pal(&self) -> bool {
        self.flags.contains(VdpModeFlags::PAL)
    }
    
    /// Retorna true se for modo NTSC
    pub fn is_ntsc(&self) -> bool {
        !self.flags.contains(VdpModeFlags::PAL)
    }
    
    /// Retorna true se suporta shadow/highlight
    pub fn has_shadow_highlight(&self) -> bool {
        self.flags.contains(VdpModeFlags::SHADOW_HIGHLIGHT)
    }
    
    /// Retorna true se display está habilitado
    pub fn display_enabled(&self) -> bool {
        self.flags.contains(VdpModeFlags::DISPLAY_ENABLED)
    }
    
    /// Retorna o modo de renderização
    pub fn render_mode(&self) -> VdpRenderMode {
        if self.has_shadow_highlight() {
            VdpRenderMode::ShadowHighlight
        } else if self.color_depth == 12 {
            VdpRenderMode::RGB444
        } else {
            VdpRenderMode::Normal
        }
    }
    
    /// Retorna a resolução como enum
    pub fn resolution_type(&self) -> VdpResolution {
        match (self.visible_width, self.visible_height) {
            (256, 224) => VdpResolution::Res256x224,
            (320, 224) => VdpResolution::Res320x224,
            (256, 240) => VdpResolution::Res256x240,
            (320, 240) => VdpResolution::Res320x240,
            (256, 256) => VdpResolution::Res256x256,
            (320, 256) => VdpResolution::Res320x256,
            (256, 448) => VdpResolution::Res256x448,
            (320, 448) => VdpResolution::Res320x448,
            (256, 480) => VdpResolution::Res256x480,
            (320, 480) => VdpResolution::Res320x480,
            _ => VdpResolution::Res320x224, // Fallback
        }
    }
    
    /// Retorna ciclos por linha baseado no modo
    pub fn cycles_per_line(&self) -> u16 {
        if self.is_pal() {
            if self.is_h40() { 424 } else { 320 }
        } else {
            if self.is_h40() { 342 } else { 320 }
        }
    }
    
    /// Retorna linhas totais por quadro
    pub fn lines_per_frame(&self) -> u16 {
        if self.is_pal() {
            313
        } else {
            262
        }
    }
    
    /// Calcula tempo por frame em segundos
    pub fn frame_time(&self) -> f64 {
        1.0 / self.refresh_rate
    }
    
    /// Calcula tempo por linha em segundos
    pub fn line_time(&self) -> f64 {
        self.frame_time() / self.lines_per_frame() as f64
    }
    
    /// Retorna se o modo é suportado (algumas combinações são inválidas)
    pub fn is_supported(&self) -> bool {
        // Verificar combinações inválidas
        if self.has_shadow_highlight() && self.is_interlace() {
            // Shadow/highlight não suportado em interlace no hardware real
            return false;
        }
        
        if self.plane_width_tiles > 128 || self.plane_height_tiles > 128 {
            // Tamanhos de plano inválidos
            return false;
        }
        
        true
    }
    
    /// Retorna informações do modo como string
    pub fn to_string(&self) -> String {
        format!(
            "{}: {}x{} @ {:.2}Hz, {} colors, {}",
            self.name,
            self.visible_width,
            self.visible_height,
            self.refresh_rate,
            self.palette_size,
            if self.is_pal() { "PAL" } else { "NTSC" }
        )
    }
    
    /// Retorna configurações detalhadas
    pub fn debug_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("Name", self.name.to_string()),
            ("Resolution", format!("{}x{}", self.visible_width, self.visible_height)),
            ("Total", format!("{}x{}", self.total_width, self.total_height)),
            ("Refresh Rate", format!("{:.2} Hz", self.refresh_rate)),
            ("Color Depth", format!("{} bits", self.color_depth)),
            ("Palette Size", format!("{} colors", self.palette_size)),
            ("Pixel Clock", format!("{:.2} MHz", self.pixel_clock)),
            ("Mode", if self.is_h40() { "H40" } else { "H32" }.to_string()),
            ("Interlace", if self.is_interlace() { "Yes" } else { "No" }.to_string()),
            ("System", if self.is_pal() { "PAL" } else { "NTSC" }.to_string()),
            ("Display", if self.display_enabled() { "Enabled" } else { "Disabled" }.to_string()),
            ("Supported", if self.is_supported() { "Yes" } else { "No" }.to_string()),
        ]
    }
}

impl Default for VdpVideoMode {
    fn default() -> Self {
        Self::new_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdp::registers::VdpRegisters;
    
    #[test]
    fn test_default_mode() {
        let mode = VdpVideoMode::new_default();
        assert_eq!(mode.name, "NTSC 320x224");
        assert_eq!(mode.visible_width, 320);
        assert_eq!(mode.visible_height, 224);
        assert_eq!(mode.refresh_rate, 59.94);
        assert!(mode.display_enabled());
        assert!(mode.is_h40());
        assert!(!mode.is_interlace());
        assert!(!mode.is_pal());
    }
    
    #[test]
    fn test_mode_from_registers_ntsc_h40() {
        let mut regs = VdpRegisters::new();
        
        // Configurar para modo H40, display enabled
        regs.set(0x01, 0x40); // Display enabled
        regs.set(0x0C, 0x01); // H40 mode
        
        let mode = VdpVideoMode::from_registers(&regs, false);
        
        assert_eq!(mode.visible_width, 320);
        assert_eq!(mode.visible_height, 224);
        assert!(mode.is_h40());
        assert!(!mode.is_interlace());
        assert!(!mode.is_pal());
        assert!(mode.display_enabled());
    }
    
    #[test]
    fn test_mode_from_registers_ntsc_h32() {
        let mut regs = VdpRegisters::new();
        
        // Configurar para modo H32, display enabled
        regs.set(0x01, 0x40); // Display enabled
        regs.set(0x0C, 0x00); // H32 mode
        
        let mode = VdpVideoMode::from_registers(&regs, false);
        
        assert_eq!(mode.visible_width, 256);
        assert_eq!(mode.visible_height, 224);
        assert!(mode.is_h32());
        assert!(!mode.is_h40());
    }
    
    #[test]
    fn test_mode_from_registers_pal() {
        let mut regs = VdpRegisters::new();
        
        // Configurar para modo PAL H40
        regs.set(0x01, 0x40); // Display enabled
        regs.set(0x0C, 0x01); // H40 mode
        
        let mode = VdpVideoMode::from_registers(&regs, true);
        
        assert_eq!(mode.visible_width, 320);
        assert_eq!(mode.visible_height, 240);
        assert!(mode.is_pal());
        assert_eq!(mode.refresh_rate, 50.0);
    }
    
    #[test]
    fn test_mode_from_registers_interlace() {
        let mut regs = VdpRegisters::new();
        
        // Configurar para modo interlace H40
        regs.set(0x01, 0x40); // Display enabled
        regs.set(0x0C, 0x03); // H40 mode + interlace
        
        let mode = VdpVideoMode::from_registers(&regs, false);
        
        assert!(mode.is_interlace());
        assert_eq!(mode.visible_height, 448); // 224 * 2
        assert!(mode.flags.contains(VdpModeFlags::INTERLACE));
    }
    
    #[test]
    fn test_mode_from_registers_shadow_highlight() {
        let mut regs = VdpRegisters::new();
        
        // Configurar para modo shadow/highlight
        regs.set(0x01, 0x40); // Display enabled
        regs.set(0x0C, 0x09); // H40 mode + shadow/highlight
        
        let mode = VdpVideoMode::from_registers(&regs, false);
        
        assert!(mode.has_shadow_highlight());
        assert_eq!(mode.color_depth, 12);
        assert_eq!(mode.render_mode(), VdpRenderMode::ShadowHighlight);
    }
    
    #[test]
    fn test_mode_properties() {
        let mode = VdpVideoMode::new_default();
        
        let (w, h) = mode.resolution();
        assert_eq!(w, 320);
        assert_eq!(h, 224);
        
        let (tw, th) = mode.total_resolution();
        assert_eq!(tw, 342);
        assert_eq!(th, 262);
        
        assert!(mode.display_enabled());
        assert!(!mode.is_pal());
        assert_eq!(mode.cycles_per_line(), 342);
        assert_eq!(mode.lines_per_frame(), 262);
        
        // Verificar que frame_time é consistente
        let frame_time = mode.frame_time();
        let expected_time = 1.0 / 59.94;
        assert!((frame_time - expected_time).abs() < 0.001);
    }
    
    #[test]
    fn test_resolution_type() {
        let mut mode = VdpVideoMode::new_default();
        
        mode.visible_width = 256;
        mode.visible_height = 224;
        assert_eq!(mode.resolution_type(), VdpResolution::Res256x224);
        
        mode.visible_width = 320;
        mode.visible_height = 240;
        assert_eq!(mode.resolution_type(), VdpResolution::Res320x240);
        
        mode.visible_width = 320;
        mode.visible_height = 448;
        assert_eq!(mode.resolution_type(), VdpResolution::Res320x448);
    }
    
    #[test]
    fn test_plane_addresses() {
        let mut regs = VdpRegisters::new();
        
        // Configurar endereços personalizados
        regs.set_plane_a_address(0x8000);
        regs.set_plane_b_address(0xA000);
        regs.set_window_address(0x9000);
        
        let mode = VdpVideoMode::from_registers(&regs, false);
        
        assert_eq!(mode.plane_a_address, 0x8000);
        assert_eq!(mode.plane_b_address, 0xA000);
        assert_eq!(mode.window_address, 0x9000);
    }
    
    #[test]
    fn test_debug_info() {
        let mode = VdpVideoMode::new_default();
        let info = mode.debug_info();
        
        // Verificar que todas as informações estão presentes
        assert!(info.iter().any(|(k, _)| *k == "Name"));
        assert!(info.iter().any(|(k, _)| *k == "Resolution"));
        assert!(info.iter().any(|(k, _)| *k == "Refresh Rate"));
        assert!(info.iter().any(|(k, _)| *k == "Color Depth"));
        
        // Verificar string format
        let string = mode.to_string();
        assert!(string.contains("NTSC 320x224"));
        assert!(string.contains("59.94"));
    }
    
    #[test]
    fn test_supported_modes() {
        // Testar modo válido
        let valid_mode = VdpVideoMode::new_default();
        assert!(valid_mode.is_supported());
        
        // Testar modo inválido (shadow/highlight + interlace)
        let mut invalid_mode = valid_mode.clone();
        invalid_mode.flags.insert(VdpModeFlags::SHADOW_HIGHLIGHT);
        invalid_mode.flags.insert(VdpModeFlags::INTERLACE);
        assert!(!invalid_mode.is_supported());
    }
}