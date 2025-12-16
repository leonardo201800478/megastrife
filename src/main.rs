//! MegaStrife - Sega Genesis/Mega Drive Emulator
//! Main entry point and system integration

mod cpu;
mod memory;
mod vdp;

use cpu::{CpuBus, M68000};
use memory::{MemoryBus, MemorySystem};
use vdp::{VDP, GENESIS_WIDTH, GENESIS_HEIGHT};
use minifb::{Key, Window, WindowOptions, Scale};
use anyhow::{Result, Context};
use log::{info, warn, error, debug};
use std::time::{Duration, Instant};
use std::path::Path;

/// Emulador principal do Genesis
struct Genesis {
    /// CPU Motorola 68000
    cpu: M68000,
    
    /// Sistema de memória
    memory: MemorySystem,
    
    /// Processador de vídeo (VDP)
    vdp: VDP,
    
    /// Estado de execução
    running: bool,
    
    /// Contador de frames
    frame_count: u64,
    
    /// Contador de ciclos totais
    total_cycles: u64,
    
    /// Tempo da última atualização de FPS
    last_fps_update: Instant,
    
    /// Frames por segundo atual
    current_fps: f64,
}

impl Genesis {
    /// Cria uma nova instância do emulador
    fn new() -> Result<Self> {
        info!("Initializing MegaStrife Genesis Emulator...");
        
        // Cria sistema de memória
        let memory: MemorySystem = MemorySystem::new();
        
        // Cria CPU com o sistema de memória
        let cpu: M68000 = M68000::new(Box::new(memory));
        
        // Cria VDP
        let vdp: VDP = VDP::new();
        
        Ok(Self {
            cpu,
            memory: MemorySystem::new(),
            vdp,
            running: true,
            frame_count: 0,
            total_cycles: 0,
            last_fps_update: Instant::now(),
            current_fps: 0.0,
        })
    }
    
    /// Carrega uma ROM de cartucho
    fn load_rom<P: AsRef<Path>>(&mut self, rom_path: P) -> Result<()> {
        let path: &Path = rom_path.as_ref();
        info!("Loading ROM from: {}", path.display());
        
        // Lê o arquivo ROM
        let rom_data: Vec<u8> = std::fs::read(path)
            .with_context(|| format!("Failed to read ROM file: {}", path.display()))?;
        
        info!("ROM size: {} bytes ({} KB)", 
              rom_data.len(), 
              rom_data.len() / 1024);
        
        // Carrega no sistema de memória
        self.memory.load_cartridge(rom_data)
            .with_context(|| "Failed to load cartridge into memory")?;
        
        // Reseta o sistema
        self.reset()?;
        
        info!("ROM loaded successfully");
        Ok(())
    }
    
    /// Reseta o sistema completo
    fn reset(&mut self) -> Result<()> {
        info!("Resetting system...");
        
        // Reseta a memória
        self.memory.reset();
        
        // Reseta o VDP
        self.vdp.initialize()?;
        
        // Reseta a CPU
        self.cpu.reset()?;
        
        self.frame_count = 0;
        self.total_cycles = 0;
        self.last_fps_update = Instant::now();
        
        info!("System reset complete");
        Ok(())
    }
    
    /// Executa um único frame (aproximadamente 1/60 segundo)
    fn run_frame(&mut self) -> Result<()> {
        // Ciclos por frame (master clock do Genesis / 60 Hz)
        let cycles_per_frame: u64 = 53_693_175 / 60; // ~894,886 ciclos por frame
        
        let mut cycles_this_frame: u64 = 0;
        
        // Executa ciclos até completar um frame
        while cycles_this_frame < cycles_per_frame && self.running {
            // Executa um passo da CPU
            let cpu_cycles: u32 = self.cpu.step()
                .with_context(|| format!("CPU execution failed at PC={:08X}", self.cpu.regs.pc))?;
            
            cycles_this_frame += cpu_cycles as u64;
            self.total_cycles += cpu_cycles as u64;
            
            // Sincroniza o VDP com os ciclos da CPU
            self.vdp.step(cpu_cycles)
                .context("VDP step failed")?;
            
            // Verifica se o VDP gerou uma interrupção
            if self.vdp.has_interrupt() {
                // Processa interrupção do VDP (simplificado)
                // No Genesis real, isso configuraria o nível de interrupção
                self.cpu.assert_interrupt(4); // Nível 4 para VBlank
            }
        }
        
        self.frame_count += 1;
        
        // Renderiza o frame no VDP
        self.vdp.render_mode5_frame()
            .context("Failed to render VDP frame")?;
        
        // Atualiza contador de FPS a cada segundo
        let now: Instant = Instant::now();
        let elapsed: Duration = now.duration_since(self.last_fps_update);
        
        if elapsed >= Duration::from_secs(1) {
            self.current_fps = self.frame_count as f64 / elapsed.as_secs_f64();
            self.last_fps_update = now;
            
            // Log a cada segundo
            info!("Frame: {}, FPS: {:.1}, PC: {:08X}, Cycles: {}M", 
                  self.frame_count, 
                  self.current_fps,
                  self.cpu.regs.pc,
                  self.total_cycles / 1_000_000);
        }
        
        Ok(())
    }
    
