//! Video Display Processor (VDP) - Sega Genesis
//! Responsável por gráficos, sprites, tiles e renderização

mod cram;
mod dma;
mod modes;
mod registers;
mod renderer;
mod vram;

pub use cram::Cram;
pub use dma::DmaController;
pub use registers::{VdpMode, VdpRegisters, VdpStatus};
pub use renderer::{PixelFormat, RenderBuffer, Renderer};
pub use vram::Vram;

use anyhow::{Context, Result};
use log::{debug, info, trace, warn};

/// Resolução padrão do Genesis
pub const GENESIS_WIDTH: usize = 320;
pub const GENESIS_HEIGHT: usize = 224;
pub const NTSC_WIDTH: usize = 320;
pub const NTSC_HEIGHT: usize = 224;
pub const PAL_WIDTH: usize = 320;
pub const PAL_HEIGHT: usize = 240;

/// Estrutura principal do VDP
pub struct VDP {
    /// VRAM (Video RAM) - 64KB
    pub vram: Vram,

    /// CRAM (Color RAM) - Paleta de cores
    pub cram: Cram,

    /// VSRAM (Vertical Scroll RAM)
    pub vsram: [u16; 40],

    /// Registradores do VDP
    pub regs: VdpRegisters,

    /// Controlador DMA
    pub dma: DmaController,

    /// Renderizador
    pub renderer: Renderer,

    /// Status register
    pub status: VdpStatus,

    /// Contadores de scanline
    pub scanline: u16, // Linha atual (0-261)
    pub hcounter: u16, // Contador horizontal
    pub vcounter: u16, // Contador vertical

    /// Buffer de renderização
    pub framebuffer: RenderBuffer,

    /// Habilita/desabilita layers
    pub layer_enable: LayerControl,

    /// Interrupções pendentes
    pub pending_interrupts: VdpInterrupts,
}

/// Controle de layers de renderização
#[derive(Debug, Clone, Copy)]
pub struct LayerControl {
    pub background_a: bool,
    pub background_b: bool,
    pub window: bool,
    pub sprites: bool,
}

/// Interrupções do VDP
#[derive(Debug, Clone, Copy, Default)]
pub struct VdpInterrupts {
    pub hblank: bool,
    pub vblank: bool,
    pub external: bool,
}

impl VDP {
    /// Cria um novo VDP
    pub fn new() -> Self {
        Self {
            vram: Vram::new(),
            cram: Cram::new(),
            vsram: [0; 40],
            regs: VdpRegisters::new(),
            dma: DmaController::new(),
            renderer: Renderer::new(),
            status: VdpStatus::default(),
            scanline: 0,
            hcounter: 0,
            vcounter: 0,
            framebuffer: RenderBuffer::new(GENESIS_WIDTH, GENESIS_HEIGHT),
            layer_enable: LayerControl {
                background_a: true,
                background_b: true,
                window: true,
                sprites: true,
            },
            pending_interrupts: VdpInterrupts::default(),
        }
    }

    /// Inicializa o VDP com valores padrão
    pub fn initialize(&mut self) -> Result<()> {
        debug!("Initializing VDP...");

        // Reset registradores
        self.regs.reset();

        // Limpa memórias
        self.vram.clear();
        self.cram.clear();
        self.vsram = [0; 40];

        // Configuração inicial
        self.status = VdpStatus::default();
        self.scanline = 0;
        self.hcounter = 0;
        self.vcounter = 0;

        // Configura paleta padrão do Genesis
        self.setup_default_palette()?;

        info!("VDP initialized successfully");
        Ok(())
    }

    /// Configura paleta padrão do Genesis
    fn setup_default_palette(&mut self) -> Result<()> {
        // Cores padrão do Genesis (16 cores)
        let default_colors: [(u8, u8, u8); 16] = [
            (0, 0, 0),       // 0: Preto
            (0, 0, 0),       // 1: Preto (duplicado)
            (68, 68, 68),    // 2: Cinza escuro
            (85, 85, 85),    // 3: Cinza médio
            (102, 102, 102), // 4: Cinza claro
            (119, 119, 119), // 5: Cinza mais claro
            (136, 136, 136), // 6: Cinza
            (153, 153, 153), // 7: Cinza
            (170, 170, 170), // 8: Cinza
            (187, 187, 187), // 9: Cinza
            (204, 204, 204), // 10: Cinza
            (221, 221, 221), // 11: Cinza
            (238, 238, 238), // 12: Cinza
            (255, 255, 255), // 13: Branco
            (255, 0, 0),     // 14: Vermelho (teste)
            (0, 255, 0),     // 15: Verde (teste)
        ];

        for (i, &(r, g, b)) in default_colors.iter().enumerate() {
            self.cram.write_color(i as u16, r, g, b)?;
        }

        debug!("Default palette configured (16 colors)");
        Ok(())
    }

