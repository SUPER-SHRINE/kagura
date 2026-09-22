#![forbid(unsafe_code)]

//! CPU 検証用の最小構成マシン。
//!
//! `test-runner` は `kagura` と各種 device を既定の memory map で束ねる。
//! アセンブリ source text は扱わず、機械語ワード列を実行する machine として振る舞う。

use debug_io::DebugIo;
use kagura::{BusFault, Cpu, DefaultBus, Fault};
use ram::Ram;
use test_reporter::{TestReporter, TestStatus};

pub const RAM_BASE: u32 = 0x0000;
pub const RAM_SIZE: u32 = 0x2000;
pub const DEBUG_IO_BASE: u32 = 0x2000;
pub const DEBUG_IO_SIZE: u32 = 0x0010;
pub const TEST_REPORTER_BASE: u32 = 0x2010;
pub const TEST_REPORTER_SIZE: u32 = 0x0010;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineStatus {
    Running,
    Passed,
    Failed { test_id: u32, detail: u32 },
}

pub struct TestRunnerMachine {
    cpu: Cpu,
    bus: DefaultBus,
}

impl Default for TestRunnerMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl TestRunnerMachine {
    pub fn new() -> Self {
        let mut bus = DefaultBus::new();
        bus.map_device(RAM_BASE, RAM_SIZE, Ram::new(RAM_SIZE as usize))
            .expect("RAM mapping must be valid");
        bus.map_device(DEBUG_IO_BASE, DEBUG_IO_SIZE, DebugIo::new())
            .expect("DebugIo mapping must be valid");
        bus.map_device(TEST_REPORTER_BASE, TEST_REPORTER_SIZE, TestReporter::new())
            .expect("TestReporter mapping must be valid");

        Self {
            cpu: Cpu::new(),
            bus,
        }
    }

    pub fn pc(&self) -> u32 {
        self.cpu.pc()
    }

    pub fn reg(&self, index: usize) -> u32 {
        self.cpu.reg(index)
    }

    pub fn set_pc(&mut self, pc: u32) {
        self.cpu.set_pc(pc);
    }

    pub fn set_reg(&mut self, index: usize, value: u32) {
        self.cpu.set_reg(index, value);
    }

    pub fn load_words(&mut self, words: &[u32]) -> Result<(), BusFault> {
        for (i, word) in words.iter().copied().enumerate() {
            self.bus.load32((i as u32) * 4, word)?;
        }
        Ok(())
    }

    pub fn step(&mut self) -> Result<MachineStatus, Fault> {
        self.cpu.step(&mut self.bus)?;
        Ok(self.status())
    }

    pub fn run_steps(&mut self, max_steps: usize) -> Result<MachineStatus, Fault> {
        for _ in 0..max_steps {
            let status = self.step()?;
            if status != MachineStatus::Running {
                return Ok(status);
            }
        }
        Ok(MachineStatus::Running)
    }

    pub fn status(&self) -> MachineStatus {
        let reporter = self
            .bus
            .device_ref::<TestReporter>(TEST_REPORTER_BASE)
            .expect("TestReporter must be present");
        match reporter.status() {
            TestStatus::Running => MachineStatus::Running,
            TestStatus::Pass => MachineStatus::Passed,
            TestStatus::Fail => MachineStatus::Failed {
                test_id: reporter.test_id(),
                detail: reporter.detail(),
            },
        }
    }

    pub fn debug_output(&self) -> &[u8] {
        self.bus
            .device_ref::<DebugIo>(DEBUG_IO_BASE)
            .expect("DebugIo must be present")
            .output()
    }

    pub fn take_debug_output(&mut self) -> Vec<u8> {
        self.bus
            .device_mut::<DebugIo>(DEBUG_IO_BASE)
            .expect("DebugIo must be present")
            .take_output()
    }

    pub fn read8_at(&mut self, addr: u32) -> Result<u8, BusFault> {
        self.bus.read8_at(addr)
    }

    pub fn read32_at(&mut self, addr: u32) -> Result<u32, BusFault> {
        self.bus.read32_at(addr)
    }
}
