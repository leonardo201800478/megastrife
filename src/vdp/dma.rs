//! DMA (Direct Memory Access) do VDP
//!
//! Implementa os três modos de DMA do VDP do Mega Drive:
//! 1. DMA Memória -> VRAM/CRAM/VSRAM (modo 0)
//! 2. DMA de Preenchimento de VRAM (modo 1)
//! 3. DMA de Cópia de VRAM (modo 2)
//!
//! O DMA pode transferir dados da memória do 68K para a VRAM, CRAM ou VSRAM,
//! ou realizar operações internas na VRAM.

use crate::vdp::registers::VdpRegisters;
use crate::vdp::vram::Vram;
use crate::vdp::cram::Cram;
use crate::vdp::vsram::Vsram;
use crate::memory::bus::Bus;

/// Modos de operação do DMA
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaMode {
    /// Modo 0: DMA da memória do 68K para VRAM/CRAM/VSRAM
    MemoryToVdp,
    /// Modo 1: DMA de preenchimento de VRAM
    VramFill,
    /// Modo 2: DMA de cópia de VRAM
    VramCopy,
    /// Nenhum DMA ativo
    Inactive,
}

/// Controlador de DMA do VDP
/// Implementa transferências VRAM, CRAM, VSRAM
#[derive(Clone)]
pub struct VdpDma {
    pub mode: DmaMode,
    pub source_addr: u32,           // Endereço de origem (68K memory address)
    pub dest_addr: u32,             // Endereço de destino (VDP address space)
    pub length: u16,                // Comprimento em palavras (16-bit)
    pub words_remaining: u16,       // Palavras restantes a transferir
    pub active: bool,
    pub fill_data: u16,             // Dado para preenchimento (modo 1)
    pub copy_source: u32,           // Endereço de origem para cópia (modo 2)
    
    // Controle de ciclos/timing
    pub cycles_until_transfer: u32,
    pub words_per_cycle: u32,
}

impl VdpDma {
    /// Cria um novo controlador DMA
    pub fn new() -> Self {
        Self {
            mode: DmaMode::Inactive,
            source_addr: 0,
            dest_addr: 0,
            length: 0,
            words_remaining: 0,
            active: false,
            fill_data: 0,
            copy_source: 0,
            cycles_until_transfer: 0,
            words_per_cycle: 1,  // Padrão: 1 palavra por ciclo
        }
    }

    /// Configura o DMA com base nos registradores do VDP
    pub fn setup_from_registers(&mut self, regs: &VdpRegisters, vram_addr: u16) -> bool {
        if !regs.dma_enabled() {
            self.active = false;
            return false;
        }

        // Determinar modo de DMA
        let mode_bits = (regs.get(0x17) >> 6) & 0x03;
        self.mode = match mode_bits {
            0 => DmaMode::MemoryToVdp,
            1 => DmaMode::VramFill,
            2 => DmaMode::VramCopy,
            _ => DmaMode::Inactive,
        };

        if self.mode == DmaMode::Inactive {
            self.active = false;
            return false;
        }

        // Endereço de destino (do registrador de endereço do VDP)
        self.dest_addr = vram_addr as u32 & 0x3FFF;  // 14 bits para VRAM

        // Comprimento (em palavras)
        self.length = regs.dma_length;
        self.words_remaining = self.length;

        // Configuração específica por modo
        match self.mode {
            DmaMode::MemoryToVdp => {
                // Endereço de origem de 23 bits
                let source_low = regs.get(0x15) as u32;
                let source_high = regs.get(0x16) as u32;
                let source_bank = ((regs.get(0x17) & 0x80) >> 7) as u32;  // Bit 23
                self.source_addr = (source_bank << 16) | (source_high << 8) | source_low;
                
                // Alinhamento: deve ser múltiplo de 2
                self.source_addr &= !0x01;
            }
            DmaMode::VramFill => {
                // Dado de preenchimento vem do byte baixo do registrador 0x15
                self.fill_data = (regs.get(0x15) as u16) << 8 | (regs.get(0x15) as u16);
                
                // Endereço de origem não é usado neste modo
                self.source_addr = 0;
            }
            DmaMode::VramCopy => {
                // Endereço de origem de 17 bits (VRAM address)
                let source_low = regs.get(0x15) as u32;
                let source_high = regs.get(0x16) as u32;
                self.copy_source = ((source_high & 0x3F) << 8) | source_low;
                
                // Alinhamento: deve ser múltiplo de 2
                self.copy_source &= !0x01;
            }
            DmaMode::Inactive => {
                return false;
            }
        }

        // Configuração de timing
        self.cycles_until_transfer = 0;
        self.words_per_cycle = 1;  // Pode ser ajustado para emulação de timing preciso

        self.active = true;
        true
    }

