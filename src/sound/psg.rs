/// src/sound/psg.rs
/// Implementação básica do SN76489 (PSG do Mega Drive) com base em registradores.
/// A emulação completa do SN76489 é complexa, mas esta estrutura
/// fornece a interface correta para um emulador.

// Constantes do SN76489 no Mega Drive
// O clock do SN76489 é o clock principal (7.67MHz) dividido por 2.
const PSG_CLOCK: f64 = 3579545.0; // ~3.58 MHz
const CLOCK_DIVIDER: u32 = 16; // O SN76489 divide o clock por 16 para o gerador de tom.
const NUM_CHANNELS: usize = 4; // 3 canais de tom + 1 canal de ruído

// Estrutura para um canal de tom (Tone Channel)
struct ToneChannel {
    counter: u16,
    period: u16,
    output: bool,
    volume: u8, // 0-15
}

impl ToneChannel {
    fn new() -> Self {
        Self {
            counter: 0,
            period: 0,
            output: false,
            volume: 0xF, // Silêncio
        }
    }

    fn tick(&mut self) {
        if self.period > 0 {
            self.counter = self.counter.wrapping_add(1);
            if self.counter >= self.period {
                self.counter = 0;
                self.output = !self.output;
            }
        }
    }

    fn sample(&self) -> f32 {
        if self.volume == 0xF {
            0.0
        } else {
            // Conversão simples de volume (logarítmica seria mais precisa)
            let amplitude = 1.0 - (self.volume as f32 / 15.0);
            if self.output { amplitude } else { -amplitude }
        }
    }
}

// Estrutura para o canal de ruído (Noise Channel)
struct NoiseChannel {
    shift_register: u16,
    period: u8, // 0-3 (0: N/512, 1: N/1024, 2: N/2048, 3: Tom 3)
    counter: u16,
    volume: u8,
    feedback_mode: bool, // true para ruído branco (período 3)
}

impl NoiseChannel {
    fn new() -> Self {
        Self {
            shift_register: 0x8000, // Valor inicial
            period: 0,
            counter: 0,
            volume: 0xF,
            feedback_mode: false,
        }
    }

    fn tick(&mut self, tone3_output: bool) {
        let period_divisor = match self.period {
            0 => 512,
            1 => 1024,
            2 => 2048,
            3 => {
                // Ruído é sincronizado com o canal de tom 3
                if tone3_output {
                    // O ruído só avança quando o tom 3 muda de estado (borda de descida)
                    // Para simplificar, vamos usar uma aproximação baseada no período do tom 3
                    // Uma implementação real precisaria do estado do ToneChannel 2
                    // Por enquanto, vamos usar uma taxa fixa para simular o ruído.
                    // Isso é um placeholder para a lógica real de sincronização.
                    self.counter = self.counter.wrapping_add(1);
                    if self.counter >= 32 { // Valor arbitrário para simular frequência
                        self.counter = 0;
                        self.shift_register = self.shift_register.wrapping_add(1); // Apenas para simular
                    }
                    return;
                }
                return;
            }
            _ => unreachable!(),
        };

        self.counter = self.counter.wrapping_add(1);
        if self.counter >= period_divisor {
            self.counter = 0;
            // Lógica de feedback do registrador de deslocamento (LFSR)
            let bit_to_shift = if self.feedback_mode {
                // Ruído branco (XOR do bit 0 e 3)
                (self.shift_register & 0x0001) ^ ((self.shift_register & 0x0008) >> 3)
            } else {
                // Ruído periódico (XOR do bit 0)
                self.shift_register & 0x0001
            };

            self.shift_register >>= 1;
            if bit_to_shift != 0 {
                self.shift_register |= 0x8000;
            }
        }
    }

    fn sample(&self) -> f32 {
        if self.volume == 0xF {
            0.0
        } else {
            let amplitude = 1.0 - (self.volume as f32 / 15.0);
            if (self.shift_register & 0x0001) != 0 { amplitude } else { -amplitude }
        }
    }
}

