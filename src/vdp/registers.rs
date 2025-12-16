//! Registradores do VDP (Mega Drive / Sega Genesis)
//!
//! Implementa todos os registradores de controle e status do VDP:
//! - 24 registradores de controle (R0-R23) de 8 bits cada
//! - Registrador de status de 8 bits
//! - Buffer de dados e controle de endereço
//! - Suporte a escrita indireta (código de operação)
//!
//! Referência: https://wiki.megadrive.org/index.php?title=VDP_Registers

use bitflags::bitflags;

bitflags! {
    /// Registrador de status do VDP
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct VdpStatus: u8 {
        /// VBlank em progresso
        const VBLANK         = 0b10000000;
        /// HBlank em progresso
        const HBLANK         = 0b01000000;
        /// DMA em progresso
        const DMA_ACTIVE     = 0b00100000;
        /// FIFO cheio
        const FIFO_FULL      = 0b00010000;
        /// FIFO vazio
        const FIFO_EMPTY     = 0b00001000;
        /// Interrupção de linha programável ativa
        const LINE_IRQ       = 0b00000100;
        /// Sprite overflow na linha anterior
        const SPRITE_OVERFLOW = 0b00000010;
        /// Colisão de sprites detectada
        const SPRITE_COLLISION = 0b00000001;
    }
}

/// Modo de DMA do VDP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaMode {
    Desativado = 0,
    MemoriaParaVdp = 1,
    PreenchimentoVram = 2,
    CopiaVram = 3,
}

/// Tipo de acesso à memória do VDP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VdpAccessType {
    VRAM,
    CRAM,
    VSRAM,
}

/// Endereçamento do VDP com código de operação
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VdpAddress {
    pub addr: u16,           // Endereço de 14 bits (VRAM) ou 6 bits (CRAM/VSRAM)
    pub access_type: VdpAccessType,
    pub read_mode: bool,     // true = leitura, false = escrita
}

/// Estrutura completa de registradores do VDP
#[derive(Clone)]
pub struct VdpRegisters {
    // Registradores de controle (R0-R23)
    pub regs: [u8; 24],
    
    // Registrador de status
    pub status: VdpStatus,
    
    // Buffer de dados do VDP (para escrita/leitura)
    pub data_buffer: u16,
    pub address_buffer: u16,
    
    // Controle de endereçamento
    pub current_address: VdpAddress,
    pub code_buffer: u8,            // Buffer do código de operação
    pub address_latch: bool,        // Latch para segundo byte do endereço
    
    // DMA
    pub dma_source: u32,            // Endereço fonte do DMA (23 bits)
    pub dma_length: u16,            // Comprimento do DMA em palavras
    pub dma_mode: DmaMode,
    pub dma_active: bool,
    
    // FIFO de comandos
    pub fifo: Vec<u16>,             // FIFO para comandos do VDP
    pub fifo_capacity: usize,       // Capacidade máxima do FIFO
    
    // Auto-incremento
    pub auto_increment: u8,         // Valor de auto-incremento (R15)
    
    // Cache de valores calculados
    pub plane_a_addr: u16,
    pub plane_b_addr: u16,
    pub window_addr: u16,
    pub sprite_table_addr: u16,
    pub hscroll_addr: u16,
    
    // Controle de interrupções
    pub hblank_interrupt_enabled: bool,
    pub vblank_interrupt_enabled: bool,
    pub line_interrupt_enabled: bool,
    
    // Modo de vídeo
    pub mode_40_cell: bool,         // H40 (320 pixels) vs H32 (256 pixels)
    pub interlace_mode: u8,         // 0=off, 1=interlace, 2=interlace+30Hz
    pub shadow_highlight: bool,     // Modo shadow/highlight (12-bit)
    
    // Debug
    pub write_count: u64,
    pub read_count: u64,
}