    /// Executa um ciclo do VDP
    pub fn step(&mut self, cpu_cycles: u32) -> Result<()> {
        for _ in 0..cpu_cycles {
            self.hcounter += 1;

            // Processa DMA se ativo
            if self.dma.is_active() {
                self.dma.step(&mut self.vram)?;
            }

            // Verifica fim da linha horizontal
            if self.hcounter >= 342 {
                // NTSC: 342 pixels por linha
                self.hcounter = 0;
                self.scanline += 1;

                // Processa fim do scanline
                self.process_scanline_end()?;

                // Verifica fim do frame
                if self.scanline >= 262 {
                    // NTSC: 262 linhas
                    self.scanline = 0;
                    self.process_frame_end()?;
                }
            }

            // Gera interrupção HBlank se necessário
            if self.hcounter == 320 && self.regs.hblank_interrupt_enabled() {
                self.pending_interrupts.hblank = true;
            }
        }

        Ok(())
    }

    /// Processa fim de um scanline
    fn process_scanline_end(&mut self) -> Result<()> {
        // Renderiza a linha atual se estiver na região visível
        if self.scanline < GENESIS_HEIGHT as u16 {
            self.render_scanline(self.scanline)?;
        }

        // Atualiza contador vertical
        self.vcounter = self.scanline;

        trace!("Scanline {} completed", self.scanline);
        Ok(())
    }

    /// Processa fim de um frame
    fn process_frame_end(&mut self) -> Result<()> {
        debug!("Frame completed");

        // Sinaliza VBlank
        self.status.set_vblank(true);

        // Gera interrupção VBlank se habilitada
        if self.regs.vblank_interrupt_enabled() {
            self.pending_interrupts.vblank = true;
        }

        // Limpa contadores
        self.hcounter = 0;

        Ok(())
    }

    /// Renderiza uma linha específica
    fn render_scanline(&mut self, line: u16) -> Result<()> {
        let mode: VdpMode = self.regs.get_mode();

        match mode {
            VdpMode::Mode5 => {
                // Modo 5: Modo principal do Genesis (320x224)
                self.render_mode5_scanline(line)?;
            }
            VdpMode::Mode4 => {
                // Modo 4: 256x224 (menos comum)
                self.render_mode4_scanline(line)?;
            }
            _ => {
                // Outros modos não implementados
                self.render_blank_scanline(line)?;
            }
        }

        Ok(())
    }

    /// Renderiza linha no Modo 5 (principal)
    fn render_mode5_scanline(&mut self, line: u16) -> Result<()> {
        let bg_a_base: u16 = self.regs.get_background_a_base();
        let bg_b_base: u16 = self.regs.get_background_b_base();
        let window_base: u16 = self.regs.get_window_base();

        // Renderiza cada pixel da linha
        for x in 0..GENESIS_WIDTH {
            let mut final_color = 0;

            // Ordem de prioridade: Window > Sprite > BG B > BG A
            if self.layer_enable.window && self.is_window_pixel(x, line) {
                final_color = self.render_window_pixel(x, line, window_base)?;
            } else if self.layer_enable.sprites {
                if let Some(color) = self.render_sprite_pixel(x, line)? {
                    final_color = color;
                } else if self.layer_enable.background_b {
                    final_color = self.render_background_pixel(x, line, bg_b_base, false)?;
                } else if self.layer_enable.background_a {
                    final_color = self.render_background_pixel(x, line, bg_a_base, true)?;
                }
            } else {
                // Sem sprites
                if self.layer_enable.background_b {
                    final_color = self.render_background_pixel(x, line, bg_b_base, false)?;
                }
                if final_color == 0 && self.layer_enable.background_a {
                    final_color = self.render_background_pixel(x, line, bg_a_base, true)?;
                }
            }

            // Escreve no framebuffer
            let idx: usize = (line as usize * GENESIS_WIDTH) + x;
            self.framebuffer.pixels[idx] = final_color;
        }

        Ok(())
    }

