// src/cpu/dma.rs

//! Controlador de DMA (Direct Memory Access) entre M68000 e Z80.
//! Permite transferências diretas de dados entre as CPUs.

use crate::cpu::bus::{Bus, BusError};
use thiserror::Error;

/// Direção da transferência DMA.
#[derive(Debug, Clone, Copy)]
pub enum DmaDirection {
    From68kToZ80,
    FromZ80To68k,
}

/// Estado atual do DMA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaState {
    Idle,
    Busy,
    Completed,
}

/// Erros possíveis durante o DMA.
#[derive(Debug, Error)]
pub enum DmaError {
    #[error("Endereço fora do intervalo de memória permitido")]
    InvalidAddress,
    #[error("Erro de barramento: {0}")]
    Bus(#[from] BusError),
}

/// Estrutura principal do controlador DMA.
pub struct DmaController {
    pub state: DmaState,
    pub source_addr: u32,
    pub dest_addr: u32,
    pub length: usize,
    pub direction: DmaDirection,
    pub bytes_transferred: usize,
}

impl DmaController {
    /// Cria um novo controlador DMA inativo.
    pub fn new() -> Self {
        Self {
            state: DmaState::Idle,
            source_addr: 0,
            dest_addr: 0,
            length: 0,
            direction: DmaDirection::From68kToZ80,
            bytes_transferred: 0,
        }
    }

    /// Inicia uma transferência DMA.
    pub fn start_transfer(
        &mut self,
        source_addr: u32,
        dest_addr: u32,
        length: usize,
        direction: DmaDirection,
    ) {
        self.state = DmaState::Busy;
        self.source_addr = source_addr;
        self.dest_addr = dest_addr;
        self.length = length;
        self.direction = direction;
        self.bytes_transferred = 0;
    }

    /// Executa um passo do DMA (simula um ciclo de transferência).
    pub fn tick(&mut self, bus68k: &mut Bus, bus_z80: &mut Bus) -> Result<(), DmaError> {
        if self.state != DmaState::Busy {
            return Ok(());
        }

        if self.bytes_transferred >= self.length {
            self.state = DmaState::Completed;
            return Ok(());
        }

        // Leitura e escrita conforme a direção
        let byte = match self.direction {
            DmaDirection::From68kToZ80 => bus68k.read8(self.source_addr)?,
            DmaDirection::FromZ80To68k => bus_z80.read8(self.source_addr)?,
        };

        match self.direction {
            DmaDirection::From68kToZ80 => bus_z80.write8(self.dest_addr, byte)?,
            DmaDirection::FromZ80To68k => bus68k.write8(self.dest_addr, byte)?,
        };

        // Avança endereços
        self.source_addr = self.source_addr.wrapping_add(1);
        self.dest_addr = self.dest_addr.wrapping_add(1);
        self.bytes_transferred += 1;

        // Quando completar o bloco
        if self.bytes_transferred >= self.length {
            self.state = DmaState::Completed;
        }

        Ok(())
    }

    /// Retorna se o DMA está em execução.
    pub fn is_busy(&self) -> bool {
        self.state == DmaState::Busy
    }

    /// Reseta o DMA.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::bus::Bus;

    #[test]
    fn test_dma_transfer_68k_to_z80() {
        let mut bus68k = Bus::new(vec![1, 2, 3, 4, 5], 64 * 1024);
        let mut bus_z80 = Bus::new(vec![0; 5], 64 * 1024);
        let mut dma = DmaController::new();

        dma.start_transfer(0x000000, 0x000000, 5, DmaDirection::From68kToZ80);
        while dma.is_busy() {
            dma.tick(&mut bus68k, &mut bus_z80).unwrap();
        }

        for i in 0..5 {
            assert_eq!(bus_z80.read8(i as u32).unwrap(), (i + 1) as u8);
        }
    }
}
