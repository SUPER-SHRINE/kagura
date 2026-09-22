#![forbid(unsafe_code)]

use high_test_rom::HighTestRom;
use kagura::{Bus, BusFault, Cpu, DefaultBus, Fault};
use ram::Ram;
pub use test_reporter::ReporterState;
use test_reporter::TestReporter;

pub const RAM_BASE: u32 = 0x0000_0000;
pub const RAM_SIZE: u32 = 0x0001_0000;
pub const REPORTER_BASE: u32 = 0xffff_f000;
pub const REPORTER_SIZE: u32 = 0x10;
pub const HIGH_ROM_BASE: u32 = 0xffff_fffc;
pub const HIGH_ROM_SIZE: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusEvent {
    pub operation: &'static str,
    pub address: u32,
    pub width: u8,
    pub result: &'static str,
    pub value: Option<u32>,
}

pub struct ConformanceMachine {
    cpu: Cpu,
    bus: TracingBus,
}

impl ConformanceMachine {
    pub fn new() -> Result<Self, String> {
        let mut bus = DefaultBus::new();
        bus.map_device(RAM_BASE, RAM_SIZE, Ram::new(RAM_SIZE as usize))
            .map_err(|error| format!("cannot map Test RAM: {error:?}"))?;
        bus.map_device(REPORTER_BASE, REPORTER_SIZE, TestReporter::new())
            .map_err(|error| format!("cannot map Test Reporter: {error:?}"))?;
        bus.map_device(HIGH_ROM_BASE, HIGH_ROM_SIZE, HighTestRom::default())
            .map_err(|error| format!("cannot map High Test ROM: {error:?}"))?;
        Ok(Self {
            cpu: Cpu::new(),
            bus: TracingBus::new(bus),
        })
    }

    pub fn set_pc(&mut self, pc: u32) {
        self.cpu.set_pc(pc);
    }
    pub fn pc(&self) -> u32 {
        self.cpu.pc()
    }
    pub fn set_reg(&mut self, index: usize, value: u32) {
        self.cpu.set_reg(index, value);
    }
    pub fn reg(&self, index: usize) -> u32 {
        self.cpu.reg(index)
    }
    pub fn step(&mut self) -> Result<(), Fault> {
        self.cpu.step(&mut self.bus)
    }
    pub fn trace(&self) -> &[BusEvent] {
        &self.bus.trace
    }
    pub fn clear_trace(&mut self) {
        self.bus.trace.clear();
    }

    pub fn load_ram(&mut self, bytes: &[u8]) -> Result<(), BusFault> {
        if bytes.len() > RAM_SIZE as usize {
            return Err(BusFault);
        }
        for (offset, value) in bytes.iter().copied().enumerate() {
            self.ram_mut().load8(offset as u32, value)?;
        }
        Ok(())
    }

    pub fn patch_memory(&mut self, address: u32, bytes: &[u8]) -> Result<(), BusFault> {
        if u64::from(address) + bytes.len() as u64 > u64::from(RAM_SIZE) {
            return Err(BusFault);
        }
        for (offset, value) in bytes.iter().copied().enumerate() {
            self.ram_mut().load8(address + offset as u32, value)?;
        }
        Ok(())
    }

    pub fn read_memory(&self, address: u32, length: usize) -> Result<Vec<u8>, BusFault> {
        if u64::from(address) + length as u64 > u64::from(RAM_SIZE) {
            return Err(BusFault);
        }
        (0..length)
            .map(|offset| self.ram().read8_at(address + offset as u32))
            .collect()
    }

    pub fn reporter(&self) -> ReporterState {
        self.reporter_ref().snapshot()
    }
    pub fn restore_reporter(&mut self, state: ReporterState) {
        self.reporter_mut().restore(state);
    }
    pub fn high_rom_word(&self) -> u32 {
        self.high_rom().word()
    }
    pub fn set_high_rom_word(&mut self, word: u32) {
        self.high_rom_mut().set_word(word);
    }

    pub fn perform(
        &mut self,
        operation: &str,
        address: u32,
        width: u8,
        value: Option<u32>,
    ) -> Result<BusEvent, String> {
        self.bus.perform(operation, address, width, value)
    }

