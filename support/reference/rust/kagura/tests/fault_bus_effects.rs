use kagura::{AccessOperation, AccessWidth, Bus, BusFault, Cpu, Fault};

struct FetchEffectBus {
    instruction: u32,
    successful_reads: u32,
    failed_data_accesses: u32,
    device_value: u32,
}

impl FetchEffectBus {
    fn data_fault<T>(&mut self) -> Result<T, BusFault> {
        self.failed_data_accesses += 1;
        Err(BusFault)
    }
}

impl Bus for FetchEffectBus {
    fn read8(&mut self, _: u32) -> Result<u8, BusFault> {
        self.data_fault()
    }

    fn read16(&mut self, _: u32) -> Result<u16, BusFault> {
        self.data_fault()
    }

    fn read32(&mut self, address: u32) -> Result<u32, BusFault> {
        if address != 0x40 {
            return self.data_fault();
        }
        self.successful_reads += 1;
        self.device_value += 1;
        Ok(self.instruction)
    }

    fn write8(&mut self, _: u32, _: u8) -> Result<(), BusFault> {
        self.data_fault()
    }

    fn write16(&mut self, _: u32, _: u16) -> Result<(), BusFault> {
        self.data_fault()
    }

    fn write32(&mut self, _: u32, _: u32) -> Result<(), BusFault> {
        self.data_fault()
    }
}

#[test]
fn faults_preserve_cpu_state_and_successful_fetch_device_effects() {
    for (instruction, expected, data_accesses) in [
        (
            0xF000_0000,
            Fault::InvalidInstruction { faulting_pc: 0x40 },
            0,
        ),
        (
            0xC503_0000,
            Fault::UnalignedPc {
                faulting_pc: 0x40,
                addr: 6,
            },
            0,
        ),
        (
            0x5210_0001,
            Fault::UnalignedAccess {
                faulting_pc: 0x40,
                addr: 0x81,
                width: AccessWidth::Half,
                operation: AccessOperation::Load,
            },
            0,
        ),
        (
            0x7210_0000,
            Fault::BusFault {
                faulting_pc: 0x40,
                addr: 0x80,
                width: AccessWidth::Word,
                operation: AccessOperation::Load,
            },
            1,
        ),
        (
            0xB210_0000,
            Fault::BusFault {
                faulting_pc: 0x40,
                addr: 0x80,
                width: AccessWidth::Word,
                operation: AccessOperation::Store,
            },
            1,
        ),
    ] {
        let mut cpu = Cpu::new();
        cpu.set_pc(0x40);
        for index in 1..16 {
            cpu.set_reg(index, index as u32 * 17);
        }
        cpu.set_reg(1, 0x80);
        cpu.set_reg(3, 6);
        let before = cpu.clone();
        let mut bus = FetchEffectBus {
            instruction,
            successful_reads: 0,
            failed_data_accesses: 0,
            device_value: 20,
        };

        assert_eq!(cpu.step(&mut bus), Err(expected));
        assert_eq!(cpu, before);
        assert_eq!(bus.successful_reads, 1);
        assert_eq!(bus.failed_data_accesses, data_accesses);
        assert_eq!(bus.device_value, 21);
    }
}
