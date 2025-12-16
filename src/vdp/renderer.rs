//! Renderer principal do VDP
//!
//! Responsável por coordenar a renderização completa de um frame:
//! 1. Plano B (background)
//! 2. Plano A (foreground)
//! 3. Plano Window (sobreposição)
//! 4. Sprites (com prioridade dupla)
//!
//! Ordem de renderização do VDP:
//! 1. Planos (tiles sem prioridade)
//! 2. Sprites (sem prioridade)
//! 3. Planos (tiles com prioridade)
//! 4. Sprites (com prioridade)

use crate::vdp::{
    cram::Cram,
    framebuffer::FrameBuffer,
    modes::{VdpVideoMode, VdpRenderMode},
    planes::{Plane, PlaneManager, PlaneType},
    registers::VdpRegisters,
    sprite::SpriteTable,
    vram::Vram,
    vsram::Vsram,
};

/// Renderer principal do VDP
#[derive(Clone)]
pub struct VdpRenderer {
    pub video_mode: VdpVideoMode,
    pub frame_buffer: FrameBuffer,
    pub last_frame_time: f64,
    pub frames_rendered: u64,
    pub render_enabled: bool,
    pub show_background: bool,
    pub show_planes: bool,
    pub show_window: bool,
    pub show_sprites: bool,
    pub debug_overlay: bool,
}

impl VdpRenderer {
    /// Cria um novo renderer com o modo de vídeo especificado
    pub fn new(video_mode: VdpVideoMode) -> Self {
        let (width, height) = video_mode.resolution();
        let frame_buffer = FrameBuffer::new(width as usize, height as usize);
        
        Self {
            video_mode,
            frame_buffer,
            last_frame_time: 0.0,
            frames_rendered: 0,
            render_enabled: true,
            show_background: true,
            show_planes: true,
            show_window: true,
            show_sprites: true,
            debug_overlay: false,
        }
    }
    
    /// Renderiza um frame completo
    pub fn render_frame(
        &mut self,
        regs: &VdpRegisters,
        cram: &Cram,
        vram: &Vram,
        vsram: &Vsram,
        sprite_table: &SpriteTable,
        timestamp: f64,
    ) -> &FrameBuffer {
        if !self.render_enabled || !self.video_mode.display_enabled() {
            return &self.frame_buffer;
        }
        
        // Atualizar modo de vídeo baseado nos registradores
        self.update_video_mode(regs);
        
        // Renderizar o frame
        self.render_internal(regs, cram, vram, vsram, sprite_table);
        
        // Aplicar efeitos pós-processamento
        self.apply_post_processing(regs);
        
        // Atualizar estatísticas
        self.last_frame_time = timestamp;
        self.frames_rendered += 1;
        
        &self.frame_buffer
    }
    
    /// Renderiza internamente o frame (sem pós-processamento)
    fn render_internal(
        &mut self,
        regs: &VdpRegisters,
        cram: &Cram,
        vram: &Vram,
        vsram: &Vsram,
        sprite_table: &SpriteTable,
    ) {
        let (width, height) = self.video_mode.resolution();
        let screen_width = width as usize;
        let screen_height = height as usize;
        
        // Verificar se o framebuffer tem o tamanho correto
        if self.frame_buffer.width != screen_width || self.frame_buffer.height != screen_height {
            self.frame_buffer.resize(screen_width, screen_height);
        }
        
        // Obter cor de fundo do registrador
        let bg_color_index = regs.get_background_color() as usize;
        let bg_color = self.get_color_from_cram(cram, bg_color_index);
        
        // Limpar com cor de fundo se habilitado
        if self.show_background {
            self.frame_buffer.clear(bg_color);
        } else {
            self.frame_buffer.clear(0x00000000); // Transparente
        }
        
        // Criar gerenciador de planos
        let mut plane_manager = PlaneManager::new(regs, &self.video_mode);
        plane_manager.update(regs, &self.video_mode);
        
        // 1. Renderizar tiles sem prioridade de todos os planos
        if self.show_planes || self.show_window {
            self.render_low_priority_tiles(&plane_manager, regs, cram, vram, vsram);
        }
        
        // 2. Renderizar sprites sem prioridade
        if self.show_sprites {
            self.render_low_priority_sprites(sprite_table, vram, cram);
        }
        
        // 3. Renderizar tiles com prioridade de todos os planos
        if self.show_planes || self.show_window {
            self.render_high_priority_tiles(&plane_manager, regs, cram, vram, vsram);
        }
        
        // 4. Renderizar sprites com prioridade
        if self.show_sprites {
            self.render_high_priority_sprites(sprite_table, vram, cram);
        }
        
        // 5. Renderizar overlay de debug (se habilitado)
        if self.debug_overlay {
            self.render_debug_overlay(regs, &plane_manager, sprite_table);
        }
    }
    