    fn ram(&self) -> &Ram {
        self.bus.inner.device_ref(RAM_BASE).expect("RAM mapped")
    }
    fn ram_mut(&mut self) -> &mut Ram {
        self.bus.inner.device_mut(RAM_BASE).expect("RAM mapped")
    }
    fn reporter_ref(&self) -> &TestReporter {
        self.bus
            .inner
            .device_ref(REPORTER_BASE)
            .expect("Reporter mapped")
    }
    fn reporter_mut(&mut self) -> &mut TestReporter {
        self.bus
            .inner
            .device_mut(REPORTER_BASE)
            .expect("Reporter mapped")
    }
    fn high_rom(&self) -> &HighTestRom {
        self.bus
            .inner
            .device_ref(HIGH_ROM_BASE)
            .expect("High ROM mapped")
    }
    fn high_rom_mut(&mut self) -> &mut HighTestRom {
        self.bus
            .inner
            .device_mut(HIGH_ROM_BASE)
            .expect("High ROM mapped")
    }
}

struct TracingBus {
    inner: DefaultBus,
    trace: Vec<BusEvent>,
}

impl TracingBus {
    fn new(inner: DefaultBus) -> Self {
        Self {
            inner,
            trace: Vec::new(),
        }
    }

    fn perform(
        &mut self,
        operation: &str,
        address: u32,
        width: u8,
        value: Option<u32>,
    ) -> Result<BusEvent, String> {
        let result = match (operation, width) {
            ("read", 8) => self.inner.read8(address).map(u32::from),
            ("read", 16) => self.inner.read16(address).map(u32::from),
            ("read", 32) => self.inner.read32(address),
            ("write", 8) => self
                .inner
                .write8(address, value.ok_or("write value missing")? as u8)
                .map(|_| 0),
            ("write", 16) => self
                .inner
                .write16(address, value.ok_or("write value missing")? as u16)
                .map(|_| 0),
            ("write", 32) => self
                .inner
                .write32(address, value.ok_or("write value missing")?)
                .map(|_| 0),
            _ => return Err("invalid transaction".into()),
        };
        Ok(BusEvent {
            operation: if operation == "read" { "read" } else { "write" },
            address,
            width,
            result: if result.is_ok() { "ok" } else { "fault" },
            value: if operation == "read" {
                result.ok()
            } else {
                None
            },
        })
    }

    fn record_read<T: Copy + Into<u32>>(
        &mut self,
        address: u32,
        width: u8,
        result: Result<T, BusFault>,
    ) -> Result<T, BusFault> {
        self.trace.push(BusEvent {
            operation: "read",
            address,
            width,
            result: if result.is_ok() { "ok" } else { "fault" },
            value: result.as_ref().ok().map(|value| (*value).into()),
        });
        result
    }

    fn record_write(
        &mut self,
        address: u32,
        width: u8,
        value: u32,
        result: Result<(), BusFault>,
    ) -> Result<(), BusFault> {
        self.trace.push(BusEvent {
            operation: "write",
            address,
            width,
            result: if result.is_ok() { "ok" } else { "fault" },
            value: Some(value),
        });
        result
    }
}

impl Bus for TracingBus {
    fn read8(&mut self, addr: u32) -> Result<u8, BusFault> {
        let result = self.inner.read8(addr);
        self.record_read(addr, 8, result)
    }
    fn read16(&mut self, addr: u32) -> Result<u16, BusFault> {
        let result = self.inner.read16(addr);
        self.record_read(addr, 16, result)
    }
    fn read32(&mut self, addr: u32) -> Result<u32, BusFault> {
        let result = self.inner.read32(addr);
        self.record_read(addr, 32, result)
    }
    fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusFault> {
        let result = self.inner.write8(addr, value);
        self.record_write(addr, 8, value.into(), result)
    }
    fn write16(&mut self, addr: u32, value: u16) -> Result<(), BusFault> {
        let result = self.inner.write16(addr, value);
        self.record_write(addr, 16, value.into(), result)
    }
    fn write32(&mut self, addr: u32, value: u32) -> Result<(), BusFault> {
        let result = self.inner.write32(addr, value);
        self.record_write(addr, 32, value, result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_high_rom_at_top_of_address_space() {
        let mut machine = ConformanceMachine::new().unwrap();
        machine.set_high_rom_word(0x1122_3344);
        assert_eq!(
            machine
                .perform("read", HIGH_ROM_BASE, 32, None)
                .unwrap()
                .value,
            Some(0x1122_3344)
        );
    }
}
