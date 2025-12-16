//! Motorola 68000 CPU Core

mod alu;
mod bus;
mod decoder;
mod registers;

pub use alu::Alu;
pub use bus::CpuBus;
pub use decoder::{AddressingMode, Instruction, Size};
pub use registers::Registers;

use anyhow::{Context, Result};
use log::{debug, trace};

/// Estado da CPU
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CpuState {
    Running,
    Halted,
    Stopped,
}

/// CPU M68000 principal
pub struct M68000 {
    /// Registradores
    pub regs: Registers,

    /// Estado atual
    pub state: CpuState,

    /// Ciclos executados
    pub cycles: u64,

    /// Bus de memória
    bus: Box<dyn CpuBus>,

    /// Último opcode executado (para debug)
    pub last_opcode: u16,
}

impl M68000 {
    /// Cria uma nova CPU M68000
    pub fn new(bus: Box<dyn CpuBus>) -> Self {
        Self {
            regs: Registers::new(),
            state: CpuState::Running,
            cycles: 0,
            bus,
            last_opcode: 0,
        }
    }

    /// Reseta a CPU
    pub fn reset(&mut self) -> Result<()> {
        debug!("CPU Reset");

        // Lê vetores de reset
        let reset_pc = self.read_long(0x00).context("Failed to read reset PC")?;
        let reset_sp = self.read_long(0x04).context("Failed to read reset SP")?;

        self.regs.pc = reset_pc;
        self.regs.ssp = reset_sp;
        self.regs.usp = reset_sp;

        self.state = CpuState::Running;
        self.cycles = 0;

        debug!("Reset complete: PC={:08X}, SP={:08X}", reset_pc, reset_sp);
        Ok(())
    }

    /// Executa um único passo da CPU
    pub fn step(&mut self) -> Result<u32> {
        if self.state != CpuState::Running {
            return Ok(4); // Ciclos mesmo quando parado
        }

        // Busca a instrução
        let opcode = self.read_word(self.regs.pc)?;
        self.last_opcode = opcode;
        self.regs.pc = self.regs.pc.wrapping_add(2);

        trace!(
            "PC={:08X}, Opcode={:04X}",
            self.regs.pc.wrapping_sub(2),
            opcode
        );

        // Decodifica
        let instruction: Instruction = decoder::decode(opcode).map_err(|e: String| anyhow::anyhow!(e))?;

        // Executa
        let cycles: u32 = self.execute(instruction)?;
        self.cycles += cycles as u64;

        Ok(cycles)
    }

    /// Executa uma instrução decodificada
fn execute(&mut self, instruction: decoder::Instruction) -> Result<u32> {
    use decoder::Instruction;
    
    let mut alu = Alu::new(self);
    
    match instruction {
        Instruction::Move(src, dst, size) => alu.move_instruction(src, dst, size),
        Instruction::Add(src, dst, size) => alu.add(src, dst, size),
        Instruction::Sub(src, dst, size) => alu.sub(src, dst, size),
        Instruction::And(src, dst, size) => alu.and(src, dst, size),
        Instruction::Or(src, dst, size) => alu.or(src, dst, size),
        Instruction::Xor(src, dst, size) => alu.xor(src, dst, size),
        Instruction::Cmp(src, dst, size) => alu.cmp(src, dst, size),
        Instruction::Nop => Ok(4),
        Instruction::Illegal(opcode) => {
            debug!("Illegal instruction: {:04X}", opcode);
            Ok(4)
        }
        _ => {
            debug!("Unimplemented: {:?}", instruction);
            Ok(4)
        }
    }
}

    // Métodos de acesso à memória
    pub fn read_byte(&mut self, address: u32) -> Result<u8> {
        self.bus.read_byte(address)
    }

    pub fn read_word(&mut self, address: u32) -> Result<u16> {
        self.bus.read_word(address)
    }

    pub fn read_long(&mut self, address: u32) -> Result<u32> {
        self.bus.read_long(address)
    }

    pub fn write_byte(&mut self, address: u32, value: u8) -> Result<()> {
        self.bus.write_byte(address, value)
    }

    pub fn write_word(&mut self, address: u32, value: u16) -> Result<()> {
        self.bus.write_word(address, value)
    }

    pub fn write_long(&mut self, address: u32, value: u32) -> Result<()> {
        self.bus.write_long(address, value)
    }

    // Métodos ALU (simplificados para exemplo)
    fn alu_move(&mut self, src: AddressingMode, dst: AddressingMode, size: Size) -> Result<u32> {
        let value = self.read_operand(src, size)?;
        self.write_operand(dst, value, size)?;
        Ok(8) // Ciclos aproximados
    }

    fn alu_add(&mut self, src: AddressingMode, dst: AddressingMode, size: Size) -> Result<u32> {
        let src_val: u32 = self.read_operand(src, size)? as u32;
        let dst_val: u32 = self.read_operand(dst.clone(), size)? as u32;

        let result: u32 = src_val.wrapping_add(dst_val);
        self.write_operand(dst, result, size)?;

        // Atualiza flags (simplificado)
        self.regs.sr.zero = result == 0;
        self.regs.sr.negative = (result as i32) < 0;

        Ok(8)
    }

    // Outros métodos ALU...

    fn read_operand(&mut self, mode: AddressingMode, size: Size) -> Result<u32> {
        // Implementação simplificada
        match mode {
            AddressingMode::DataRegister(n) => Ok(self.regs.d[n as usize] as u32),
            AddressingMode::Immediate(value) => Ok(value),
            _ => Ok(0),
        }
    }

    fn write_operand(&mut self, mode: AddressingMode, value: u32, size: Size) -> Result<()> {
        match mode {
            AddressingMode::DataRegister(n) => {
                self.regs.d[n as usize] = value;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