impl VdpRegisters {
    /// Cria novos registradores do VDP com valores padrão
    pub fn new() -> Self {
        let mut regs = Self {
            regs: [0; 24],
            status: VdpStatus::empty(),
            data_buffer: 0,
            address_buffer: 0,
            current_address: VdpAddress {
                addr: 0,
                access_type: VdpAccessType::VRAM,
                read_mode: false,
            },
            code_buffer: 0,
            address_latch: false,
            dma_source: 0,
            dma_length: 0,
            dma_mode: DmaMode::Desativado,
            dma_active: false,
            fifo: Vec::new(),
            fifo_capacity: 4,        // FIFO do VDP tem 4 slots
            auto_increment: 2,       // Valor padrão de auto-incremento
            plane_a_addr: 0xC000,    // Valores padrão do VDP
            plane_b_addr: 0xE000,
            window_addr: 0xB000,
            sprite_table_addr: 0xF800,
            hscroll_addr: 0xFC00,
            hblank_interrupt_enabled: false,
            vblank_interrupt_enabled: false,
            line_interrupt_enabled: false,
            mode_40_cell: false,
            interlace_mode: 0,
            shadow_highlight: false,
            write_count: 0,
            read_count: 0,
        };
        
        // Inicializar alguns registradores com valores padrão
        regs.regs[15] = 2;  // Auto-incremento padrão
        
        regs
    }
    
    // =====================================================
    // ACESSO A REGISTRADORES
    // =====================================================
    
    /// Escreve em um registrador específico (R0-R23)
    pub fn write_reg(&mut self, index: usize, value: u8) {
        if index >= 24 {
            return;
        }
        
        self.write_count += 1;
        
        // Armazenar valor antigo
        let old_value = self.regs[index];
        self.regs[index] = value;
        
        // Processar mudanças específicas do registrador
        self.process_register_change(index, old_value, value);
    }
    
    /// Lê um registrador específico (R0-R23)
    pub fn read_reg(&self, index: usize) -> u8 {
        self.regs[index % 24]
    }
    
    /// Processa mudanças em registradores específicos
    fn process_register_change(&mut self, index: usize, old_value: u8, new_value: u8) {
        if old_value == new_value {
            return;
        }
        
        match index {
            0 => {
                // R0: Miscellaneous
                self.hblank_interrupt_enabled = (new_value & 0x10) != 0;
                self.line_interrupt_enabled = (new_value & 0x08) != 0;
            }
            1 => {
                // R1: Display Control
                self.vblank_interrupt_enabled = (new_value & 0x20) != 0;
                self.dma_mode = if (new_value & 0x10) != 0 {
                    DmaMode::MemoriaParaVdp
                } else {
                    DmaMode::Desativado
                };
                
                // Atualizar modo DMA baseado nos bits 6-7
                let dma_bits = (new_value >> 6) & 0x03;
                self.dma_mode = match dma_bits {
                    0 => DmaMode::Desativado,
                    1 => DmaMode::PreenchimentoVram,
                    2 => DmaMode::CopiaVram,
                    3 => DmaMode::MemoriaParaVdp,
                    _ => DmaMode::Desativado,
                };
            }
            10 => {
                // R10: Line interrupt counter
                // Nada especial a processar
            }
            12 => {
                // R12: Mode Set 3
                self.mode_40_cell = (new_value & 0x01) != 0;
                self.interlace_mode = (new_value >> 1) & 0x03;
                self.shadow_highlight = (new_value & 0x08) != 0;
            }
            13 => {
                // R13: HScroll data
                // Nada especial a processar
            }
            14 => {
                // R14: Nametable pattern base address
                self.update_plane_addresses();
            }
            15 => {
                // R15: Auto-increment value
                self.auto_increment = new_value;
            }
            16 => {
                // R16: Scroll size
                self.update_plane_addresses();
            }
            17 => {
                // R17: Window plane horizontal position
                // Nada especial a processar
            }
            18 => {
                // R18: Window plane vertical position
                // Nada especial a processar
            }
            19 => {
                // R19: DMA length low
                self.dma_length = (self.dma_length & 0xFF00) | (new_value as u16);
            }
            20 => {
                // R20: DMA length high
                self.dma_length = (self.dma_length & 0x00FF) | ((new_value as u16) << 8);
            }
            21 => {
                // R21: DMA source low
                self.dma_source = (self.dma_source & 0xFFFF00) | (new_value as u32);
            }
            22 => {
                // R22: DMA source mid
                self.dma_source = (self.dma_source & 0xFF00FF) | ((new_value as u32) << 8);
            }
            23 => {
                // R23: DMA source high & mode
                self.dma_source = (self.dma_source & 0x00FFFF) | (((new_value & 0x7F) as u32) << 16);
                
                // Bits 6-7: DMA mode
                let dma_mode_bits = (new_value >> 6) & 0x03;
                self.dma_mode = match dma_mode_bits {
                    0 => DmaMode::MemoriaParaVdp,
                    1 => DmaMode::PreenchimentoVram,
                    2 => DmaMode::CopiaVram,
                    _ => DmaMode::Desativado,
                };
            }
            _ => {
                // Outros registradores não têm processamento especial
            }
        }
    }
    
