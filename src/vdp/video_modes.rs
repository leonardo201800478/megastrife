//! Modos de vídeo do VDP (Mega Drive / Sega Genesis)
//!
//! Implementa todos os modos de exibição suportados pelo VDP:
//! - Modos de resolução (256x224, 320x224, 256x240, 320x240)
//! - Modos de interlace (non-interlace, interlace)
//! - Modos de cores (15-bit, 12-bit shadow/highlight)
//! - Modos especiais (224, 240, 256, 480 linhas)

use bitflags::bitflags;
use crate::vdp::registers::VdpRegisters;

bitflags! {
    /// Flags de modo de vídeo
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct VdpModeFlags: u16 {
        /// Modo H40 (40 células horizontais)
        const H40               = 1 << 0;
        /// Modo interlace (480 linhas)
        const INTERLACE         = 1 << 1;
        /// Modo shadow/highlight (12-bit)
        const SHADOW_HIGHLIGHT  = 1 << 2;
        /// HBlank interrupt habilitado
        const HBLANK_INT        = 1 << 3;
        /// VBlank interrupt habilitado
        const VBLANK_INT        = 1 << 4;
        /// DMA habilitado
        const DMA_ENABLED       = 1 << 5;
        /// Display habilitado
        const DISPLAY_ENABLED   = 1 << 6;
        /// Modo PAL (50Hz)
        const PAL               = 1 << 7;
        /// Interlace a 30Hz
        const INTERLACE_30HZ    = 1 << 8;
        /// Window plane habilitado
        const WINDOW_ENABLED    = 1 << 9;
        /// External interrupt habilitado
        const EXTERNAL_INT      = 1 << 10;
    }
}

/// Estrutura completa de modo de vídeo
#[derive(Clone, Debug, PartialEq)]
pub struct VdpVideoMode {
    pub name: &'static str,
    pub flags: VdpModeFlags,
    pub visible_width: u16,
    pub visible_height: u16,
    pub total_width: u16,
    pub total_height: u16,
    pub color_depth: u8,
    pub palette_size: u16,
    pub supports_shadow_highlight: bool,
    pub pixel_clock: f64,
    pub refresh_rate: f64,
    pub h_total_cycles: u16,
    pub v_total_lines: u16,
    pub plane_a_address: u16,
    pub plane_b_address: u16,
    pub window_address: u16,
    pub sprite_table_address: u16,
    pub hscroll_table_address: u16,
    pub tile_width: u8,
    pub tile_height: u8,
    pub plane_width_tiles: u16,
    pub plane_height_tiles: u16,
}

/// Tipos de resolução suportados
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VdpResolution {
    Res256x224,
    Res320x224,
    Res256x240,
    Res320x240,
    Res256x256,
    Res320x256,
    Res256x448,
    Res320x448,
    Res256x480,
    Res320x480,
}

/// Tipos de renderização
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VdpRenderMode {
    Normal,
    ShadowHighlight,
    RGB444,
}

impl VdpVideoMode {
    /// Cria o modo padrão NTSC 320x224
    pub fn new_default() -> Self {
        Self::create_ntsc_320x224()
    }