    /// Obtém o framebuffer atual para renderização
    fn get_framebuffer(&self) -> &vdp::RenderBuffer {
        self.vdp.get_framebuffer()
    }
    
    /// Manipula entrada do teclado
    fn handle_input(&mut self, window: &Window) -> Result<()> {
        // Tecla ESC: Sai do emulador
        if window.is_key_down(Key::Escape) {
            info!("ESC pressed - stopping emulation");
            self.running = false;
        }
        
        // F1: Reseta o sistema
        if window.is_key_pressed(Key::F1, minifb::KeyRepeat::No) {
            info!("F1 pressed - resetting system");
            self.reset()?;
        }
        
        // F2: Alterna informação de debug
        if window.is_key_pressed(Key::F2, minifb::KeyRepeat::No) {
            info!("F2 pressed - toggling debug info");
            // Implementar toggle de debug info
        }
        
        // F3: Salva estado (TODO)
        if window.is_key_pressed(Key::F3, minifb::KeyRepeat::No) {
            info!("F3 pressed - save state (not implemented)");
        }
        
        // F4: Carrega estado (TODO)
        if window.is_key_pressed(Key::F4, minifb::KeyRepeat::No) {
            info!("F4 pressed - load state (not implemented)");
        }
        
        // Setas: Controle do jogador 1 (simplificado)
        let left: bool = window.is_key_down(Key::Left);
        let right: bool = window.is_key_down(Key::Right);
        let up: bool = window.is_key_down(Key::Up);
        let down: bool = window.is_key_down(Key::Down);
        
        // Botões A, B, C
        let a_button: bool = window.is_key_down(Key::Z);
        let b_button: bool = window.is_key_down(Key::X);
        let c_button: bool = window.is_key_down(Key::C);
        
        // Start button
        let start_button: bool = window.is_key_down(Key::Enter);
        
        // Aqui você processaria a entrada do controle
        // Por enquanto, apenas log se alguma tecla estiver pressionada
        if left || right || up || down || a_button || b_button || c_button || start_button {
            debug!("Input: L:{}, R:{}, U:{}, D:{}, A:{}, B:{}, C:{}, Start:{}",
                   left, right, up, down, a_button, b_button, c_button, start_button);
        }
        
        Ok(())
    }
}

/// Implementação do CpuBus para MemorySystem
impl CpuBus for MemorySystem {
    fn read_byte(&mut self, address: u32) -> Result<u8> {
        MemoryBus::read_byte(self, address)
    }
    
    fn read_word(&mut self, address: u32) -> Result<u16> {
        MemoryBus::read_word(self, address)
    }
    
    fn read_long(&mut self, address: u32) -> Result<u32> {
        MemoryBus::read_long(self, address)
    }
    
    fn write_byte(&mut self, address: u32, value: u8) -> Result<()> {
        MemoryBus::write_byte(self, address, value)
    }
    
    fn write_word(&mut self, address: u32, value: u16) -> Result<()> {
        MemoryBus::write_word(self, address, value)
    }
    
    fn write_long(&mut self, address: u32, value: u32) -> Result<()> {
        MemoryBus::write_long(self, address, value)
    }
}