    /// Atualiza endereços dos planos baseado nos registradores
    fn update_plane_addresses(&mut self) {
        // Plano A: R2 (bits 3-5) * 0x400
        self.plane_a_addr = ((self.regs[2] & 0x38) as u16) << 10;
        
        // Plano B: R4 (bits 0-2) * 0x2000
        self.plane_b_addr = ((self.regs[4] & 0x07) as u16) << 13;
        
        // Window: R3 (bits 0-1, 3-5) * 0x400
        let window_bits = ((self.regs[3] & 0x30) >> 4) | ((self.regs[3] & 0x06) << 1);
        self.window_addr = (window_bits as u16) << 10;
        
        // Sprite table: R5 (bits 0-6) * 0x200
        self.sprite_table_addr = ((self.regs[5] & 0x7F) as u16) << 9;
        
        // HScroll table: R13 (bits 0-3) * 0x400
        self.hscroll_addr = ((self.regs[13] & 0x0F) as u16) << 10;
    }
    
    // =====================================================
    // BUFFER DE DADOS E ENDEREÇAMENTO
    // =====================================================
    
    /// Processa escrita na porta de dados do VDP (0xC00000/0xC00002)
    pub fn write_data_port(&mut self, value: u16) {
        self.write_count += 1;
        self.data_buffer = value;
        
        // Se DMA está ativo, tratar DMA
        if self.dma_active {
            self.process_dma_write(value);
            return;
        }
        
        // Adicionar ao FIFO se não estiver cheio
        if self.fifo.len() < self.fifo_capacity {
            self.fifo.push(value);
        } else {
            // FIFO cheio - setar flag
            self.status.insert(VdpStatus::FIFO_FULL);
        }
        
        // Atualizar status do FIFO
        self.update_fifo_status();
    }
    
    /// Processa leitura da porta de dados do VDP
    pub fn read_data_port(&mut self) -> u16 {
        self.read_count += 1;
        
        // Se em modo de leitura, retornar dado do endereço atual
        if self.current_address.read_mode {
            // Lógica de leitura da memória (VRAM/CRAM/VSRAM)
            // Retorna data_buffer e incrementa endereço
            let result = self.data_buffer;
            self.increment_address();
            result
        } else {
            // Retornar buffer de dados atual
            self.data_buffer
        }
    }
    
    /// Processa escrita na porta de controle do VDP (0xC00004/0xC00006)
    pub fn write_control_port(&mut self, value: u16) {
        self.write_count += 1;
        
        if self.address_latch {
            // Segundo byte - completar endereço/código
            self.address_buffer = (self.address_buffer & 0xFF00) | (value & 0xFF) as u16;
            self.process_control_word();
            self.address_latch = false;
        } else {
            // Primeiro byte - bits altos
            self.address_buffer = ((value & 0xFF) as u16) << 8;
            self.code_buffer = (value >> 8) as u8;
            self.address_latch = true;
        }
    }
    
    /// Processa leitura da porta de controle do VDP (retorna status)
    pub fn read_control_port(&mut self) -> u8 {
        self.read_count += 1;
        
        // Lê registrador de status
        let status = self.status.bits();
        
        // Limpar algumas flags após leitura
        self.status.remove(VdpStatus::VBLANK | VdpStatus::HBLANK | VdpStatus::LINE_IRQ);
        
        status
    }
    