    /// Executa um ciclo do DMA
    /// Retorna true se o DMA terminou
    pub fn tick(
        &mut self,
        bus: &mut Bus,
        vram: &mut Vram,
        cram: &mut Cram,
        vsram: &mut Vsram,
    ) -> bool {
        if !self.active || self.words_remaining == 0 {
            self.active = false;
            self.mode = DmaMode::Inactive;
            return true;
        }

        // Controle de timing (simplificado)
        if self.cycles_until_transfer > 0 {
            self.cycles_until_transfer -= 1;
            return false;
        }

        // Executa transferência
        let completed = self.execute_transfer(bus, vram, cram, vsram);
        
        // Configura próximo ciclo
        self.cycles_until_transfer = self.words_per_cycle.saturating_sub(1);
        
        completed
    }

    /// Executa uma transferência individual
    fn execute_transfer(
        &mut self,
        bus: &mut Bus,
        vram: &mut Vram,
        cram: &mut Cram,
        vsram: &mut Vsram,
    ) -> bool {
        // Determinar tipo de destino baseado no endereço de destino
        let dest_type = (self.dest_addr >> 14) & 0x03;
        let dest_addr_word = (self.dest_addr & 0x3FFF) >> 1;  // Endereço em palavras

        match self.mode {
            DmaMode::MemoryToVdp => {
                // Ler palavra da memória do 68K
                let data = bus.read16(self.source_addr);
                
                // Escrever no destino apropriado
                match dest_type {
                    0 => {  // VRAM
                        vram.write16(self.dest_addr, data);
                    }
                    1 => {  // CRAM
                        if dest_addr_word < 64 {
                            cram.write(dest_addr_word as usize, data & 0x0EEE);
                        }
                    }
                    2 => {  // VSRAM
                        if dest_addr_word < 40 {
                            vsram.write16(self.dest_addr, data);
                        }
                    }
                    _ => {  // VRAM (fallback)
                        vram.write16(self.dest_addr, data);
                    }
                }
                
                // Atualizar endereços
                self.source_addr = self.source_addr.wrapping_add(2);
            }
            
            DmaMode::VramFill => {
                // Apenas VRAM suportada para preenchimento
                if dest_type == 0 {
                    vram.write16(self.dest_addr, self.fill_data);
                }
                // Nota: O VDP preenche apenas VRAM, CRAM/VSRAM ignorados neste modo
            }
            
            DmaMode::VramCopy => {
                // Ler da VRAM fonte
                let data = vram.read16(self.copy_source);
                
                // Escrever na VRAM destino
                vram.write16(self.dest_addr, data);
                
                // Atualizar endereços
                self.copy_source = self.copy_source.wrapping_add(2);
            }
            
            DmaMode::Inactive => {
                return true;
            }
        }

        // Atualizar endereço de destino
        self.dest_addr = self.dest_addr.wrapping_add(2);
        
        // Decrementar contador
        self.words_remaining = self.words_remaining.saturating_sub(1);
        
        // Verificar conclusão
        if self.words_remaining == 0 {
            self.active = false;
            self.mode = DmaMode::Inactive;
            return true;
        }
        
        false
    }

    /// Retorna o número de palavras restantes
    pub fn words_remaining(&self) -> u16 {
        self.words_remaining
    }

