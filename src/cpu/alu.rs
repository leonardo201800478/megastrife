//! Unidade Lógica Aritmética (ALU) do Motorola 68000
//! Implementa todas as operações aritméticas e lógicas

use super::bus::CpuBus;
use super::decoder::{AddressingMode, Size};
use super::registers::{Registers, StatusRegister};
use anyhow::{bail, Context, Result};
use log::{debug, trace};

/// Contexto da ALU com acesso à CPU
pub struct Alu<'a> {
    cpu: &'a mut crate::cpu::M68000,
}

/// Resultado de uma operação aritmética
#[derive(Debug, Clone, Copy)]
pub struct AluResult {
    pub value: u32,
    pub cycles: u32,
    pub flags: AluFlags,
}

/// Flags da ALU após operação
#[derive(Debug, Clone, Copy, Default)]
pub struct AluFlags {
    pub carry: bool,
    pub overflow: bool,
    pub zero: bool,
    pub negative: bool,
    pub extend: bool,
}

impl<'a> Alu<'a> {
    /// Cria um novo contexto ALU
    pub fn new(cpu: &'a mut crate::cpu::M68000) -> Self {
        Self { cpu }
    }

    // ============================================
    // OPERAÇÕES DE MOVIMENTAÇÃO DE DADOS
    // ============================================

    /// Instrução MOVE - Move dados entre operandos
    pub fn move_instruction(
        &mut self,
        src: AddressingMode,
        dst: AddressingMode,
        size: Size,
    ) -> Result<u32> {
        trace!("MOVE {:?} -> {:?} ({:?})", src, dst, size);

        // Lê o valor fonte
        let (src_value, src_cycles) = self.read_operand(src, size, false)?;

        // Escreve no destino
        let dst_cycles = self.write_operand(dst, src_value, size)?;

        // Atualiza flags (MOVE atualiza flags baseado no valor movido)
        self.update_flags_after_move(src_value, size);

        Ok(src_cycles + dst_cycles)
    }