    /// Renderiza tiles sem prioridade
    fn render_low_priority_tiles(
        &mut self,
        plane_manager: &PlaneManager,
        regs: &VdpRegisters,
        cram: &Cram,
        vram: &Vram,
        vsram: &Vsram,
    ) {
        // Criar um framebuffer temporário para composição de planos
        let mut temp_buffer = FrameBuffer::new(self.frame_buffer.width, self.frame_buffer.height);
        
        // Renderizar planos na ordem correta (B, A, Window)
        for &plane_type in &plane_manager.plane_order {
            match plane_type {
                PlaneType::A if self.show_planes => {
                    plane_manager.plane_a.render(
                        &mut temp_buffer,
                        vram,
                        cram,
                        vsram,
                        regs,
                        &self.video_mode,
                        false, // Sem prioridade
                    );
                }
                PlaneType::B if self.show_planes => {
                    plane_manager.plane_b.render(
                        &mut temp_buffer,
                        vram,
                        cram,
                        vsram,
                        regs,
                        &self.video_mode,
                        false, // Sem prioridade
                    );
                }
                PlaneType::Window if self.show_window => {
                    plane_manager.plane_window.render(
                        &mut temp_buffer,
                        vram,
                        cram,
                        vsram,
                        regs,
                        &self.video_mode,
                        false, // Sem prioridade
                    );
                }
                _ => {}
            }
        }
        
        // Mesclar com framebuffer principal (somente pixels não transparentes)
        self.merge_framebuffers(&temp_buffer);
    }
    
    /// Renderiza tiles com prioridade
    fn render_high_priority_tiles(
        &mut self,
        plane_manager: &PlaneManager,
        regs: &VdpRegisters,
        cram: &Cram,
        vram: &Vram,
        vsram: &Vsram,
    ) {
        // Criar um framebuffer temporário
        let mut temp_buffer = FrameBuffer::new(self.frame_buffer.width, self.frame_buffer.height);
        
        // Renderizar planos na ordem correta
        for &plane_type in &plane_manager.plane_order {
            match plane_type {
                PlaneType::A if self.show_planes => {
                    plane_manager.plane_a.render(
                        &mut temp_buffer,
                        vram,
                        cram,
                        vsram,
                        regs,
                        &self.video_mode,
                        true, // Com prioridade
                    );
                }
                PlaneType::B if self.show_planes => {
                    plane_manager.plane_b.render(
                        &mut temp_buffer,
                        vram,
                        cram,
                        vsram,
                        regs,
                        &self.video_mode,
                        true, // Com prioridade
                    );
                }
                PlaneType::Window if self.show_window => {
                    plane_manager.plane_window.render(
                        &mut temp_buffer,
                        vram,
                        cram,
                        vsram,
                        regs,
                        &self.video_mode,
                        true, // Com prioridade
                    );
                }
                _ => {}
            }
        }
        
        // Mesclar com framebuffer principal
        self.merge_framebuffers(&temp_buffer);
    }
    
    /// Renderiza sprites sem prioridade
    fn render_low_priority_sprites(
        &mut self,
        sprite_table: &SpriteTable,
        vram: &Vram,
        cram: &Cram,
    ) {
        // Criar um framebuffer temporário para sprites
        let mut temp_buffer = FrameBuffer::new(self.frame_buffer.width, self.frame_buffer.height);
        
        // Renderizar sprites sem prioridade
        for sprite in &sprite_table.sprites {
            if sprite.valid && sprite.visible && !sprite.priority {
                sprite.render(&mut temp_buffer, vram, cram, None);
            }
        }
        
        // Mesclar com framebuffer principal
        self.merge_framebuffers(&temp_buffer);
    }
    
