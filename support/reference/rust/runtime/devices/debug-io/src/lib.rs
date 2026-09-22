#![forbid(unsafe_code)]

//! Kagura 用の最小構成 debug I/O device。
//!
//! この device は 8-bit 幅の入出力を提供する簡易 MMIO ポートであり、
//! デバッグ実行やサンプルプログラムの標準入出力相当として使うことを想定している。
//!
//! レジスタ配置:
//! - `0x00` (`DATA`): read で 1 byte 入力を取り出し、write で 1 byte 出力を追記する
//! - `0x04` (`STATUS`): bit0 = 入力が存在する, bit1 = 出力可能
//!
//! 16-bit / 32-bit access は受け付けず、常に `BusFault` を返す。

use core::any::Any;
use kagura::{BusFault, Device};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DebugIo {
    input: Vec<u8>,
    output: Vec<u8>,
}

impl DebugIo {
    pub const DATA: u32 = 0x00;
    pub const STATUS: u32 = 0x04;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_input(&mut self, value: u8) {
        self.input.push(value);
    }

    pub fn output(&self) -> &[u8] {
        &self.output
    }

    pub fn take_output(&mut self) -> Vec<u8> {
        core::mem::take(&mut self.output)
    }

    fn status(&self) -> u8 {
        let has_input = !self.input.is_empty();
        let can_write = true;
        (has_input as u8) | ((can_write as u8) << 1)
    }
}

impl Device for DebugIo {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn read8(&mut self, addr: u32) -> Result<u8, BusFault> {
        match addr {
            Self::DATA => Ok(if self.input.is_empty() {
                0
            } else {
                self.input.remove(0)
            }),
            Self::STATUS => Ok(self.status()),
            _ => Err(BusFault),
        }
    }

    fn read16(&mut self, _addr: u32) -> Result<u16, BusFault> {
        Err(BusFault)
    }

    fn read32(&mut self, _addr: u32) -> Result<u32, BusFault> {
        Err(BusFault)
    }

    fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusFault> {
        match addr {
            Self::DATA => {
                self.output.push(value);
                Ok(())
            }
            _ => Err(BusFault),
        }
    }

    fn write16(&mut self, _addr: u32, _value: u16) -> Result<(), BusFault> {
        Err(BusFault)
    }

    fn write32(&mut self, _addr: u32, _value: u32) -> Result<(), BusFault> {
        Err(BusFault)
    }
}
