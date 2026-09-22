#![forbid(unsafe_code)]

//! Reporter device used by the Kagura conformance test machine.

use core::any::Any;
use kagura::{BusFault, Device};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Running,
    Pass,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReporterState {
    pub status: u32,
    pub case_id: u32,
    pub detail: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TestReporter {
    state: ReporterState,
}

impl TestReporter {
    pub const STATUS: u32 = 0x00;
    pub const TEST_ID: u32 = 0x04;
    pub const DETAIL: u32 = 0x08;
    pub const RESERVED: u32 = 0x0c;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn status(&self) -> TestStatus {
        match self.state.status {
            1 => TestStatus::Pass,
            2 => TestStatus::Fail,
            _ => TestStatus::Running,
        }
    }

    pub fn status_raw(&self) -> u32 {
        self.state.status
    }

    pub fn test_id(&self) -> u32 {
        self.state.case_id
    }

    pub fn detail(&self) -> u32 {
        self.state.detail
    }

    pub fn snapshot(&self) -> ReporterState {
        self.state
    }

    /// Host-only setup operation. It is not a bus transaction.
    pub fn restore(&mut self, state: ReporterState) {
        self.state = state;
    }
}

impl Device for TestReporter {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn read8(&mut self, _addr: u32) -> Result<u8, BusFault> {
        Err(BusFault)
    }

    fn read16(&mut self, _addr: u32) -> Result<u16, BusFault> {
        Err(BusFault)
    }

    fn read32(&mut self, _addr: u32) -> Result<u32, BusFault> {
        Err(BusFault)
    }

    fn write8(&mut self, _addr: u32, _value: u8) -> Result<(), BusFault> {
        Err(BusFault)
    }

    fn write16(&mut self, _addr: u32, _value: u16) -> Result<(), BusFault> {
        Err(BusFault)
    }

    fn write32(&mut self, addr: u32, value: u32) -> Result<(), BusFault> {
        match addr {
            Self::STATUS => self.state.status = value,
            Self::TEST_ID => self.state.case_id = value,
            Self::DETAIL => self.state.detail = value,
            Self::RESERVED => return Err(BusFault),
            _ => return Err(BusFault),
        }
        Ok(())
    }
}