    /// Renderiza pixel do background
    fn render_background_pixel(
        &self,
        x: u16,
        y: u16,
        base_addr: u16,
        is_bg_a: bool,
    ) -> Result<u32> {
        // Implementação simplificada
        // Em uma implementação real, você precisaria:
        // 1. Calcular tile coordinates
        // 2. Ler tile data da VRAM
        // 3. Aplicar scroll
        // 4. Obter cor da CRAM

        let tile_x: u16 = (x / 8) as u16;
        let tile_y: u16 = (y / 8) as u16;

        // Calcula endereço do tile
        let tile_addr: u16 = base_addr + (tile_y * 40) + tile_x;

        // Lê dados do tile da VRAM (simplificado)
        let tile_data: () = self.vram.read_word(tile_addr * 2)?;

        // Extrai palette index e tile index
        let palette_index = (tile_data >> 13) & 0x07;
        let tile_index = tile_data & 0x07FF;

        // Calcula pixel dentro do tile
        let pixel_x: u16 = x % 8;
        let pixel_y: u16 = y % 8;

        // Lê pixel data (simplificado)
        let pixel_data_addr =
            (self.regs.get_pattern_base() + (tile_index * 32)) + (pixel_y * 4) + (pixel_x / 2);

        let pixel_data: () = self.vram.read_byte(pixel_data_addr)?;

        // Extrai nibble (4 bits por pixel)
        let color_index = if pixel_x % 2 == 0 {
            (pixel_data >> 4) & 0x0F
        } else {
            pixel_data & 0x0F
        };

        // Se color_index == 0, pixel é transparente
        if color_index == 0 {
            return Ok(0);
        }

        // Obtém cor da CRAM
        let color_addr: u16 = (palette_index * 16) + color_index;
        Ok(self.cram.read_color(color_addr)?)
    }

    /// Renderiza pixel de sprite
    fn render_sprite_pixel(&self, x: u16, y: u16) -> Result<Option<u32>> {
        // Implementação simplificada de sprites
        // O Genesis suporta até 80 sprites

        // Para cada sprite (começando do mais prioritário)
        for sprite_index in 0..80 {
            let sprite_addr: i32 = 0xFC00 + (sprite_index * 8); // Sprite table

            // Lê dados do sprite
            let y_pos: u16 = self.vram.read_word(sprite_addr)? as u16;
            let size_info: () = self.vram.read_word(sprite_addr + 2)?;
            let tile_info: () = self.vram.read_word(sprite_addr + 4)?;
            let x_pos: u16 = self.vram.read_word(sprite_addr + 6)? as u16;

            // Verifica se sprite está ativo
            if y_pos == 0 || y_pos >= 0xE0 {
                continue;
            }

            // Extrai informações
            let width: u16 = match (size_info >> 2) & 0x03 {
                0 => 1, // 1 tile wide
                1 => 2, // 2 tiles wide
                2 => 3, // 3 tiles wide
                _ => 4, // 4 tiles wide
            };

            let height: u16 = match size_info & 0x03 {
                0 => 1,
                1 => 2,
                2 => 3,
                _ => 4,
            };

            // Verifica se pixel está dentro do sprite
            if x >= x_pos && x < x_pos + (width * 8) && y >= y_pos && y < y_pos + (height * 8) {
                // Calcula coordenada relativa
                let rel_x: u16 = x - x_pos;
                let rel_y: u16 = y - y_pos;

                // Obtém cor (simplificado)
                let palette_index = (tile_info >> 13) & 0x07;
                let color_index: i32 = 1; // Exemplo simplificado

                if color_index != 0 {
                    let color_addr = (palette_index * 16) + color_index;
                    return Ok(Some(self.cram.read_color(color_addr)?));
                }
            }
        }

        Ok(None)
    }

    /// Renderiza pixel da window
    fn render_window_pixel(&self, x: u16, y: u16, window_base: u16) -> Result<u32> {
        // Similar ao background, mas com área específica
        self.render_background_pixel(x, y, window_base, false)
    }

    /// Verifica se pixel está na área da window
    fn is_window_pixel(&self, x: u16, y: u16) -> bool {
        let window_x: u16 = self.regs.get_window_x();
        let window_y: u16 = self.regs.get_window_y();

        // Verifica contra área da window (simplificado)
        x >= window_x && y >= window_y
    }