    /// Processa palavra de controle completa
    fn process_control_word(&mut self) {
        let code = self.code_buffer;
        let addr = self.address_buffer;
        
        // Decodificar código de operação
        let access_type = match (code >> 6) & 0x03 {
            0 => VdpAccessType::VRAM,
            1 => VdpAccessType::CRAM,
            2 => VdpAccessType::VSRAM,
            _ => VdpAccessType::VRAM, // Inválido, fallback para VRAM
        };
        
        let read_mode = (code & 0x40) != 0;
        
        // Configurar endereço atual
        self.current_address = VdpAddress {
            addr: addr & 0x3FFF, // 14 bits para VRAM, menos para CRAM/VSRAM
            access_type,
            read_mode,
        };
        
        // Se for escrita e não leitura, preparar para escrita
        if !read_mode {
            self.prepare_for_write();
        }
    }
    
    /// Prepara para escrita após configuração de endereço
    fn prepare_for_write(&mut self) {
        // Resetar buffer de dados
        self.data_buffer = 0;
        
        // Se DMA está habilitado, ativar DMA
        if self.dma_mode != DmaMode::Desativado {
            self.dma_active = true;
            self.status.insert(VdpStatus::DMA_ACTIVE);
        }
    }
    
    /// Incrementa endereço atual baseado no valor de auto-incremento
    fn increment_address(&mut self) {
        match self.current_address.access_type {
            VdpAccessType::VRAM => {
                self.current_address.addr = self.current_address.addr.wrapping_add(self.auto_increment as u16);
                // VRAM tem 14 bits de endereço
                self.current_address.addr &= 0x3FFF;
            }
            VdpAccessType::CRAM => {
                self.current_address.addr = self.current_address.addr.wrapping_add(self.auto_increment as u16);
                // CRAM tem 64 entradas (6 bits)
                self.current_address.addr &= 0x3F;
            }
            VdpAccessType::VSRAM => {
                self.current_address.addr = self.current_address.addr.wrapping_add(self.auto_increment as u16);
                // VSRAM tem 40 entradas (6 bits)
                self.current_address.addr &= 0x3F;
            }
        }
    }
    
    // =====================================================
    // DMA
    // =====================================================
    
    /// Processa escrita de dados durante DMA
    fn process_dma_write(&mut self, value: u16) {
        // Processar transferência DMA baseada no modo
        match self.dma_mode {
            DmaMode::MemoriaParaVdp => {
                // Transferir da memória para VDP
                // (Esta é uma simplificação - na prática precisaria acessar o barramento)
                self.data_buffer = value;
                
                // Decrementar contador
                self.dma_length = self.dma_length.wrapping_sub(1);
                
                // Incrementar endereço fonte
                self.dma_source = self.dma_source.wrapping_add(2);
            }
            DmaMode::PreenchimentoVram => {
                // Preenchimento de VRAM com valor fixo
                self.data_buffer = value;
                self.dma_length = self.dma_length.wrapping_sub(1);
            }
            DmaMode::CopiaVram => {
                // Cópia dentro da VRAM
                self.data_buffer = value;
                self.dma_length = self.dma_length.wrapping_sub(1);
            }
            DmaMode::Desativado => {
                // DMA não ativo
                return;
            }
        }
        
        // Verificar se DMA terminou
        if self.dma_length == 0 {
            self.dma_active = false;
            self.status.remove(VdpStatus::DMA_ACTIVE);
            self.dma_mode = DmaMode::Desativado;
        }
    }
    
    /// Atualiza status do FIFO
    fn update_fifo_status(&mut self) {
        if self.fifo.is_empty() {
            self.status.insert(VdpStatus::FIFO_EMPTY);
            self.status.remove(VdpStatus::FIFO_FULL);
        } else if self.fifo.len() >= self.fifo_capacity {
            self.status.insert(VdpStatus::FIFO_FULL);
            self.status.remove(VdpStatus::FIFO_EMPTY);
        } else {
            self.status.remove(VdpStatus::FIFO_EMPTY | VdpStatus::FIFO_FULL);
        }
    }
    
    /// Obtém próximo comando do FIFO (se disponível)
    pub fn pop_fifo(&mut self) -> Option<u16> {
        let result = self.fifo.pop();
        self.update_fifo_status();
        result
    }
    
    /// Verifica se FIFO tem comandos pendentes
    pub fn has_fifo_commands(&self) -> bool {
        !self.fifo.is_empty()
    }
    