    /// Renderiza sprites com prioridade
    fn render_high_priority_sprites(
        &mut self,
        sprite_table: &SpriteTable,
        vram: &Vram,
        cram: &Cram,
    ) {
        // Criar um framebuffer temporário para sprites
        let mut temp_buffer = FrameBuffer::new(self.frame_buffer.width, self.frame_buffer.height);
        
        // Renderizar sprites com prioridade
        for sprite in &sprite_table.sprites {
            if sprite.valid && sprite.visible && sprite.priority {
                sprite.render(&mut temp_buffer, vram, cram, None);
            }
        }
        
        // Mesclar com framebuffer principal
        self.merge_framebuffers(&temp_buffer);
    }
    
    /// Mescla dois framebuffers (destino = this, fonte = other)
    fn merge_framebuffers(&mut self, other: &FrameBuffer) {
        for y in 0..self.frame_buffer.height {
            for x in 0..self.frame_buffer.width {
                if let Some(src_color) = other.get_pixel(x, y) {
                    // Verificar se pixel fonte não é transparente (alpha > 0)
                    let alpha = (src_color >> 24) & 0xFF;
                    if alpha > 0 {
                        // Obter cor atual no destino
                        let dst_color = self.frame_buffer.get_pixel(x, y).unwrap_or(0);
                        
                        // Verificar prioridade (bits 28-31 do alpha channel)
                        let src_priority = (src_color >> 28) & 0x0F;
                        let dst_priority = (dst_color >> 28) & 0x0F;
                        
                        // Se pixel fonte tem prioridade maior ou destino é transparente, substituir
                        if src_priority > dst_priority || ((dst_color >> 24) & 0xFF) == 0 {
                            self.frame_buffer.set_pixel(x, y, src_color);
                        } else if src_priority == dst_priority && alpha == 0xFF {
                            // Mesma prioridade e totalmente opaco: usar fonte
                            self.frame_buffer.set_pixel(x, y, src_color);
                        }
                        // Caso contrário, manter destino
                    }
                }
            }
        }
    }
    
    /// Atualiza o modo de vídeo baseado nos registradores
    fn update_video_mode(&mut self, regs: &VdpRegisters) {
        // Obtém configuração PAL/NTSC do sistema (simplificado)
        let is_pal = false; // Por padrão assume NTSC, deveria vir de configuração
        
        // Criar novo modo de vídeo baseado nos registradores
        let new_mode = VdpVideoMode::from_registers(regs, is_pal);
        
        // Se o modo mudou, redimensionar framebuffer
        if new_mode.resolution() != self.video_mode.resolution() {
            let (width, height) = new_mode.resolution();
            self.frame_buffer.resize(width as usize, height as usize);
        }
        
        self.video_mode = new_mode;
    }
    
    /// Aplica pós-processamento baseado nos registradores
    fn apply_post_processing(&mut self, regs: &VdpRegisters) {
        // Aplicar modo shadow/highlight se habilitado
        if self.video_mode.has_shadow_highlight() {
            self.apply_shadow_highlight_effect(regs);
        }
        
        // Aplicar efeito de interlace se necessário
        if self.video_mode.is_interlace() {
            self.apply_interlace_effect();
        }
        
        // Ajustar brilho/contraste baseado em configurações (se houver)
        let brightness = 0.0; // Deveria vir dos registradores
        let contrast = 0.0;   // Deveria vir dos registradores
        if brightness != 0.0 || contrast != 0.0 {
            self.frame_buffer.adjust_brightness_contrast(brightness, contrast);
        }
    }
    
    /// Aplica efeito shadow/highlight (modo de 12 bits)
    fn apply_shadow_highlight_effect(&mut self, regs: &VdpRegisters) {
        // Em modo shadow/highlight, cada canal de cor tem 4 bits (0-15)
        // O VDP aplica uma curva de correção específica
        
        for y in 0..self.frame_buffer.height {
            for x in 0..self.frame_buffer.width {
                if let Some(color) = self.frame_buffer.get_pixel(x, y) {
                    let alpha = (color >> 24) & 0xFF;
                    
                    // Extrair componentes (originalmente 0-7, mas expandidos para 0-255)
                    let r = ((color >> 16) & 0xFF) as f32;
                    let g = ((color >> 8) & 0xFF) as f32;
                    let b = (color & 0xFF) as f32;
                    
                    // Aplicar curva shadow/highlight do VDP
                    // Simplificação: converter de 9-bit (0-511) para 12-bit (0-4095)
                    // e depois para 8-bit (0-255)
                    
                    // Na prática, o VDP tem registradores específicos para shadow/highlight
                    // que ajustam a intensidade. Aqui fazemos uma simulação simples.
                    
                    let r_new = (r * 1.2).min(255.0) as u32; // Aumentar brilho
                    let g_new = (g * 1.2).min(255.0) as u32;
                    let b_new = (b * 1.2).min(255.0) as u32;
                    
                    let new_color = (alpha << 24) | (r_new << 16) | (g_new << 8) | b_new;
                    self.frame_buffer.set_pixel(x, y, new_color);
                }
            }
        }
    }
    