    /// Renderiza linha em branco
    fn render_blank_scanline(&mut self, line: u16) -> Result<()> {
        let start: usize = (line as usize) * GENESIS_WIDTH;
        let end: usize = start + GENESIS_WIDTH;

        // Preenche com cor de borda
        let border_color: u32 = self.cram.read_color(0)?;

        for pixel in &mut self.framebuffer.pixels[start..end] {
            *pixel = border_color;
        }

        Ok(())
    }

    /// Lê do port de dados do VDP
    pub fn read_data_port(&mut self) -> Result<u16> {
        // Implementação do read do port de dados
        match self.regs.get_code() {
            0x00 => {
                // Leitura de VRAM
                let addr: u16 = self.regs.get_address();
                let value: u16 = self.vram.read_word(addr)?;

                // Incrementa endereço
                self.regs.increment_address();

                Ok(value)
            }
            0x04 => {
                // Leitura de CRAM
                let addr: u16 = self.regs.get_address();
                let value: u16 = self.cram.read_color(addr)? as u16;

                self.regs.increment_address();
                Ok(value)
            }
            0x08 => {
                // Leitura de VSRAM
                let addr: usize = self.regs.get_address() as usize / 2;
                let value: u16 = if addr < self.vsram.len() {
                    self.vsram[addr]
                } else {
                    0
                };

                self.regs.increment_address();
                Ok(value)
            }
            _ => {
                warn!("Invalid read code: {:X}", self.regs.get_code());
                Ok(0)
            }
        }
    }

    /// Escreve no port de dados do VDP
    pub fn write_data_port(&mut self, value: u16) -> Result<()> {
        let code: u8 = self.regs.get_code();
        let addr: u16 = self.regs.get_address();

        match code {
            0x00 => {
                // Escrita em VRAM
                self.vram.write_word(addr, value)?;
            }
            0x04 => {
                // Escrita em CRAM
                let r: u8 = ((value >> 1) & 0x1F) as u8 * 8;
                let g: u8 = ((value >> 5) & 0x1F) as u8 * 8;
                let b: u8 = ((value >> 9) & 0x1F) as u8 * 8;

                self.cram.write_color(addr, r, g, b)?;
            }
            0x08 => {
                // Escrita em VSRAM
                let vsram_addr: usize = addr as usize / 2;
                if vsram_addr < self.vsram.len() {
                    self.vsram[vsram_addr] = value;
                }
            }
            0x0C => {
                // Escrita em registro
                self.write_register(value as u8, (value >> 8) as u8)?;
            }
            _ => {
                warn!("Invalid write code: {:X}", code);
            }
        }

        // Incrementa endereço após escrita
        self.regs.increment_address();

        Ok(())
    }

    /// Escreve no port de controle do VDP
    pub fn write_control_port(&mut self, value: u16) -> Result<()> {
        self.regs.process_control_word(value)
    }

    /// Escreve em um registro do VDP
    pub fn write_register(&mut self, reg: u8, value: u8) -> Result<()> {
        self.regs.write(reg, value)
    }

    /// Lê status do VDP
    pub fn read_status(&mut self) -> u16 {
        let status: u16 = self.status.to_u16();

        // Limpa flags após leitura
        self.status.clear();
        self.pending_interrupts.hblank = false;
        self.pending_interrupts.vblank = false;

        status
    }

    /// Verifica se há interrupção pendente
    pub fn has_interrupt(&self) -> bool {
        self.pending_interrupts.hblank
            || self.pending_interrupts.vblank
            || self.pending_interrupts.external
    }

    /// Obtém buffer de renderização para exibição
    pub fn get_framebuffer(&self) -> &RenderBuffer {
        &self.framebuffer
    }

    /// Obtém buffer de renderização mutável
    pub fn get_framebuffer_mut(&mut self) -> &mut RenderBuffer {
        &mut self.framebuffer
    }

    /// Atualiza resolução baseado no modo atual
    pub fn update_resolution(&mut self) -> Result<()> {
        let (width, height) = match self.regs.get_mode() {
            VdpMode::Mode5 => (GENESIS_WIDTH, GENESIS_HEIGHT),
            VdpMode::Mode4 => (256, GENESIS_HEIGHT),
            _ => (320, 240), // Fallback
        };

        if width != self.framebuffer.width || height != self.framebuffer.height {
            debug!("Changing resolution: {}x{}", width, height);
            self.framebuffer = RenderBuffer::new(width, height);
        }

        Ok(())
    }
}