    // =====================================================
    // MÉTODOS DE CONSULTA E CONTROLE
    // =====================================================
    
    /// Retorna true se display está habilitado
    pub fn display_enabled(&self) -> bool {
        (self.regs[1] & 0x40) != 0
    }
    
    /// Retorna true se DMA está habilitado
    pub fn dma_enabled(&self) -> bool {
        self.dma_mode != DmaMode::Desativado
    }
    
    /// Retorna true se modo H40 (320 pixels) está ativo
    pub fn mode_40_cell(&self) -> bool {
        self.mode_40_cell
    }
    
    /// Retorna true se modo H32 (256 pixels) está ativo
    pub fn mode_32_cell(&self) -> bool {
        !self.mode_40_cell
    }
    
    /// Retorna true se modo shadow/highlight está ativo
    pub fn shadow_highlight_enabled(&self) -> bool {
        self.shadow_highlight
    }
    
    /// Retorna true se modo interlace está ativo
    pub fn interlace_enabled(&self) -> bool {
        self.interlace_mode != 0
    }
    
    /// Retorna modo de interlace (0=off, 1=interlace, 2=interlace+30Hz)
    pub fn interlace_mode(&self) -> u8 {
        self.interlace_mode
    }
    
    /// Retorna endereço do plano A
    pub fn plane_a_address(&self) -> u16 {
        self.plane_a_addr
    }
    
    /// Retorna endereço do plano B
    pub fn plane_b_address(&self) -> u16 {
        self.plane_b_addr
    }
    
    /// Retorna endereço da janela
    pub fn window_address(&self) -> u16 {
        self.window_addr
    }
    
    /// Retorna endereço da tabela de sprites
    pub fn sprite_table_address(&self) -> u16 {
        self.sprite_table_addr
    }
    
    /// Retorna endereço da tabela de HScroll
    pub fn hscroll_address(&self) -> u16 {
        self.hscroll_addr
    }
    
    /// Retorna valor de auto-incremento
    pub fn auto_increment_value(&self) -> u8 {
        self.auto_increment
    }
    
    /// Retorna cor de fundo (índice 0 da CRAM)
    pub fn background_color_index(&self) -> u8 {
        self.regs[7] & 0x3F
    }
    
    /// Retorna contador de linha para interrupção
    pub fn line_interrupt_counter(&self) -> u8 {
        self.regs[10]
    }
    
    /// Retorna scroll horizontal do plano A
    pub fn hscroll_a(&self) -> u8 {
        self.regs[8]
    }
    
    /// Retorna scroll horizontal do plano B
    pub fn hscroll_b(&self) -> u8 {
        self.regs[9]
    }
    
    /// Retorna scroll vertical do plano A
    pub fn vscroll_a(&self) -> u8 {
        self.regs[11]
    }
    
    /// Retorna scroll vertical do plano B
    pub fn vscroll_b(&self) -> u8 {
        self.regs[19] // Nota: R19 também é usado para DMA
    }
    
    /// Retorna tamanho dos planos (em tiles)
    pub fn plane_size(&self) -> (u8, u8) {
        let size = self.regs[16];
        let width = match (size >> 4) & 0x03 {
            0 => 32,
            1 => 64,
            2 => 128,
            _ => 32, // Inválido, fallback para 32
        };
        let height = match size & 0x03 {
            0 => 32,
            1 => 64,
            2 => 128,
            _ => 32, // Inválido, fallback para 32
        };
        (width, height)
    }
    
    /// Retorna tamanho dos tiles (8x8 ou 8x8 sempre no MD)
    pub fn tile_size(&self) -> u8 {
        // Bit 0 do registrador 12 controla tamanho do sprite
        // Para tiles de plano, é sempre 8x8
        8
    }
    
    /// Retorna posição da janela (horizontal, vertical)
    pub fn window_position(&self) -> (i8, i8) {
        let h_pos = self.regs[17] as i8;
        let v_pos = self.regs[18] as i8;
        (h_pos, v_pos)
    }
    