fn main() -> Result<()> {
    // Inicializa logging
    env_logger::init();
    
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║           MEGASTRIFE - Genesis Emulator              ║");
    println!("║                 Version 0.1.0                        ║");
    println!("║      Built with Rust + Minifb - No SDL2 required     ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();
    
    // Verifica argumentos da linha de comando
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        println!("Usage: {} <rom_file>", args[0]);
        println!("Example: {} roms/sonic.smd", args[0]);
        println!();
        println!("Available test ROMs:");
        println!("  roms/test/Sonic The Hedgehog (USA, Europe).md");
        println!("  roms/test/Altered Beast (USA, Europe).md");
        return Ok(());
    }
    
    let rom_path: &String = &args[1];
    
    // Cria o emulador
    let mut genesis: Genesis = Genesis::new()
        .context("Failed to create Genesis emulator")?;
    
    // Carrega a ROM
    genesis.load_rom(rom_path)
        .with_context(|| format!("Failed to load ROM: {}", rom_path))?;
    
    // Configuração da janela
    let window_width: usize = GENESIS_WIDTH * 2;
    let window_height: usize = GENESIS_HEIGHT * 2;
    
    let mut buffer: Vec<u32> = vec![0u32; window_width * window_height];
    
    // Cria a janela
    let mut window: Window = Window::new(
        "MegaStrife - Sega Genesis Emulator",
        window_width,
        window_height,
        WindowOptions {
            title: true,
            resize: true,
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    ).context("Failed to create window")?;
    
    println!();
    println!("═ Controls ════════════════════════════════════════");
    println!("  Arrow Keys:   D-Pad");
    println!("  Z:            Button A");
    println!("  X:            Button B");
    println!("  C:            Button C");
    println!("  Enter:        Start Button");
    println!("  ESC:          Exit Emulator");
    println!("  F1:           Reset System");
    println!("  F2:           Toggle Debug Info");
    println!("  F3:           Save State (TODO)");
    println!("  F4:           Load State (TODO)");
    println!("═══════════════════════════════════════════════════");
    println!();
    println!("Emulation started. Press ESC to exit.");
    println!();
    
    let mut last_frame_time: Instant = Instant::now();
    
    // Loop principal
    while genesis.running && window.is_open() {
        // Processa entrada
        genesis.handle_input(&window)?;
        
        // Executa um frame
        genesis.run_frame()?;
        
        // Obtém o framebuffer do VDP
        let vdp_buffer: &vdp::RenderBuffer = genesis.get_framebuffer();
        
        // Converte para a janela (com upscaling 2x)
        for y in 0..GENESIS_HEIGHT {
            for x in 0..GENESIS_WIDTH {
                let vdp_pixel: u32 = vdp_buffer.pixels[y * GENESIS_WIDTH + x];
                
                // Upscale 2x
                let wx1: usize = x * 2;
                let wx2: usize = x * 2 + 1;
                let wy1: usize = y * 2;
                let wy2: usize = y * 2 + 1;
                
                if wx2 < window_width && wy2 < window_height {
                    buffer[wy1 * window_width + wx1] = vdp_pixel;
                    buffer[wy1 * window_width + wx2] = vdp_pixel;
                    buffer[wy2 * window_width + wx1] = vdp_pixel;
                    buffer[wy2 * window_width + wx2] = vdp_pixel;
                }
            }
        }
        
        // Atualiza a janela
        window.update_with_buffer(&buffer, window_width, window_height)
            .context("Failed to update window buffer")?;
        
        // Limita a 60 FPS
        let frame_time: Instant = Instant::now();
        let elapsed: Duration = frame_time.duration_since(last_frame_time);
        let target_frame_time: Duration = Duration::from_nanos(16_666_667); // 60 FPS
        
        if elapsed < target_frame_time {
            std::thread::sleep(target_frame_time - elapsed);
        }
        
        last_frame_time = Instant::now();
        
        // Verifica se a janela foi fechada
        if !window.is_open() {
            genesis.running = false;
        }
    }
    
    // Finalização
    println!();
    println!("═══════════════════════════════════════════════════");
    println!("Emulation stopped.");
    println!("Total frames: {}", genesis.frame_count);
    println!("Total cycles: {}M", genesis.total_cycles / 1_000_000);
    println!("Average FPS: {:.1}", genesis.current_fps);
    println!("═══════════════════════════════════════════════════");
    
    // Salva SRAM se existir
    if let Some(cartridge_info) = genesis.memory.get_cartridge_info() {
        if cartridge_info.ram_size_kb > 0 {
            println!("SRAM detected ({} KB). Saving...", cartridge_info.ram_size_kb);
            // Aqui você implementaria o save da SRAM
        }
    }
    
    println!("Goodbye!");
    Ok(())
}

// Implementação do assert_interrupt para M68000 (se não existir)
impl M68000 {
    pub fn assert_interrupt(&mut self, level: u8) {
        // Implementação simplificada de interrupção
        // Em uma implementação real, isso configuraria uma interrupção pendente
        debug!("Interrupt level {} asserted", level);
    }
}