    /// Retorna o comprimento total do DMA
    pub fn length(&self) -> u16 {
        self.length
    }

    /// Retorna o modo atual do DMA
    pub fn mode(&self) -> DmaMode {
        self.mode
    }

    /// Aborta o DMA ativo
    pub fn abort(&mut self) {
        self.active = false;
        self.mode = DmaMode::Inactive;
        self.words_remaining = 0;
    }

    /// Retorna se o DMA está ativo
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Retorna o endereço de origem atual
    pub fn current_source_addr(&self) -> u32 {
        match self.mode {
            DmaMode::MemoryToVdp => self.source_addr,
            DmaMode::VramCopy => self.copy_source,
            _ => 0,
        }
    }

    /// Retorna o endereço de destino atual
    pub fn current_dest_addr(&self) -> u32 {
        self.dest_addr
    }

    /// Calcula o tempo estimado para conclusão (em ciclos)
    pub fn estimated_cycles_remaining(&self) -> u32 {
        (self.words_remaining as u32) * self.words_per_cycle + self.cycles_until_transfer
    }
}

impl Default for VdpDma {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdp::registers::VdpRegisters;
    use crate::memory::bus::Bus;
    
    struct MockBus {
        memory: Vec<u8>,
    }
    
    impl MockBus {
        fn new() -> Self {
            Self {
                memory: vec![0; 0x100000],  // 1MB de memória
            }
        }
        
        fn write_memory(&mut self, addr: u32, data: &[u8]) {
            let start = addr as usize;
            let end = start + data.len();
            if end <= self.memory.len() {
                self.memory[start..end].copy_from_slice(data);
            }
        }
    }
    
    impl Bus {
        fn read16_dma(&mut self, _addr: u32) -> u16 {
            // Implementação mock para teste
            0x1234
        }
    }

    #[test]
    fn test_dma_setup_memory_to_vdp() {
        let mut regs = VdpRegisters::new();
        let mut dma = VdpDma::new();
        
        // Configurar registradores para DMA modo 0
        regs.set(0x15, 0x34);  // Source low
        regs.set(0x16, 0x12);  // Source high
        regs.set(0x17, 0x81);  // DMA enable + bit 23 + modo 0
        regs.set_dma_length(0x0100);  // 256 palavras
        
        let vram_addr = 0x0000;  // Destino VRAM
        
        let result = dma.setup_from_registers(&regs, vram_addr);
        assert!(result);
        assert_eq!(dma.mode, DmaMode::MemoryToVdp);
        assert_eq!(dma.source_addr, 0x1234);
        assert_eq!(dma.length, 0x0100);
        assert_eq!(dma.words_remaining, 0x0100);
        assert!(dma.active);
    }

    #[test]
    fn test_dma_setup_vram_fill() {
        let mut regs = VdpRegisters::new();
        let mut dma = VdpDma::new();
        
        // Configurar registradores para DMA modo 1
        regs.set(0x15, 0xAB);  // Fill data
        regs.set(0x17, 0xC0);  // DMA enable + modo 1
        regs.set_dma_length(0x0080);  // 128 palavras
        
        let vram_addr = 0x4000;  // Destino VRAM
        
        let result = dma.setup_from_registers(&regs, vram_addr);
        assert!(result);
        assert_eq!(dma.mode, DmaMode::VramFill);
        assert_eq!(dma.fill_data, 0xABAB);  // Replicado para 16 bits
        assert_eq!(dma.length, 0x0080);
        assert!(dma.active);
    }

    #[test]
    fn test_dma_setup_vram_copy() {
        let mut regs = VdpRegisters::new();
        let mut dma = VdpDma::new();
        
        // Configurar registradores para DMA modo 2
        regs.set(0x15, 0x34);  // Source low
        regs.set(0x16, 0x12);  // Source high (6 bits)
        regs.set(0x17, 0x40);  // DMA enable + modo 2
        regs.set_dma_length(0x0040);  // 64 palavras
        
        let vram_addr = 0x2000;  // Destino VRAM
        
        let result = dma.setup_from_registers(&regs, vram_addr);
        assert!(result);
        assert_eq!(dma.mode, DmaMode::VramCopy);
        assert_eq!(dma.copy_source, 0x1234);
        assert_eq!(dma.length, 0x0040);
        assert!(dma.active);
    }