    /// Atualiza flags de interrupção
    pub fn update_interrupt_flags(&mut self, vblank: bool, hblank: bool, line_irq: bool) {
        if vblank {
            self.status.insert(VdpStatus::VBLANK);
        } else {
            self.status.remove(VdpStatus::VBLANK);
        }
        
        if hblank {
            self.status.insert(VdpStatus::HBLANK);
        } else {
            self.status.remove(VdpStatus::HBLANK);
        }
        
        if line_irq {
            self.status.insert(VdpStatus::LINE_IRQ);
        } else {
            self.status.remove(VdpStatus::LINE_IRQ);
        }
    }
    
    /// Atualiza flags de sprite
    pub fn update_sprite_flags(&mut self, overflow: bool, collision: bool) {
        if overflow {
            self.status.insert(VdpStatus::SPRITE_OVERFLOW);
        } else {
            self.status.remove(VdpStatus::SPRITE_OVERFLOW);
        }
        
        if collision {
            self.status.insert(VdpStatus::SPRITE_COLLISION);
        } else {
            self.status.remove(VdpStatus::SPRITE_COLLISION);
        }
    }
    
    /// Reseta todos os registradores para valores padrão
    pub fn reset(&mut self) {
        self.regs.fill(0);
        self.status = VdpStatus::empty();
        self.data_buffer = 0;
        self.address_buffer = 0;
        self.current_address = VdpAddress {
            addr: 0,
            access_type: VdpAccessType::VRAM,
            read_mode: false,
        };
        self.code_buffer = 0;
        self.address_latch = false;
        self.dma_source = 0;
        self.dma_length = 0;
        self.dma_mode = DmaMode::Desativado;
        self.dma_active = false;
        self.fifo.clear();
        self.auto_increment = 2;
        self.update_plane_addresses();
        self.hblank_interrupt_enabled = false;
        self.vblank_interrupt_enabled = false;
        self.line_interrupt_enabled = false;
        self.mode_40_cell = false;
        self.interlace_mode = 0;
        self.shadow_highlight = false;
        self.write_count = 0;
        self.read_count = 0;
        
        // Restaurar alguns valores padrão
        self.regs[15] = 2;  // Auto-incremento padrão
    }
    
    /// Retorna informações de debug sobre os registradores
    pub fn debug_info(&self) -> Vec<String> {
        let mut info = Vec::new();
        
        info.push(format!("Status: {:08b}", self.status.bits()));
        info.push(format!("Display: {}", if self.display_enabled() { "ON" } else { "OFF" }));
        info.push(format!("Mode: {}", if self.mode_40_cell { "H40" } else { "H32" }));
        info.push(format!("Interlace: {}", self.interlace_mode));
        info.push(format!("Shadow/Highlight: {}", self.shadow_highlight));
        info.push(format!("DMA Mode: {:?}", self.dma_mode));
        info.push(format!("DMA Active: {}", self.dma_active));
        info.push(format!("FIFO: {}/{}", self.fifo.len(), self.fifo_capacity));
        info.push(format!("Auto-increment: {}", self.auto_increment));
        info.push(format!("Plane A Addr: 0x{:04X}", self.plane_a_addr));
        info.push(format!("Plane B Addr: 0x{:04X}", self.plane_b_addr));
        info.push(format!("Window Addr: 0x{:04X}", self.window_addr));
        info.push(format!("Sprite Table Addr: 0x{:04X}", self.sprite_table_addr));
        info.push(format!("HScroll Addr: 0x{:04X}", self.hscroll_addr));
        info.push(format!("Write Count: {}", self.write_count));
        info.push(format!("Read Count: {}", self.read_count));
        
        info
    }
    
    /// Dump de todos os registradores
    pub fn dump_registers(&self) -> String {
        let mut dump = String::new();
        
        for i in 0..24 {
            dump.push_str(&format!("R{:02}: 0x{:02X} ", i, self.regs[i]));
            if (i + 1) % 4 == 0 {
                dump.push('\n');
            }
        }
        
        dump
    }
}

impl Default for VdpRegisters {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_register_creation() {
        let regs = VdpRegisters::new();
        
        assert_eq!(regs.regs.len(), 24);
        assert_eq!(regs.auto_increment, 2);
        assert!(!regs.display_enabled());
        assert!(!regs.dma_enabled());
        assert!(!regs.mode_40_cell());
        assert_eq!(regs.write_count, 0);
        assert_eq!(regs.read_count, 0);
    }
    