pub struct Psg {
    channels: [ToneChannel; 3],
    noise: NoiseChannel,
    latch: u8,
    clock_cycles: f64,
    sample_rate: f64,
    cycles_per_sample: f64,
    current_cycles: f64,
}

impl Psg {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            channels: [ToneChannel::new(), ToneChannel::new(), ToneChannel::new()],
            noise: NoiseChannel::new(),
            latch: 0,
            clock_cycles: PSG_CLOCK,
            sample_rate: sample_rate as f64,
            cycles_per_sample: PSG_CLOCK / sample_rate as f64,
            current_cycles: 0.0,
        }
    }

    /// Escreve um byte no registrador do PSG.
    /// O SN76489 usa um único registrador de escrita.
    pub fn write_data(&mut self, data: u8) {
        if (data & 0x80) != 0 {
            // Latch/Escrita de endereço
            self.latch = (data >> 4) & 0x07;
            self.write_register(self.latch, data & 0x3F);
        } else {
            // Escrita de dados
            self.write_register(self.latch, data & 0x7F);
        }
    }

    fn write_register(&mut self, reg: u8, data: u8) {
        match reg {
            0..=2 => {
                // Canais de Tom 0, 1, 2 (Período Baixo)
                let channel_index = reg as usize;
                let channel = &mut self.channels[channel_index];
                channel.period = (channel.period & 0x03C0) | (data as u16 & 0x003F);
                channel.counter = 0;
            }
            4..=6 => {
                // Canais de Tom 0, 1, 2 (Período Alto)
                let channel_index = (reg - 4) as usize;
                let channel = &mut self.channels[channel_index];
                channel.period = (channel.period & 0x003F) | ((data as u16 & 0x0003) << 6);
                channel.counter = 0;
            }
            3 => {
                // Canal de Ruído (Controle)
                self.noise.period = data & 0x03;
                self.noise.feedback_mode = (data & 0x04) != 0;
                self.noise.shift_register = 0x8000; // Reset do LFSR
            }
            7 => {
                // Canais de Volume 0, 1, 2, Ruído
                let channel_index = (self.latch - 7) as usize;
                let volume = data & 0x0F;
                match channel_index {
                    0..=2 => self.channels[channel_index].volume = volume,
                    3 => self.noise.volume = volume,
                    _ => unreachable!(),
                }
            }
            _ => {
                // Registradores de volume
                let channel_index = (reg - 8) as usize;
                if channel_index < 3 {
                    self.channels[channel_index].volume = data & 0x0F;
                } else if channel_index == 3 {
                    self.noise.volume = data & 0x0F;
                }
            }
        }
    }

    /// Avança o emulador por um número de ciclos.
    pub fn tick(&mut self, cycles: u32) {
        for _ in 0..cycles {
            // O SN76489 avança a cada 16 ciclos do clock principal (7.67MHz)
            // ou a cada 8 ciclos do clock do PSG (3.58MHz).
            // Vamos simplificar e usar o clock do PSG.
            // O gerador de tom/ruído avança a cada 16 ciclos do clock do PSG.
            // Como o clock do PSG é 3.58MHz, e o divisor é 16, a taxa de atualização é ~223kHz.
            // Para simplificar, vamos usar um contador interno.
            self.current_cycles += 1.0;
            if self.current_cycles >= CLOCK_DIVIDER as f64 {
                self.current_cycles -= CLOCK_DIVIDER as f64;
                self.channels[0].tick();
                self.channels[1].tick();
                self.channels[2].tick();
                self.noise.tick(self.channels[2].output);
            }
        }
    }

    /// Gera uma amostra de áudio.
    pub fn sample(&self) -> f32 {
        let mut sample = 0.0;
        for i in 0..3 {
            sample += self.channels[i].sample();
        }
        sample += self.noise.sample();
        // Normalização simples (o volume real é mais complexo)
        sample / NUM_CHANNELS as f32
    }
}
