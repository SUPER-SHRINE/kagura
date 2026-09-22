#![forbid(unsafe_code)]

//! Kagura 用の単純な RAM device。
//!
//! この device は指定されたサイズの連続した byte-addressed memory を提供する。
//! 8-bit / 16-bit / 32-bit の read/write を little-endian で受け付け、
//! 範囲外 access では `BusFault` を返す。
//!
//! 読み出し種別は区別せず、常に同じメモリ内容を返す。

use core::any::Any;
use kagura::{BusFault, Device};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ram {
    mem: Vec<u8>,
}

impl Ram {
    pub fn new(size: usize) -> Self {
        Self { mem: vec![0; size] }
    }

    pub fn len(&self) -> usize {
        self.mem.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mem.is_empty()
    }

    pub fn load8(&mut self, addr: u32, value: u8) -> Result<(), BusFault> {
        self.write8(addr, value)
    }

    pub fn load16(&mut self, addr: u32, value: u16) -> Result<(), BusFault> {
        self.write16(addr, value)
    }

    pub fn load32(&mut self, addr: u32, value: u32) -> Result<(), BusFault> {
        self.write32(addr, value)
    }

    pub fn read8_at(&self, addr: u32) -> Result<u8, BusFault> {
        self.mem.get(addr as usize).copied().ok_or(BusFault)
    }

    pub fn read16_at(&self, addr: u32) -> Result<u16, BusFault> {
        let base = addr as usize;
        let bytes = self.mem.get(base..base + 2).ok_or(BusFault)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read32_at(&self, addr: u32) -> Result<u32, BusFault> {
        let base = addr as usize;
        let bytes = self.mem.get(base..base + 4).ok_or(BusFault)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

impl Device for Ram {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn read8(&mut self, addr: u32) -> Result<u8, BusFault> {
        self.read8_at(addr)
    }

    fn read16(&mut self, addr: u32) -> Result<u16, BusFault> {
        let base = addr as usize;
        let bytes = self.mem.get(base..base + 2).ok_or(BusFault)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read32(&mut self, addr: u32) -> Result<u32, BusFault> {
        let base = addr as usize;
        let bytes = self.mem.get(base..base + 4).ok_or(BusFault)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusFault> {
        let slot = self.mem.get_mut(addr as usize).ok_or(BusFault)?;
        *slot = value;
        Ok(())
    }

    fn write16(&mut self, addr: u32, value: u16) -> Result<(), BusFault> {
        let base = addr as usize;
        let bytes = self.mem.get_mut(base..base + 2).ok_or(BusFault)?;
        bytes.copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write32(&mut self, addr: u32, value: u32) -> Result<(), BusFault> {
        let base = addr as usize;
        let bytes = self.mem.get_mut(base..base + 4).ok_or(BusFault)?;
        bytes.copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
}