    #[test]
    fn test_register_write_read() {
        let mut regs = VdpRegisters::new();
        
        // Escrever em registradores
        regs.write_reg(0, 0x12);
        regs.write_reg(1, 0x34);
        regs.write_reg(15, 0x04);
        
        // Ler registradores
        assert_eq!(regs.read_reg(0), 0x12);
        assert_eq!(regs.read_reg(1), 0x34);
        assert_eq!(regs.read_reg(15), 0x04);
        
        // Verificar processamento
        assert_eq!(regs.auto_increment, 0x04);
    }
    
    #[test]
    fn test_display_enable() {
        let mut regs = VdpRegisters::new();
        
        // Habilitar display (bit 6 do R1)
        regs.write_reg(1, 0x40);
        assert!(regs.display_enabled());
        
        // Desabilitar display
        regs.write_reg(1, 0x00);
        assert!(!regs.display_enabled());
    }
    
    #[test]
    fn test_mode_40_cell() {
        let mut regs = VdpRegisters::new();
        
        // Habilitar modo H40 (bit 0 do R12)
        regs.write_reg(12, 0x01);
        assert!(regs.mode_40_cell());
        assert!(!regs.mode_32_cell());
        
        // Desabilitar modo H40
        regs.write_reg(12, 0x00);
        assert!(!regs.mode_40_cell());
        assert!(regs.mode_32_cell());
    }
    
    #[test]
    fn test_dma_mode() {
        let mut regs = VdpRegisters::new();
        
        // Testar diferentes modos DMA
        regs.write_reg(1, 0x10); // Habilitar DMA
        assert_eq!(regs.dma_mode, DmaMode::MemoriaParaVdp);
        assert!(regs.dma_enabled());
        
        // Configurar modo preenchimento (bits 6-7 do R23)
        regs.write_reg(23, 0x40); // Bits 6-7 = 01
        assert_eq!(regs.dma_mode, DmaMode::PreenchimentoVram);
        
        // Configurar modo cópia (bits 6-7 do R23)
        regs.write_reg(23, 0x80); // Bits 6-7 = 10
        assert_eq!(regs.dma_mode, DmaMode::CopiaVram);
    }
    
    #[test]
    fn test_plane_address_calculation() {
        let mut regs = VdpRegisters::new();
        
        // Configurar endereços dos planos
        regs.write_reg(2, 0x38);  // Plano A: 0x38 -> 0xE000
        regs.write_reg(4, 0x07);  // Plano B: 0x07 -> 0xE000
        regs.write_reg(3, 0x36);  // Window: bits específicos
        regs.write_reg(5, 0x7F);  // Sprite table: 0x7F -> 0xFE00
        regs.write_reg(13, 0x0F); // HScroll: 0x0F -> 0x3C00
        
        // Verificar cálculos
        assert_eq!(regs.plane_a_address(), 0xE000);
        assert_eq!(regs.plane_b_address(), 0xE000);
        assert_eq!(regs.sprite_table_address(), 0xFE00);
        assert_eq!(regs.hscroll_address(), 0x3C00);
    }
    
    #[test]
    fn test_control_port_write() {
        let mut regs = VdpRegisters::new();
        
        // Primeira escrita (bits altos)
        regs.write_control_port(0x8200); // Código = 0x82, addr_high = 0x00
        assert!(regs.address_latch);
        assert_eq!(regs.code_buffer, 0x82);
        assert_eq!(regs.address_buffer, 0x0000);
        
        // Segunda escrita (bits baixos)
        regs.write_control_port(0x1234); // addr_low = 0x34
        assert!(!regs.address_latch);
        assert_eq!(regs.address_buffer, 0x0034);
        
        // Verificar endereço atual configurado
        assert_eq!(regs.current_address.addr, 0x0034);
        assert_eq!(regs.current_address.access_type, VdpAccessType::VRAM);
        assert!(!regs.current_address.read_mode); // Bit 6 = 0
    }
    
