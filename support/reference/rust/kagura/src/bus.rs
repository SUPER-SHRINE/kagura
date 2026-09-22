use crate::device::Device;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessWidth {
    Byte,
    Half,
    Word,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusFault;

pub trait Bus {
    fn read8(&mut self, addr: u32) -> Result<u8, BusFault>;
    fn read16(&mut self, addr: u32) -> Result<u16, BusFault>;
    fn read32(&mut self, addr: u32) -> Result<u32, BusFault>;

    fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusFault>;
    fn write16(&mut self, addr: u32, value: u16) -> Result<(), BusFault>;
    fn write32(&mut self, addr: u32, value: u32) -> Result<(), BusFault>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    EmptyRange,
    AddressOverflow,
    Overlap,
}

pub struct DefaultBus {
    mappings: Vec<DeviceMapping>,
}

impl Default for DefaultBus {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultBus {
    pub fn new() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }

    pub fn map_device<D>(&mut self, base: u32, size: u32, device: D) -> Result<(), MapError>
    where
        D: Device + 'static,
    {
        if size == 0 {
            return Err(MapError::EmptyRange);
        }

        let end = u64::from(base) + u64::from(size);
        if end > u64::from(u32::MAX) + 1 {
            return Err(MapError::AddressOverflow);
        }
        for mapping in &self.mappings {
            if u64::from(base) < mapping.end && u64::from(mapping.base) < end {
                return Err(MapError::Overlap);
            }
        }

        self.mappings.push(DeviceMapping {
            base,
            end,
            device: Box::new(device),
        });
        Ok(())
    }

    pub fn device_ref<T: 'static>(&self, base: u32) -> Option<&T> {
        self.mappings
            .iter()
            .find(|mapping| mapping.base == base)
            .and_then(|mapping| mapping.device.as_any().downcast_ref::<T>())
    }

    pub fn device_mut<T: 'static>(&mut self, base: u32) -> Option<&mut T> {
        self.mappings
            .iter_mut()
            .find(|mapping| mapping.base == base)
            .and_then(|mapping| mapping.device.as_any_mut().downcast_mut::<T>())
    }

    fn find_device_mut(&mut self, addr: u32) -> Option<(&mut dyn Device, u32)> {
        for mapping in &mut self.mappings {
            if mapping.contains(addr) {
                let offset = addr - mapping.base;
                return Some((mapping.device.as_mut(), offset));
            }
        }
        None
    }

    pub fn read8_at(&mut self, addr: u32) -> Result<u8, BusFault> {
        let (device, offset) = self.find_device_mut(addr).ok_or(BusFault)?;
        device.read8(offset)
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

    pub fn read16_at(&mut self, addr: u32) -> Result<u16, BusFault> {
        let (device, offset) = self.find_device_mut(addr).ok_or(BusFault)?;
        device.read16(offset)
    }

    pub fn read32_at(&mut self, addr: u32) -> Result<u32, BusFault> {
        let (device, offset) = self.find_device_mut(addr).ok_or(BusFault)?;
        device.read32(offset)
    }
}

impl Bus for DefaultBus {
    fn read8(&mut self, addr: u32) -> Result<u8, BusFault> {
        let (device, offset) = self.find_device_mut(addr).ok_or(BusFault)?;
        device.read8(offset)
    }

    fn read16(&mut self, addr: u32) -> Result<u16, BusFault> {
        let (device, offset) = self.find_device_mut(addr).ok_or(BusFault)?;
        device.read16(offset)
    }

    fn read32(&mut self, addr: u32) -> Result<u32, BusFault> {
        let (device, offset) = self.find_device_mut(addr).ok_or(BusFault)?;
        device.read32(offset)
    }

    fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusFault> {
        let (device, offset) = self.find_device_mut(addr).ok_or(BusFault)?;
        device.write8(offset, value)
    }

    fn write16(&mut self, addr: u32, value: u16) -> Result<(), BusFault> {
        let (device, offset) = self.find_device_mut(addr).ok_or(BusFault)?;
        device.write16(offset, value)
    }

    fn write32(&mut self, addr: u32, value: u32) -> Result<(), BusFault> {
        let (device, offset) = self.find_device_mut(addr).ok_or(BusFault)?;
        device.write32(offset, value)
    }
}

struct DeviceMapping {
    base: u32,
    end: u64,
    device: Box<dyn Device>,
}

impl DeviceMapping {
    fn contains(&self, addr: u32) -> bool {
        self.base <= addr && u64::from(addr) < self.end
    }
}
