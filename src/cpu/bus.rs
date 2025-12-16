//! Interface de bus de memória para CPU

pub trait CpuBus {
    fn read_byte(&mut self, address: u32) -> anyhow::Result<u8>;
    fn read_word(&mut self, address: u32) -> anyhow::Result<u16>;
    fn read_long(&mut self, address: u32) -> anyhow::Result<u32>;

    fn write_byte(&mut self, address: u32, value: u8) -> anyhow::Result<()>;
    fn write_word(&mut self, address: u32, value: u16) -> anyhow::Result<()>;
    fn write_long(&mut self, address: u32, value: u32) -> anyhow::Result<()>;
}