    #[test]
    fn test_data_port_write_read() {
        let mut regs = VdpRegisters::new();
        
        // Configurar endereço para escrita
        regs.write_control_port(0x4000); // Código para escrita VRAM
        regs.write_control_port(0x1234);
        
        // Escrever dados
        regs.write_data_port(0xABCD);
        
        // Verificar que dados foram armazenados
        assert_eq!(regs.data_buffer, 0xABCD);
        
        // Verificar FIFO
        assert!(regs.has_fifo_commands());
        assert_eq!(regs.fifo.len(), 1);
        
        // Ler dados (em modo escrita, retorna buffer)
        let data = regs.read_data_port();
        assert_eq!(data, 0xABCD);
    }
    
    #[test]
    fn test_fifo_operations() {
        let mut regs = VdpRegisters::new();
        
        // FIFO deve começar vazio
        assert!(!regs.has_fifo_commands());
        assert!(regs.status.contains(VdpStatus::FIFO_EMPTY));
        
        // Adicionar comandos ao FIFO
        regs.write_data_port(0x1111);
        regs.write_data_port(0x2222);
        regs.write_data_port(0x3333);
        regs.write_data_port(0x4444);
        
        // FIFO deve estar cheio (capacidade = 4)
        assert!(regs.status.contains(VdpStatus::FIFO_FULL));
        assert_eq!(regs.fifo.len(), 4);
        
        // Remover comandos
        assert_eq!(regs.pop_fifo(), Some(0x4444));
        assert_eq!(regs.pop_fifo(), Some(0x3333));
        assert_eq!(regs.pop_fifo(), Some(0x2222));
        assert_eq!(regs.pop_fifo(), Some(0x1111));
        
        // FIFO deve estar vazio novamente
        assert!(!regs.has_fifo_commands());
        assert!(regs.status.contains(VdpStatus::FIFO_EMPTY));
    }
    
    #[test]
    fn test_status_register() {
        let mut regs = VdpRegisters::new();
        
        // Verificar status inicial
        assert_eq!(regs.status.bits(), 0);
        
        // Atualizar flags
        regs.update_interrupt_flags(true, false, true);
        assert!(regs.status.contains(VdpStatus::VBLANK));
        assert!(!regs.status.contains(VdpStatus::HBLANK));
        assert!(regs.status.contains(VdpStatus::LINE_IRQ));
        
        // Atualizar flags de sprite
        regs.update_sprite_flags(true, true);
        assert!(regs.status.contains(VdpStatus::SPRITE_OVERFLOW));
        assert!(regs.status.contains(VdpStatus::SPRITE_COLLISION));
        
        // Ler status (deve limpar algumas flags)
        let status = regs.read_control_port();
        assert!(status & VdpStatus::VBLANK.bits() != 0);
        assert!(status & VdpStatus::LINE_IRQ.bits() != 0);
        
        // Verificar que flags foram limpas após leitura
        assert!(!regs.status.contains(VdpStatus::VBLANK));
        assert!(!regs.status.contains(VdpStatus::LINE_IRQ));
        // Flags de sprite não são limpas
        assert!(regs.status.contains(VdpStatus::SPRITE_OVERFLOW));
        assert!(regs.status.contains(VdpStatus::SPRITE_COLLISION));
    }
    
    #[test]
    fn test_reset() {
        let mut regs = VdpRegisters::new();
        
        // Modificar alguns valores
        regs.write_reg(1, 0x40);
        regs.write_reg(12, 0x01);
        regs.write_data_port(0x1234);
        regs.write_count = 100;
        regs.read_count = 50;
        
        // Resetar
        regs.reset();
        
        // Verificar valores padrão
        assert_eq!(regs.read_reg(1), 0);
        assert_eq!(regs.read_reg(12), 0);
        assert_eq!(regs.data_buffer, 0);
        assert_eq!(regs.write_count, 0);
        assert_eq!(regs.read_count, 0);
        assert_eq!(regs.auto_increment, 2);
        assert!(!regs.display_enabled());
        assert!(!regs.mode_40_cell());
    }
    
    #[test]
    fn test_debug_info() {
        let regs = VdpRegisters::new();
        let info = regs.debug_info();
        
        assert!(!info.is_empty());
        assert!(info[0].contains("Status:"));
        assert!(info[1].contains("Display:"));
        
        // Verificar dump de registradores
        let dump = regs.dump_registers();
        assert!(dump.contains("R00:"));
        assert!(dump.contains("R23:"));
    }
}