    /// Aplica efeito de interlace (para modos de alta resolução)
    fn apply_interlace_effect(&mut self) {
        // Em modo interlace, alternamos entre campos pares e ímpares
        // Para simulação, podemos aplicar um leve blur vertical
        
        let mut temp_buffer = self.frame_buffer.clone();
        
        for y in 1..self.frame_buffer.height - 1 {
            for x in 0..self.frame_buffer.width {
                if let Some(color1) = self.frame_buffer.get_pixel(x, y - 1) {
                    if let Some(color2) = self.frame_buffer.get_pixel(x, y + 1) {
                        // Misturar com linhas adjacentes
                        let r1 = (color1 >> 16) & 0xFF;
                        let g1 = (color1 >> 8) & 0xFF;
                        let b1 = color1 & 0xFF;
                        
                        let r2 = (color2 >> 16) & 0xFF;
                        let g2 = (color2 >> 8) & 0xFF;
                        let b2 = color2 & 0xFF;
                        
                        let r_avg = (r1 + r2) / 2;
                        let g_avg = (g1 + g2) / 2;
                        let b_avg = (b1 + b2) / 2;
                        
                        let alpha = (color1 >> 24) & 0xFF; // Manter alpha original
                        let new_color = (alpha << 24) | (r_avg << 16) | (g_avg << 8) | b_avg;
                        
                        temp_buffer.set_pixel(x, y, new_color);
                    }
                }
            }
        }
        
        self.frame_buffer = temp_buffer;
    }
    
    /// Renderiza overlay de debug
    fn render_debug_overlay(
        &mut self,
        regs: &VdpRegisters,
        plane_manager: &PlaneManager,
        sprite_table: &SpriteTable,
    ) {
        let width = self.frame_buffer.width;
        let height = self.frame_buffer.height;
        
        // Desenhar borda da tela
        let border_color = 0x80FF0000; // Vermelho semi-transparente
        self.frame_buffer.draw_rect(0, 0, width, height, border_color);
        
        // Desenhar grade (células de 8x8 pixels)
        let grid_color = 0x4000FF00; // Verde semi-transparente
        for x in (0..width).step_by(8) {
            self.frame_buffer.draw_vertical_line(x, 0, height - 1, grid_color);
        }
        for y in (0..height).step_by(8) {
            self.frame_buffer.draw_horizontal_line(0, width - 1, y, grid_color);
        }
        
        // Desenhar informações de debug
        self.render_debug_text(regs, plane_manager, sprite_table);
    }
    
    /// Renderiza texto de debug
    fn render_debug_text(
        &mut self,
        regs: &VdpRegisters,
        plane_manager: &PlaneManager,
        sprite_table: &SpriteTable,
    ) {
        // Esta é uma implementação simplificada de renderização de texto
        // Em um emulador real, você teria uma fonte bitmap
        
        let info_lines = vec![
            format!("Mode: {}", self.video_mode.name),
            format!("Res: {}x{}", self.video_mode.visible_width, self.video_mode.visible_height),
            format!("Sprites: {}/{}", sprite_table.sprite_count, sprite_table.sprites.len()),
            format!("Display: {}", if self.video_mode.display_enabled() { "ON" } else { "OFF" }),
            format!("Frames: {}", self.frames_rendered),
        ];
        
        // Desenhar fundo para texto
        let bg_color = 0xC0000000; // Preto semi-transparente
        self.frame_buffer.fill_rect(5, 5, 200, (info_lines.len() * 12) as usize + 10, bg_color);
        
        // "Desenhar" texto (simplificado - apenas pontos)
        let text_color = 0xFFFFFFFF; // Branco
        for (i, line) in info_lines.iter().enumerate() {
            let y = 10 + i * 12;
            
            // Desenhar pontos para cada caractere (simulação)
            for (j, _) in line.chars().enumerate() {
                let x = 10 + j * 6;
                if x < self.frame_buffer.width && y < self.frame_buffer.height {
                    self.frame_buffer.set_pixel(x, y, text_color);
                    self.frame_buffer.set_pixel(x + 1, y, text_color);
                    self.frame_buffer.set_pixel(x, y + 1, text_color);
                    self.frame_buffer.set_pixel(x + 1, y + 1, text_color);
                }
            }
        }
    }
    