    /// Atualiza flags após instrução MOVE
    fn update_flags_after_move(&mut self, value: u32, size: Size) {
        let (masked_value, is_negative, is_zero) = self.get_value_info(value, size);

        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::OVERFLOW, false);
        self.cpu.regs.sr.set(StatusRegister::CARRY, false);
    }

    // ============================================
    // OPERAÇÕES ARITMÉTICAS
    // ============================================

    /// Instrução ADD - Adição
    pub fn add(&mut self, src: AddressingMode, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("ADD {:?} + {:?} ({:?})", src, dst, size);

        // Lê operandos
        let (src_value, src_cycles) = self.read_operand(src, size, false)?;
        let (dst_value, _) = self.read_operand(dst.clone(), size, true)?;

        // Executa adição
        let result = self.add_internal(dst_value, src_value, size, false);

        // Escreve resultado
        let dst_cycles = self.write_operand(dst, result.value, size)?;

        // Atualiza flags
        self.update_flags_from_alu(result.flags);

        Ok(src_cycles + dst_cycles + result.cycles)
    }

    /// Instrução ADD com extend (ADDX)
    pub fn addx(&mut self, src: AddressingMode, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("ADDX {:?} + {:?} ({:?})", src, dst, size);

        let (src_value, src_cycles) = self.read_operand(src, size, false)?;
        let (dst_value, _) = self.read_operand(dst.clone(), size, true)?;

        // ADDX inclui o flag X no carry-in
        let carry_in = if self.cpu.regs.sr.contains(StatusRegister::EXTEND) {
            1
        } else {
            0
        };
        let result = self.add_with_carry(dst_value, src_value, carry_in, size);

        let dst_cycles = self.write_operand(dst, result.value, size)?;
        self.update_flags_from_alu(result.flags);

        Ok(src_cycles + dst_cycles + result.cycles)
    }

    /// Instrução SUB - Subtração
    pub fn sub(&mut self, src: AddressingMode, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("SUB {:?} - {:?} ({:?})", dst, src, size);

        let (src_value, src_cycles) = self.read_operand(src, size, false)?;
        let (dst_value, _) = self.read_operand(dst.clone(), size, true)?;

        // SUB é ADD com o complemento de 2 da fonte
        let src_complement = (!src_value).wrapping_add(1);
        let result = self.add_internal(dst_value, src_complement, size, false);

        let dst_cycles = self.write_operand(dst, result.value, size)?;
        self.update_flags_from_alu(result.flags);

        Ok(src_cycles + dst_cycles + result.cycles)
    }

    /// Instrução SUB com extend (SUBX)
    pub fn subx(&mut self, src: AddressingMode, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("SUBX {:?} - {:?} ({:?})", dst, src, size);

        let (src_value, src_cycles) = self.read_operand(src, size, false)?;
        let (dst_value, _) = self.read_operand(dst.clone(), size, true)?;

        // SUBX: dst - src - X
        let carry_in = if self.cpu.regs.sr.contains(StatusRegister::EXTEND) {
            1
        } else {
            0
        };
        let src_complement = (!src_value).wrapping_add(1);
        let result = self.add_with_carry(dst_value, src_complement, carry_in, size);

        let dst_cycles = self.write_operand(dst, result.value, size)?;
        self.update_flags_from_alu(result.flags);

        Ok(src_cycles + dst_cycles + result.cycles)
    }

    /// Instrução MULU - Multiplicação sem sinal
    pub fn mulu(&mut self, src: AddressingMode, dst: AddressingMode) -> Result<u32> {
        trace!("MULU {:?} * {:?}", src, dst);

        let (src_value, src_cycles) = self.read_operand(src, Size::Word, false)?;
        let (dst_value, _) = self.read_operand(dst.clone(), Size::Word, true)?;

        // Multiplicação 16x16 → 32 bits
        let result = (src_value as u32).wrapping_mul(dst_value as u32);

        // Escreve resultado em Dn (32 bits)
        let dst_cycles = self.write_operand(dst, result, Size::Long)?;

        // Atualiza flags
        let is_negative = (result as i32) < 0;
        let is_zero = result == 0;

        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::OVERFLOW, false);
        self.cpu.regs.sr.set(StatusRegister::CARRY, false);

        // MULU tem ciclos variáveis (38 + 2n onde n é número de 1s no multiplicador)
        let extra_cycles = self.calculate_mulu_cycles(src_value as u16);

        Ok(src_cycles + dst_cycles + extra_cycles)
    }

    /// Instrução MULS - Multiplicação com sinal
    pub fn muls(&mut self, src: AddressingMode, dst: AddressingMode) -> Result<u32> {
        trace!("MULS {:?} * {:?}", src, dst);

        let (src_value, src_cycles) = self.read_operand(src, Size::Word, false)?;
        let (dst_value, _) = self.read_operand(dst.clone(), Size::Word, true)?;

        // Multiplicação com sinal
        let src_signed = src_value as i16 as i32;
        let dst_signed = dst_value as i16 as i32;
        let result = (src_signed.wrapping_mul(dst_signed)) as u32;

        let dst_cycles = self.write_operand(dst, result, Size::Long)?;

        // Atualiza flags (igual MULU)
        let is_negative = (result as i32) < 0;
        let is_zero = result == 0;

        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::OVERFLOW, false);
        self.cpu.regs.sr.set(StatusRegister::CARRY, false);

        // MULS tem ciclos similares ao MULU
        let extra_cycles = self.calculate_muls_cycles(src_value as u16);

        Ok(src_cycles + dst_cycles + extra_cycles)
    }

    /// Instrução DIVU - Divisão sem sinal
    pub fn divu(&mut self, src: AddressingMode, dst: AddressingMode) -> Result<u32> {
        trace!("DIVU {:?} / {:?}", dst, src);

        let (src_value, src_cycles) = self.read_operand(src, Size::Word, false)?;
        let (dst_value, _) = self.read_operand(dst.clone(), Size::Long, true)?;

        // Verifica divisão por zero
        if src_value == 0 {
            // Gera exceção de divisão por zero
            return Ok(src_cycles + 10); // Ciclos antes da exceção
        }

        // Divisão 32/16 → 16 quociente, 16 resto
        let quotient = (dst_value as u32).wrapping_div(src_value as u32) as u16;
        let remainder = (dst_value as u32).wrapping_rem(src_value as u32) as u16;

        // Resultado: quociente no lower word, resto no higher word
        let result = ((remainder as u32) << 16) | (quotient as u32);

        let dst_cycles = self.write_operand(dst, result, Size::Long)?;

        // Atualiza flags
        let is_negative = (quotient as i16) < 0;
        let is_zero = quotient == 0;
        let overflow = quotient > 0xFFFF; // Overflow se quociente não cabe em 16 bits

        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::OVERFLOW, overflow);
        self.cpu.regs.sr.set(StatusRegister::CARRY, false);

        // DIVU tem ciclos variáveis
        let extra_cycles = self.calculate_divu_cycles(dst_value as u32, src_value as u16);

        Ok(src_cycles + dst_cycles + extra_cycles)
    }

    /// Instrução DIVS - Divisão com sinal
    pub fn divs(&mut self, src: AddressingMode, dst: AddressingMode) -> Result<u32> {
        trace!("DIVS {:?} / {:?}", dst, src);

        let (src_value, src_cycles) = self.read_operand(src, Size::Word, false)?;
        let (dst_value, _) = self.read_operand(dst.clone(), Size::Long, true)?;

        if src_value == 0 {
            return Ok(src_cycles + 10); // Exceção divisão por zero
        }

        let src_signed = src_value as i16 as i32;
        let dst_signed = dst_value as i32;

        let quotient = dst_signed.wrapping_div(src_signed) as i16 as u16;
        let remainder = dst_signed.wrapping_rem(src_signed) as i16 as u16;

        let result = ((remainder as u32) << 16) | (quotient as u32);

        let dst_cycles = self.write_operand(dst, result, Size::Long)?;

        // Flags para DIVS
        let is_negative = (quotient as i16) < 0;
        let is_zero = quotient == 0;
        let overflow = (quotient as i16) < -32768 || (quotient as i16) > 32767;

        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::OVERFLOW, overflow);
        self.cpu.regs.sr.set(StatusRegister::CARRY, false);

        let extra_cycles = self.calculate_divs_cycles(dst_value as i32, src_value as i16);

        Ok(src_cycles + dst_cycles + extra_cycles)
    }

    /// Instrução NEG - Negação (complemento de 2)
    pub fn neg(&mut self, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("NEG {:?} ({:?})", dst, size);

        let (dst_value, _) = self.read_operand(dst.clone(), size, true)?;

        // NEG é 0 - dst
        let result = self.add_internal(0, !dst_value + 1, size, false);

        let dst_cycles = self.write_operand(dst, result.value, size)?;
        self.update_flags_from_alu(result.flags);

        Ok(dst_cycles + result.cycles)
    }

    /// Instrução NEGX - Negação com extend
    pub fn negx(&mut self, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("NEGX {:?} ({:?})", dst, size);

        let (dst_value, _) = self.read_operand(dst.clone(), size, true)?;
        let carry_in = if self.cpu.regs.sr.contains(StatusRegister::EXTEND) {
            1
        } else {
            0
        };

        // NEGX: 0 - dst - X
        let result = self.add_with_carry(0, !dst_value + 1, carry_in, size);

        let dst_cycles = self.write_operand(dst, result.value, size)?;
        self.update_flags_from_alu(result.flags);

        Ok(dst_cycles + result.cycles)
    }

    // ============================================
    // OPERAÇÕES LÓGICAS
    // ============================================

    /// Instrução AND - AND lógico
    pub fn and(&mut self, src: AddressingMode, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("AND {:?} & {:?} ({:?})", src, dst, size);

        let (src_value, src_cycles) = self.read_operand(src, size, false)?;
        let (dst_value, _) = self.read_operand(dst.clone(), size, true)?;

        let result = dst_value & src_value;

        let dst_cycles = self.write_operand(dst, result, size)?;

        // Atualiza flags
        let (_, is_negative, is_zero) = self.get_value_info(result, size);
        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::OVERFLOW, false);
        self.cpu.regs.sr.set(StatusRegister::CARRY, false);

        Ok(src_cycles + dst_cycles)
    }

    /// Instrução OR - OR lógico
    pub fn or(&mut self, src: AddressingMode, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("OR {:?} | {:?} ({:?})", src, dst, size);

        let (src_value, src_cycles) = self.read_operand(src, size, false)?;
        let (dst_value, _) = self.read_operand(dst.clone(), size, true)?;

        let result = dst_value | src_value;

        let dst_cycles = self.write_operand(dst, result, size)?;

        let (_, is_negative, is_zero) = self.get_value_info(result, size);
        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::OVERFLOW, false);
        self.cpu.regs.sr.set(StatusRegister::CARRY, false);

        Ok(src_cycles + dst_cycles)
    }

    /// Instrução EOR - XOR lógico
    pub fn xor(&mut self, src: AddressingMode, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("XOR {:?} ^ {:?} ({:?})", src, dst, size);

        let (src_value, src_cycles) = self.read_operand(src, size, false)?;
        let (dst_value, _) = self.read_operand(dst.clone(), size, true)?;

        let result = dst_value ^ src_value;

        let dst_cycles = self.write_operand(dst, result, size)?;

        let (_, is_negative, is_zero) = self.get_value_info(result, size);
        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::OVERFLOW, false);
        self.cpu.regs.sr.set(StatusRegister::CARRY, false);

        Ok(src_cycles + dst_cycles)
    }

    /// Instrução NOT - Complemento de 1
    pub fn not(&mut self, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("NOT {:?} ({:?})", dst, size);

        let (dst_value, _) = self.read_operand(dst.clone(), size, true)?;

        let result = !dst_value;

        let dst_cycles = self.write_operand(dst, result, size)?;

        let (_, is_negative, is_zero) = self.get_value_info(result, size);
        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::OVERFLOW, false);
        self.cpu.regs.sr.set(StatusRegister::CARRY, false);

        Ok(dst_cycles)
    }

    // ============================================
    // OPERAÇÕES DE COMPARAÇÃO E TESTE
    // ============================================

    /// Instrução CMP - Comparação (dst - src, sem armazenar resultado)
    pub fn cmp(&mut self, src: AddressingMode, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("CMP {:?} - {:?} ({:?})", dst, src, size);

        let (src_value, src_cycles) = self.read_operand(src, size, false)?;
        let (dst_value, _) = self.read_operand(dst, size, false)?;

        // Executa subtração apenas para atualizar flags
        let src_complement = (!src_value).wrapping_add(1);
        let result = self.add_internal(dst_value, src_complement, size, false);

        // Apenas atualiza flags, não escreve resultado
        self.update_flags_from_alu(result.flags);

        Ok(src_cycles + result.cycles)
    }

    /// Instrução CMPA - Comparação com registrador de endereço
    pub fn cmpa(&mut self, src: AddressingMode, dst: AddressingMode, size: Size) -> Result<u32> {
        // CMPA sempre usa tamanho word ou long
        let actual_size = match size {
            Size::Byte => Size::Word, // Byte é estendido para word
            s => s,
        };

        self.cmp(src, dst, actual_size)
    }

    /// Instrução CMPM - Comparação com incremento pós (memória)
    pub fn cmpm(&mut self, src: AddressingMode, dst: AddressingMode, size: Size) -> Result<u32> {
        // Similar ao CMP, mas incrementa os registradores de endereço após
        let cycles = self.cmp(src, dst, size)?;

        // Incrementa os registradores de endereço (se aplicável)
        // (Implementação simplificada)

        Ok(cycles + 2) // Ciclos extras para incremento
    }

    /// Instrução TST - Teste (AND consigo mesmo, apenas flags)
    pub fn tst(&mut self, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("TST {:?} ({:?})", dst, size);

        let (dst_value, dst_cycles) = self.read_operand(dst, size, false)?;

        // TST é basicamente AND dst, dst (sem armazenar)
        let (_, is_negative, is_zero) = self.get_value_info(dst_value, size);

        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::OVERFLOW, false);
        self.cpu.regs.sr.set(StatusRegister::CARRY, false);

        Ok(dst_cycles)
    }

    // ============================================
    // OPERAÇÕES DE DESLOCAMENTO E ROTAÇÃO
    // ============================================

    /// Instrução ASL/ASR - Deslocamento aritmético
    pub fn arithmetic_shift(
        &mut self,
        dst: AddressingMode,
        count: u8,
        size: Size,
        left: bool, // true = ASL, false = ASR
    ) -> Result<u32> {
        trace!(
            "AS{} {:?}, {} ({:?})",
            if left { 'L' } else { 'R' },
            dst,
            count,
            size
        );

        let (mut value, dst_cycles) = self.read_operand(dst.clone(), size, true)?;
        let shift_count = count & 0x3F; // M68000 limita a 63 deslocamentos

        if shift_count == 0 {
            // Nenhum deslocamento, apenas atualiza flags (exceto carry)
            let (_, is_negative, is_zero) = self.get_value_info(value, size);
            self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
            self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
            // Overflow flag é limpa quando count = 0
            self.cpu.regs.sr.set(StatusRegister::OVERFLOW, false);
            return Ok(dst_cycles);
        }

        let bit_width = self.get_bit_width(size);
        let mut last_carry = false;
        let mut overflow_occurred = false;

        for i in 0..shift_count {
            if left {
                // ASL: desloca para esquerda, MSB → X, LSB ← 0
                let msb = (value >> (bit_width - 1)) & 1;
                last_carry = msb == 1;

                // Verifica overflow (mudança de sinal)
                if i == 0 {
                    let new_msb = (value >> (bit_width - 2)) & 1;
                    overflow_occurred = (msb != 0) && (msb != new_msb);
                }

                value = (value << 1) & self.get_size_mask(size);
            } else {
                // ASR: desloca para direita, MSB se preserva, LSB → X
                let lsb = value & 1;
                last_carry = lsb == 1;

                let sign_bit = value >> (bit_width - 1);
                value = (value >> 1) | (sign_bit << (bit_width - 1));
            }
        }

        let dst_cycles_write = self.write_operand(dst, value, size)?;

        // Atualiza flags
        let (_, is_negative, is_zero) = self.get_value_info(value, size);
        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::CARRY, last_carry);
        self.cpu
            .regs
            .sr
            .set(StatusRegister::OVERFLOW, overflow_occurred);
        // X flag é copiada do C flag
        self.cpu.regs.sr.set(StatusRegister::EXTEND, last_carry);

        // Ciclos extras baseados no count
        let extra_cycles = if shift_count == 1 {
            0
        } else {
            (shift_count as u32) * 2
        };

        Ok(dst_cycles + dst_cycles_write + extra_cycles)
    }

    /// Instrução LSL/LSR - Deslocamento lógico
    pub fn logical_shift(
        &mut self,
        dst: AddressingMode,
        count: u8,
        size: Size,
        left: bool, // true = LSL, false = LSR
    ) -> Result<u32> {
        trace!(
            "LS{} {:?}, {} ({:?})",
            if left { 'L' } else { 'R' },
            dst,
            count,
            size
        );

        let (mut value, dst_cycles) = self.read_operand(dst.clone(), size, true)?;
        let shift_count = count & 0x3F;

        if shift_count == 0 {
            // Mesmo comportamento do ASL/ASR
            let (_, is_negative, is_zero) = self.get_value_info(value, size);
            self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
            self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
            self.cpu.regs.sr.set(StatusRegister::OVERFLOW, false);
            return Ok(dst_cycles);
        }

        let bit_width = self.get_bit_width(size);
        let mut last_carry = false;
        let mut overflow_occurred = false;

        for i in 0..shift_count {
            if left {
                // LSL: igual ASL mas overflow é definido diferente
                let msb = (value >> (bit_width - 1)) & 1;
                last_carry = msb == 1;

                // Overflow é definido como XOR dos dois MSBs após o shift
                if i == 0 {
                    let new_msb = (value >> (bit_width - 2)) & 1;
                    overflow_occurred = (msb ^ new_msb) != 0;
                }

                value = (value << 1) & self.get_size_mask(size);
            } else {
                // LSR: desloca para direita, 0 → MSB, LSB → X
                let lsb = value & 1;
                last_carry = lsb == 1;
                value >>= 1;
            }
        }

        let dst_cycles_write = self.write_operand(dst, value, size)?;

        let (_, is_negative, is_zero) = self.get_value_info(value, size);
        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::CARRY, last_carry);
        self.cpu
            .regs
            .sr
            .set(StatusRegister::OVERFLOW, overflow_occurred);
        self.cpu.regs.sr.set(StatusRegister::EXTEND, last_carry);

        let extra_cycles = if shift_count == 1 {
            0
        } else {
            (shift_count as u32) * 2
        };

        Ok(dst_cycles + dst_cycles_write + extra_cycles)
    }

    /// Instrução ROL/ROR - Rotação
    pub fn rotate(
        &mut self,
        dst: AddressingMode,
        count: u8,
        size: Size,
        left: bool, // true = ROL, false = ROR
    ) -> Result<u32> {
        trace!(
            "RO{} {:?}, {} ({:?})",
            if left { 'L' } else { 'R' },
            dst,
            count,
            size
        );

        let (mut value, dst_cycles) = self.read_operand(dst.clone(), size, true)?;
        let shift_count = count & 0x3F;

        if shift_count == 0 {
            // Apenas atualiza flags (exceto carry)
            let (_, is_negative, is_zero) = self.get_value_info(value, size);
            self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
            self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
            self.cpu.regs.sr.set(StatusRegister::OVERFLOW, false);
            return Ok(dst_cycles);
        }

        let bit_width = self.get_bit_width(size);
        let mask = self.get_size_mask(size);
        let mut last_carry = false;

        for _ in 0..shift_count {
            if left {
                // ROL: rotaciona para esquerda através do carry
                let msb = (value >> (bit_width - 1)) & 1;
                last_carry = msb == 1;

                let old_carry = if self.cpu.regs.sr.contains(StatusRegister::CARRY) {
                    1
                } else {
                    0
                };
                value = ((value << 1) & mask) | old_carry;
            } else {
                // ROR: rotaciona para direita através do carry
                let lsb = value & 1;
                last_carry = lsb == 1;

                let old_carry = if self.cpu.regs.sr.contains(StatusRegister::CARRY) {
                    1
                } else {
                    0
                };
                value = (value >> 1) | (old_carry << (bit_width - 1));
            }
        }

        let dst_cycles_write = self.write_operand(dst, value, size)?;

        let (_, is_negative, is_zero) = self.get_value_info(value, size);
        self.cpu.regs.sr.set(StatusRegister::NEGATIVE, is_negative);
        self.cpu.regs.sr.set(StatusRegister::ZERO, is_zero);
        self.cpu.regs.sr.set(StatusRegister::CARRY, last_carry);
        // Overflow é indefinido para ROL/ROR
        self.cpu.regs.sr.set(StatusRegister::OVERFLOW, false);

        let extra_cycles = if shift_count == 1 {
            0
        } else {
            (shift_count as u32) * 2
        };

        Ok(dst_cycles + dst_cycles_write + extra_cycles)
    }

    // ============================================
    // OPERAÇÕES DE BIT
    // ============================================

    /// Instrução BTST - Teste de bit
    pub fn btst(&mut self, bit: u8, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("BTST #{}, {:?} ({:?})", bit, dst, size);

        let (value, dst_cycles) = self.read_operand(dst, size, false)?;
        let bit_pos = bit & 0x1F; // 0-31
        let mask = 1 << bit_pos;

        let bit_is_zero = (value & mask) == 0;
        self.cpu.regs.sr.set(StatusRegister::ZERO, bit_is_zero);

        Ok(dst_cycles)
    }

    /// Instrução BSET - Setar bit
    pub fn bset(&mut self, bit: u8, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("BSET #{}, {:?} ({:?})", bit, dst, size);

        let (value, dst_cycles) = self.read_operand(dst.clone(), size, true)?;
        let bit_pos = bit & 0x1F;
        let mask = 1 << bit_pos;

        let bit_is_zero = (value & mask) == 0;
        let new_value = value | mask;

        let write_cycles = self.write_operand(dst, new_value, size)?;

        self.cpu.regs.sr.set(StatusRegister::ZERO, bit_is_zero);

        Ok(dst_cycles + write_cycles)
    }

    /// Instrução BCLR - Limpar bit
    pub fn bclr(&mut self, bit: u8, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("BCLR #{}, {:?} ({:?})", bit, dst, size);

        let (value, dst_cycles) = self.read_operand(dst.clone(), size, true)?;
        let bit_pos = bit & 0x1F;
        let mask = 1 << bit_pos;

        let bit_is_zero = (value & mask) == 0;
        let new_value = value & !mask;

        let write_cycles = self.write_operand(dst, new_value, size)?;

        self.cpu.regs.sr.set(StatusRegister::ZERO, bit_is_zero);

        Ok(dst_cycles + write_cycles)
    }

    /// Instrução BCHG - Alternar (toggle) bit
    pub fn bchg(&mut self, bit: u8, dst: AddressingMode, size: Size) -> Result<u32> {
        trace!("BCHG #{}, {:?} ({:?})", bit, dst, size);

        let (value, dst_cycles) = self.read_operand(dst.clone(), size, true)?;
        let bit_pos = bit & 0x1F;
        let mask = 1 << bit_pos;

        let bit_is_zero = (value & mask) == 0;
        let new_value = value ^ mask;

        let write_cycles = self.write_operand(dst, new_value, size)?;

        self.cpu.regs.sr.set(StatusRegister::ZERO, bit_is_zero);

        Ok(dst_cycles + write_cycles)
    }

    // ============================================
    // MÉTODOS INTERNOS DA ALU
    // ============================================

    /// Executa adição com flags
    fn add_internal(&self, a: u32, b: u32, size: Size, use_extend: bool) -> AluResult {
        let carry_in = if use_extend && self.cpu.regs.sr.contains(StatusRegister::EXTEND) {
            1
        } else {
            0
        };

        self.add_with_carry(a, b, carry_in, size)
    }

    /// Adição com carry-in
    fn add_with_carry(&self, a: u32, b: u32, carry_in: u32, size: Size) -> AluResult {
        let (bit_width, mask, signed_mask) = self.get_alu_params(size);

        // Estende os valores para 32 bits mantendo o sinal
        let a_signed = self.sign_extend(a, size) as i64;
        let b_signed = self.sign_extend(b, size) as i64;
        let carry_signed = carry_in as i64;

        // Soma com sinal para detectar overflow
        let signed_sum = a_signed + b_signed + carry_signed;

        // Soma sem sinal para detectar carry
        let unsigned_sum = (a as u64) + (b as u64) + (carry_in as u64);

        // Resultado final (com máscara)
        let result = unsigned_sum as u32 & mask;

        // Flags
        let is_negative = (result & signed_mask) != 0;
        let is_zero = result == 0;

        // Carry: se houve carry além do bit mais significativo
        let carry = (unsigned_sum >> bit_width) != 0;

        // Overflow: mudança de sinal incorreta
        let min_signed = -(1 << (bit_width - 1));
        let max_signed = (1 << (bit_width - 1)) - 1;
        let overflow = signed_sum < min_signed || signed_sum > max_signed;

        // Extend: copia do carry
        let extend = carry;

        AluResult {
            value: result,
            cycles: 0, // Ciclos são calculados externamente
            flags: AluFlags {
                carry,
                overflow,
                zero: is_zero,
                negative: is_negative,
                extend,
            },
        }
    }

    /// Lê um operando (com ou sem efeitos colaterais)
    fn read_operand(
        &mut self,
        mode: AddressingMode,
        size: Size,
        read_for_write: bool,
    ) -> Result<(u32, u32)> {
        use AddressingMode::*;

        match mode {
            DataRegister(n) => {
                let value = self.cpu.regs.d[n as usize];
                let masked = self.mask_value(value, size);
                Ok((masked, 0))
            }
            AddressRegister(n) => {
                let value = self.cpu.regs.get_a(n);
                Ok((value, 0))
            }
            Immediate(value) => {
                let masked = self.mask_value(value, size);
                Ok((masked, 0))
            }
            _ => {
                // Para outros modos de endereçamento, precisamos do sistema de memória
                // Implementação simplificada por enquanto
                Ok((0, 4))
            }
        }
    }

    /// Escreve um operando
    fn write_operand(&mut self, mode: AddressingMode, value: u32, size: Size) -> Result<u32> {
        use AddressingMode::*;

        let masked = self.mask_value(value, size);

        match mode {
            DataRegister(n) => {
                self.cpu.regs.d[n as usize] = masked;
                Ok(0)
            }
            AddressRegister(n) => {
                // Para registradores de endereço, não aplicamos máscara de tamanho
                self.cpu.regs.set_a(n, value);
                Ok(0)
            }
            _ => {
                // Outros modos precisam de acesso à memória
                Ok(4)
            }
        }
    }

    /// Atualiza flags do Status Register a partir da ALU
    fn update_flags_from_alu(&mut self, flags: AluFlags) {
        self.cpu.regs.sr.set(StatusRegister::CARRY, flags.carry);
        self.cpu
            .regs
            .sr
            .set(StatusRegister::OVERFLOW, flags.overflow);
        self.cpu.regs.sr.set(StatusRegister::ZERO, flags.zero);
        self.cpu
            .regs
            .sr
            .set(StatusRegister::NEGATIVE, flags.negative);
        self.cpu.regs.sr.set(StatusRegister::EXTEND, flags.extend);
    }

    /// Obtém informações sobre um valor (mascarado, negativo, zero)
    fn get_value_info(&self, value: u32, size: Size) -> (u32, bool, bool) {
        let masked = self.mask_value(value, size);
        let (_, _, signed_mask) = self.get_alu_params(size);

        let is_negative = (masked & signed_mask) != 0;
        let is_zero = masked == 0;

        (masked, is_negative, is_zero)
    }

    /// Aplica máscara de tamanho a um valor
    fn mask_value(&self, value: u32, size: Size) -> u32 {
        match size {
            Size::Byte => value & 0xFF,
            Size::Word => value & 0xFFFF,
            Size::Long => value,
        }
    }

    /// Estende sinal para 32 bits
    fn sign_extend(&self, value: u32, size: Size) -> i32 {
        match size {
            Size::Byte => {
                let byte = value as u8 as i8;
                byte as i32
            }
            Size::Word => {
                let word = value as u16 as i16;
                word as i32
            }
            Size::Long => value as i32,
        }
    }

    /// Obtém parâmetros da ALU baseado no tamanho
    fn get_alu_params(&self, size: Size) -> (u32, u32, u32) {
        match size {
            Size::Byte => (8, 0xFF, 0x80),
            Size::Word => (16, 0xFFFF, 0x8000),
            Size::Long => (32, 0xFFFFFFFF, 0x80000000),
        }
    }

    /// Obtém largura em bits
    fn get_bit_width(&self, size: Size) -> u32 {
        match size {
            Size::Byte => 8,
            Size::Word => 16,
            Size::Long => 32,
        }
    }

    /// Obtém máscara de tamanho
    fn get_size_mask(&self, size: Size) -> u32 {
        match size {
            Size::Byte => 0xFF,
            Size::Word => 0xFFFF,
            Size::Long => 0xFFFFFFFF,
        }
    }

    // ============================================
    // CÁLCULOS DE CICLOS (aproximados)
    // ============================================

    fn calculate_mulu_cycles(&self, multiplicand: u16) -> u32 {
        // MULU: 38 + 2n onde n é número de 1s no multiplicand
        let ones_count = multiplicand.count_ones() as u32;
        38 + (2 * ones_count)
    }

    fn calculate_muls_cycles(&self, multiplicand: u16) -> u32 {
        // MULS: 38 + 2n onde n é número de 1s + número de 0s após o primeiro 1
        // Implementação simplificada
        self.calculate_mulu_cycles(multiplicand) + 2
    }

    fn calculate_divu_cycles(&self, dividend: u32, divisor: u16) -> u32 {
        if divisor == 0 {
            return 10; // Antes da exceção
        }

        // DIVU: ciclos variam com o quociente
        // Implementação simplificada: 136 ciclos mínimo
        let quotient = dividend / divisor as u32;
        136 + (quotient.count_ones() as u32 * 2)
    }

    fn calculate_divs_cycles(&self, dividend: i32, divisor: i16) -> u32 {
        if divisor == 0 {
            return 10;
        }

        // DIVS: similar a DIVU mas um pouco mais
        let quotient = dividend / divisor as i32;
        140 + (quotient.abs() as u32).count_ones() as u32 * 2
    }
}