    #[test]
    fn test_dma_execute_memory_to_vram() {
        let mut dma = VdpDma::new();
        let mut vram = Vram::new();
        let mut cram = Cram::new();
        let mut vsram = Vsram::new();
        let mut bus = Bus::new(
            std::sync::Arc::new(std::sync::Mutex::new(crate::memory::ram::Ram::new(0x10000))),
            std::sync::Arc::new(std::sync::Mutex::new(crate::vdp::vdp::Vdp::new())),
            std::sync::Arc::new(std::sync::Mutex::new(crate::io::io::Io::new())),
            std::sync::Arc::new(std::sync::Mutex::new(crate::memory::rom::Rom::new(Vec::new()))),
            std::sync::Arc::new(std::sync::Mutex::new(crate::sound::ym2612::Ym2612::new())),
            std::sync::Arc::new(std::sync::Mutex::new(crate::sound::psg::Psg::new())),
        );
        
        // Setup DMA
        dma.mode = DmaMode::MemoryToVdp;
        dma.source_addr = 0x1000;
        dma.dest_addr = 0x0000;  // VRAM
        dma.length = 4;
        dma.words_remaining = 4;
        dma.active = true;
        
        // Write test data to bus memory
        bus.write16(0x1000, 0x1234);
        bus.write16(0x1002, 0x5678);
        bus.write16(0x1004, 0x9ABC);
        bus.write16(0x1006, 0xDEF0);
        
        // Executar transferências
        for _ in 0..4 {
            dma.execute_transfer(&mut bus, &mut vram, &mut cram, &mut vsram);
        }
        
        assert_eq!(dma.words_remaining, 0);
        assert!(!dma.active);
    }

    #[test]
    fn test_dma_execute_vram_fill() {
        let mut dma = VdpDma::new();
        let mut vram = Vram::new();
        let mut cram = Cram::new();
        let mut vsram = Vsram::new();
        
        // Setup DMA
        dma.mode = DmaMode::VramFill;
        dma.dest_addr = 0x1000;  // VRAM
        dma.fill_data = 0xABCD;
        dma.length = 8;
        dma.words_remaining = 8;
        dma.active = true;
        
        // Mock bus (não usado neste modo)
        struct TestBus;
        impl TestBus {
            fn read16_dma(&mut self, _addr: u32) -> u16 { 0 }
        }
        
        let mut bus = TestBus;
        
        // Executar transferências
        dma.execute_transfer(&mut bus, &mut vram, &mut cram, &mut vsram);
        
        // Verificar que a VRAM foi preenchida
        assert_eq!(vram.read16(0x1000), 0xABCD);
        assert_eq!(dma.words_remaining, 7);
    }

    #[test]
    fn test_dma_abort() {
        let mut dma = VdpDma::new();
        
        dma.mode = DmaMode::MemoryToVdp;
        dma.active = true;
        dma.words_remaining = 100;
        
        dma.abort();
        
        assert!(!dma.active);
        assert_eq!(dma.mode, DmaMode::Inactive);
        assert_eq!(dma.words_remaining, 0);
    }

    #[test]
    fn test_dma_status() {
        let mut dma = VdpDma::new();
        
        dma.mode = DmaMode::VramCopy;
        dma.active = true;
        dma.length = 256;
        dma.words_remaining = 128;
        
        assert!(dma.is_active());
        assert_eq!(dma.mode(), DmaMode::VramCopy);
        assert_eq!(dma.length(), 256);
        assert_eq!(dma.words_remaining(), 128);
        assert_eq!(dma.estimated_cycles_remaining(), 128);  // 1 palavra por ciclo
    }
}