    /// Obtém uma cor da CRAM e converte para ARGB
    fn get_color_from_cram(&self, cram: &Cram, color_index: usize) -> u32 {
        let color_9bit = cram.read(color_index % 64);
        
        // Extrair componentes (3 bits cada)
        let r = ((color_9bit >> 0) & 0x07) as u32;
        let g = ((color_9bit >> 4) & 0x07) as u32;
        let b = ((color_9bit >> 8) & 0x07) as u32;
        
        // Converter para 8 bits por canal
        let r8 = (r * 36) as u32;  // 36 ≈ 255/7
        let g8 = (g * 36) as u32;
        let b8 = (b * 36) as u32;
        
        // Alpha máximo (opaco)
        (0xFF << 24) | (r8 << 16) | (g8 << 8) | b8
    }
    
    /// Configura quais elementos renderizar
    pub fn set_render_flags(&mut self, flags: RenderFlags) {
        self.show_background = flags.show_background;
        self.show_planes = flags.show_planes;
        self.show_window = flags.show_window;
        self.show_sprites = flags.show_sprites;
        self.debug_overlay = flags.debug_overlay;
    }
    
    /// Habilita/desabilita renderização
    pub fn set_render_enabled(&mut self, enabled: bool) {
        self.render_enabled = enabled;
    }
    
    /// Retorna o framebuffer atual
    pub fn get_frame_buffer(&self) -> &FrameBuffer {
        &self.frame_buffer
    }
    
    /// Retorna uma cópia do framebuffer
    pub fn copy_frame_buffer(&self) -> FrameBuffer {
        self.frame_buffer.clone()
    }
    
    /// Retorna estatísticas de renderização
    pub fn get_stats(&self) -> RenderStats {
        RenderStats {
            frames_rendered: self.frames_rendered,
            frame_time: self.last_frame_time,
            resolution: (self.frame_buffer.width, self.frame_buffer.height),
            video_mode: self.video_mode.clone(),
        }
    }
    
    /// Redimensiona o renderer (para mudança de modo de vídeo)
    pub fn resize(&mut self, width: usize, height: usize) {
        self.frame_buffer.resize(width, height);
    }
}

/// Flags de controle de renderização
#[derive(Debug, Clone, Copy)]
pub struct RenderFlags {
    pub show_background: bool,
    pub show_planes: bool,
    pub show_window: bool,
    pub show_sprites: bool,
    pub debug_overlay: bool,
}

impl Default for RenderFlags {
    fn default() -> Self {
        Self {
            show_background: true,
            show_planes: true,
            show_window: true,
            show_sprites: true,
            debug_overlay: false,
        }
    }
}

/// Estatísticas de renderização
#[derive(Debug, Clone)]
pub struct RenderStats {
    pub frames_rendered: u64,
    pub frame_time: f64,
    pub resolution: (usize, usize),
    pub video_mode: VdpVideoMode,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdp::modes::VdpVideoMode;
    use crate::vdp::registers::VdpRegisters;
    use crate::vdp::sprite::{Sprite, SpriteSize, SpriteTable};
    
    fn create_test_components() -> (VdpRegisters, Cram, Vram, Vsram, SpriteTable) {
        let regs = VdpRegisters::new();
        let cram = Cram::new();
        let vram = Vram::new();
        let vsram = Vsram::new();
        let sprite_table = SpriteTable::new(0xF800, false);
        
        (regs, cram, vram, vsram, sprite_table)
    }
    
