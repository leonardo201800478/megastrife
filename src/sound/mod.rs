/// src/sound/mod.rs
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

pub mod psg;
pub mod ym2612;

use psg::Psg;
use ym2612::Ym2612;

/// Constantes de clock do Mega Drive
const MASTER_CLOCK: u32 = 53693175; // 53.693175 MHz (NTSC)
const CPU_CLOCK: u32 = MASTER_CLOCK / 15; // 3.579545 MHz (Z80)
const PSG_CLOCK: u32 = CPU_CLOCK; // 3.579545 MHz
const YM2612_CLOCK: u32 = MASTER_CLOCK / 7; // 7.670454 MHz

/// Representa o sistema de som do Mega Drive (PSG + YM2612)
pub struct Sound {
    // Usamos RwLock para permitir múltiplas leituras (sample) e escrita exclusiva (tick)
    // Embora Mutex seja suficiente para este caso, RwLock é mais idiomático para acesso a hardware.
    pub psg: Arc<RwLock<Psg>>,
    pub fm: Arc<RwLock<Ym2612>>,
    // Taxa de amostragem de saída (e.g., 44100 Hz)
    sample_rate: u32,
    // Ciclos do clock principal por amostra de saída
    cycles_per_sample: f64,
    // Contador de ciclos para sincronização
    current_cycles: f64,
}

impl Sound {
    pub fn new(sample_rate: u32) -> Self {
        // O Mega Drive tem um clock principal de 53.693175 MHz.
        // O PSG é clockado a 3.579545 MHz (CPU_CLOCK).
        // O YM2612 é clockado a 7.670454 MHz (MASTER_CLOCK / 7).
        // A emulação deve avançar os chips com base no clock principal.
        let cycles_per_sample = MASTER_CLOCK as f64 / sample_rate as f64;

        Self {
            psg: Arc::new(RwLock::new(Psg::new(sample_rate))),
            fm: Arc::new(RwLock::new(Ym2612::new(sample_rate))),
            sample_rate,
            cycles_per_sample,
            current_cycles: 0.0,
        }
    }

    /// Atualiza os chips de som por um número de ciclos do clock principal.
    pub fn tick(&mut self, cycles: u32) {
        // O Mega Drive usa o clock principal para sincronizar os chips.
        // O PSG avança a cada 16 ciclos do clock do PSG (3.58MHz).
        // O YM2612 avança a cada 12 ciclos do clock do YM2612 (7.67MHz).
        // Como estamos usando implementações baseadas em registradores,
        // vamos delegar a lógica de clock para dentro de cada chip,
        // mas o `tick` principal deve ser chamado com os ciclos do clock principal.

        // O PSG usa o clock do Z80 (3.58MHz).
        let psg_cycles = (cycles as f64 * (PSG_CLOCK as f64 / MASTER_CLOCK as f64)).round() as u32;
        // O YM2612 usa o clock de 7.67MHz.
        let fm_cycles = (cycles as f64 * (YM2612_CLOCK as f64 / MASTER_CLOCK as f64)).round() as u32;

        self.psg.write().tick(psg_cycles);
        self.fm.write().tick(fm_cycles);
    }

    /// Gera uma amostra de áudio.
    /// Esta função deve ser chamada na taxa de amostragem de saída (e.g., 44100 Hz).
    pub fn sample(&self) -> f32 {
        // A mixagem deve ser feita em estéreo, mas o PSG é mono.
        // O YM2612 é estéreo.

        // PSG (Mono)
        let psg_sample = self.psg.read().sample();

        // YM2612 (Estéreo)
        let (fm_left, fm_right) = self.fm.read().sample();

        // Mixagem simples (média)
        // Para um emulador real, a mixagem é mais complexa, envolvendo
        // a atenuação correta dos volumes dos chips.
        (psg_sample + fm_left + fm_right) / 3.0
    }

    /// Gera uma amostra de áudio estéreo.
    pub fn sample_stereo(&self) -> (f32, f32) {
        // PSG (Mono, distribuído igualmente para L e R)
        let psg_sample = self.psg.read().sample();

        // YM2612 (Estéreo)
        let (fm_left, fm_right) = self.fm.read().sample();

        // Mixagem simples (média)
        // Para um emulador real, a mixagem é mais complexa, envolvendo
        // a atenuação correta dos volumes dos chips.
        let left = (psg_sample + fm_left) / 2.0;
        let right = (psg_sample + fm_right) / 2.0;

        (left, right)
    }
}