    /// Cria modo NTSC 320x224
    pub fn create_ntsc_320x224() -> Self {
        Self {
            name: "NTSC 320x224",
            flags: VdpModeFlags::H40 | VdpModeFlags::DISPLAY_ENABLED,
            visible_width: 320,
            visible_height: 224,
            total_width: 342,
            total_height: 262,
            color_depth: 9,
            palette_size: 64,
            supports_shadow_highlight: false,
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

    /// Cria modo NTSC 256x224
    pub fn create_ntsc_256x224() -> Self {
        let mut mode = Self::create_ntsc_320x224();
        mode.name = "NTSC 256x224";
        mode.flags = VdpModeFlags::DISPLAY_ENABLED;
        mode.visible_width = 256;
        mode.total_width = 320;
        mode.pixel_clock = 6.14;
        mode.plane_width_tiles = 32;
        mode
    }

    /// Cria modo PAL 320x240
    pub fn create_pal_320x240() -> Self {
        Self {
            name: "PAL 320x240",
            flags: VdpModeFlags::H40 | VdpModeFlags::DISPLAY_ENABLED | VdpModeFlags::PAL,
            visible_width: 320,
            visible_height: 240,
            total_width: 424,
            total_height: 313,
            color_depth: 9,
            palette_size: 64,
            supports_shadow_highlight: false,
            pixel_clock: 7.67,
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

    /// Cria modo interlace 320x448
    pub fn create_interlace_320x448() -> Self {
        let mut mode = Self::create_ntsc_320x224();
        mode.name = "Interlace 320x448";
        mode.flags |= VdpModeFlags::INTERLACE;
        mode.visible_height = 448;
        mode.total_height = 525;
        mode.plane_height_tiles = 56;
        mode
    }

    /// Cria modo shadow/highlight 320x224
    pub fn create_shadow_highlight_320x224() -> Self {
        let mut mode = Self::create_ntsc_320x224();
        mode.name = "Shadow/Highlight 320x224";
        mode.flags |= VdpModeFlags::SHADOW_HIGHLIGHT;
        mode.color_depth = 12;
        mode.supports_shadow_highlight = true;
        mode
    }

    /// Obtém modo atual a partir dos registradores
    pub fn from_registers(regs: &VdpRegisters, is_pal: bool) -> Self {
        let r0 = regs.get(0x00);
        let r1 = regs.get(0x01);
        let r12 = regs.get(0x0C);

        let mut flags = VdpModeFlags::empty();

        if (r1 & 0x40) != 0 {
            flags.insert(VdpModeFlags::DISPLAY_ENABLED);
        }
        if (r12 & 0x01) != 0 {
            flags.insert(VdpModeFlags::H40);
        }

        let interlace_mode = (r12 >> 1) & 0x03;
        if interlace_mode >= 2 {
            flags.insert(VdpModeFlags::INTERLACE);
            if interlace_mode == 3 {
                flags.insert(VdpModeFlags::INTERLACE_30HZ);
            }
        }

        if (r12 & 0x08) != 0 {
            flags.insert(VdpModeFlags::SHADOW_HIGHLIGHT);
        }

        if (r0 & 0x10) != 0 {
            flags.insert(VdpModeFlags::HBLANK_INT);
        }
        if (r1 & 0x20) != 0 {
            flags.insert(VdpModeFlags::VBLANK_INT);
        }
        if (r1 & 0x10) != 0 {
            flags.insert(VdpModeFlags::DMA_ENABLED);
        }
        if is_pal {
            flags.insert(VdpModeFlags::PAL);
        }
        if (r0 & 0x20) != 0 {
            flags.insert(VdpModeFlags::EXTERNAL_INT);
        }
        if regs.get_window_address() != 0 {
            flags.insert(VdpModeFlags::WINDOW_ENABLED);
        }

        let mut mode = if flags.contains(VdpModeFlags::H40) {
            if is_pal {
                Self::create_pal_320x240()
            } else {
                Self::create_ntsc_320x224()
            }
        } else {
            Self::create_ntsc_256x224()
        };

        mode.flags = flags;

        if flags.contains(VdpModeFlags::INTERLACE) {
            if flags.contains(VdpModeFlags::H40) {
                mode = Self::create_interlace_320x448();
                mode.flags = flags;
            } else {
                mode.visible_height *= 2;
                mode.total_height = if is_pal { 626 } else { 525 };
                mode.plane_height_tiles *= 2;
                mode.name = "Interlace 256x448";
            }

            if flags.contains(VdpModeFlags::INTERLACE_30HZ) {
                mode.refresh_rate /= 2.0;
            }
        }

        if flags.contains(VdpModeFlags::SHADOW_HIGHLIGHT) {
            mode.color_depth = 12;
            mode.supports_shadow_highlight = true;
            mode.name = if flags.contains(VdpModeFlags::H40) {
                "Shadow/Highlight 320x224"
            } else {
                "Shadow/Highlight 256x224"
            };
        }

        mode.plane_a_address = regs.get_plane_a_address();
        mode.plane_b_address = regs.get_plane_b_address();
        mode.window_address = regs.get_window_address();
        mode.sprite_table_address = regs.get_sprite_table_address();
        mode.hscroll_table_address = regs.get_hscroll_address();

        let plane_size = regs.get_plane_size();
        mode.plane_width_tiles = match (plane_size >> 2) & 0x03 {
            0 => 32,
            1 => 64,
            2 => 128,
            _ => 32,
        };
        mode.plane_height_tiles = match plane_size & 0x03 {
            0 => 32,
            1 => 64,
            2 => 128,
            _ => 32,
        };

        mode
    }

    // =====================================================
    // MÉTODOS DE CONSULTA
    // =====================================================

    pub fn resolution(&self) -> (u16, u16) {
        (self.visible_width, self.visible_height)
    }

    pub fn total_resolution(&self) -> (u16, u16) {
        (self.total_width, self.total_height)
    }

    pub fn is_h40(&self) -> bool {
        self.flags.contains(VdpModeFlags::H40)
    }

    pub fn is_h32(&self) -> bool {
        !self.flags.contains(VdpModeFlags::H40)
    }

    pub fn is_interlace(&self) -> bool {
        self.flags.contains(VdpModeFlags::INTERLACE)
    }

    pub fn is_pal(&self) -> bool {
        self.flags.contains(VdpModeFlags::PAL)
    }

    pub fn is_ntsc(&self) -> bool {
        !self.flags.contains(VdpModeFlags::PAL)
    }

    pub fn has_shadow_highlight(&self) -> bool {
        self.flags.contains(VdpModeFlags::SHADOW_HIGHLIGHT)
    }

    pub fn display_enabled(&self) -> bool {
        self.flags.contains(VdpModeFlags::DISPLAY_ENABLED)
    }

    pub fn render_mode(&self) -> VdpRenderMode {
        if self.has_shadow_highlight() {
            VdpRenderMode::ShadowHighlight
        } else if self.color_depth == 12 {
            VdpRenderMode::RGB444
        } else {
            VdpRenderMode::Normal
        }
    }

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
            _ => VdpResolution::Res320x224,
        }
    }

    pub fn cycles_per_line(&self) -> u16 {
        if self.is_pal() {
            if self.is_h40() { 424 } else { 320 }
        } else if self.is_h40() {
            342
        } else {
            320
        }
    }

    pub fn lines_per_frame(&self) -> u16 {
        if self.is_pal() { 313 } else { 262 }
    }

    pub fn frame_time(&self) -> f64 {
        1.0 / self.refresh_rate
    }

    pub fn line_time(&self) -> f64 {
        self.frame_time() / self.lines_per_frame() as f64
    }

    pub fn is_supported(&self) -> bool {
        if self.has_shadow_highlight() && self.is_interlace() {
            return false;
        }
        if self.plane_width_tiles > 128 || self.plane_height_tiles > 128 {
            return false;
        }
        true
    }

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