    #[test]
    fn test_renderer_creation() {
        let video_mode = VdpVideoMode::new_default();
        let renderer = VdpRenderer::new(video_mode.clone());
        
        assert_eq!(renderer.video_mode.name, video_mode.name);
        assert_eq!(renderer.frame_buffer.width, video_mode.visible_width as usize);
        assert_eq!(renderer.frame_buffer.height, video_mode.visible_height as usize);
        assert!(renderer.render_enabled);
        assert!(renderer.show_background);
        assert!(renderer.show_planes);
        assert!(renderer.show_window);
        assert!(renderer.show_sprites);
        assert!(!renderer.debug_overlay);
        assert_eq!(renderer.frames_rendered, 0);
    }
    
    #[test]
    fn test_render_frame() {
        let video_mode = VdpVideoMode::new_default();
        let mut renderer = VdpRenderer::new(video_mode);
        
        let (regs, cram, vram, vsram, sprite_table) = create_test_components();
        
        // Renderizar um frame
        let frame_buffer = renderer.render_frame(&regs, &cram, &vram, &vsram, &sprite_table, 0.0);
        
        // Verificar que um frame foi renderizado
        assert_eq!(renderer.frames_rendered, 1);
        assert_eq!(frame_buffer.width, 320);
        assert_eq!(frame_buffer.height, 224);
        
        // Verificar que o framebuffer não está vazio (tem pelo menos a cor de fundo)
        let has_pixels = frame_buffer.pixels().iter().any(|&c| c != 0);
        assert!(has_pixels);
    }
    
    #[test]
    fn test_render_flags() {
        let video_mode = VdpVideoMode::new_default();
        let mut renderer = VdpRenderer::new(video_mode);
        
        // Configurar flags
        let flags = RenderFlags {
            show_background: false,
            show_planes: false,
            show_window: false,
            show_sprites: false,
            debug_overlay: true,
        };
        
        renderer.set_render_flags(flags);
        
        assert!(!renderer.show_background);
        assert!(!renderer.show_planes);
        assert!(!renderer.show_window);
        assert!(!renderer.show_sprites);
        assert!(renderer.debug_overlay);
    }
    
    #[test]
    fn test_renderer_stats() {
        let video_mode = VdpVideoMode::new_default();
        let mut renderer = VdpRenderer::new(video_mode.clone());
        
        let (regs, cram, vram, vsram, sprite_table) = create_test_components();
        
        // Renderizar alguns frames
        for i in 0..3 {
            renderer.render_frame(&regs, &cram, &vram, &vsram, &sprite_table, i as f64);
        }
        
        let stats = renderer.get_stats();
        
        assert_eq!(stats.frames_rendered, 3);
        assert_eq!(stats.frame_time, 2.0); // Último timestamp
        assert_eq!(stats.resolution, (320, 224));
        assert_eq!(stats.video_mode.name, video_mode.name);
    }
    
    #[test]
    fn test_renderer_resize() {
        let video_mode = VdpVideoMode::new_default();
        let mut renderer = VdpRenderer::new(video_mode);
        
        // Redimensionar
        renderer.resize(640, 480);
        
        assert_eq!(renderer.frame_buffer.width, 640);
        assert_eq!(renderer.frame_buffer.height, 480);
    }
    
    #[test]
    fn test_merge_framebuffers() {
        let video_mode = VdpVideoMode::new_default();
        let mut renderer = VdpRenderer::new(video_mode);
        
        // Criar framebuffers de teste
        let mut src = FrameBuffer::new(320, 224);
        let dst = FrameBuffer::new(320, 224);
        
        // Preencher src com alguns pixels
        src.set_pixel(10, 10, 0xFFFFFFFF); // Pixel opaco branco
        src.set_pixel(20, 20, 0x80FF0000); // Pixel semi-transparente vermelho
        
        // Copiar dst para renderer
        renderer.frame_buffer = dst;
        
        // Mesclar
        renderer.merge_framebuffers(&src);
        
        // Verificar que os pixels foram mesclados
        assert_eq!(renderer.frame_buffer.get_pixel(10, 10), Some(0xFFFFFFFF));
        assert_eq!(renderer.frame_buffer.get_pixel(20, 20), Some(0x80FF0000));
        
        // Pixel transparente não deve afetar
        assert_eq!(renderer.frame_buffer.get_pixel(30, 30), None);
    }
}