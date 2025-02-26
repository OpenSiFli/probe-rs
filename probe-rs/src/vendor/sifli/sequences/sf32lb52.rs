use crate::architecture::arm::ArmError;
use crate::architecture::arm::armv8m;
use crate::architecture::arm::armv8m::Dhcsr;
use crate::architecture::arm::memory::ArmMemoryInterface;
use crate::architecture::arm::sequences::ArmDebugSequence;
use crate::probe::DebugProbeError;
use crate::vendor::st::sequences::stm32_armv6::Stm32Armv6Family;
use probe_rs_target::CoreType;
use std::sync::Arc;
use crate::MemoryMappedRegister;

#[derive(Debug)]
pub struct Sf32lb52 {}

impl Sf32lb52 {
    pub fn create() -> Arc<Self> {
        Arc::new(Self {})
    }
}

mod pmuc {
    use crate::architecture::arm::{ArmError, memory::ArmMemoryInterface};
    use bitfield::bitfield;

    /// The base address of the PMUC component
    const PMUC: u64 = 0x500C_A000;

    bitfield! {
        /// The control register (CR) of the PMUC.
        pub struct Control(u32);
        impl Debug;

        // [19:15] PIN1_SEL 占 5 位，默认值应设置为 1
        pub u8, pin1_sel, set_pin1_sel: 19, 15;

        // [14:10] PIN0_SEL 占 5 位，默认值为 0
        pub u8, pin0_sel, set_pin0_sel: 14, 10;

        // [9:7] PIN1_MODE 占 3 位，默认值为 0
        pub u8, pin1_mode, set_pin1_mode: 9, 7;

        // [6:4] PIN0_MODE 占 3 位，默认值为 0
        pub u8, pin0_mode, set_pin0_mode: 6, 4;

        // [3] PIN_RET 占 1 位，默认值为 0
        pub bool, pin_ret, set_pin_ret: 3;

        // [2] REBOOT 占 1 位，默认值为 0
        pub bool, reboot, set_reboot: 2;

        // [1] HIBER_EN 占 1 位，默认值为 0
        pub bool, hiber_en, set_hiber_en: 1;

        // [0] SEL_LPCLK 占 1 位，默认值为 0
        pub bool, sel_lpclk, set_sel_lpclk: 0;
    }

    impl Control {
        /// The offset of the Control register in the PMUC block.
        const ADDRESS: u64 = 0x00;

        /// Read the control register from memory.
        pub fn read(memory: &mut dyn ArmMemoryInterface) -> Result<Self, ArmError> {
            let contents = memory.read_word_32(PMUC + Self::ADDRESS)?;
            Ok(Self(contents))
        }

        /// Write the control register to memory.
        pub fn write(&mut self, memory: &mut dyn ArmMemoryInterface) -> Result<(), ArmError> {
            memory.write_word_32(PMUC + Self::ADDRESS, self.0)
        }
    }
}

fn halt_core(interface: &mut dyn ArmMemoryInterface) -> Result<(), ArmError> {
    let mut value = Dhcsr(0);
    value.set_c_halt(true);
    value.set_c_debugen(true);
    value.enable_write();

    interface.write_word_32(Dhcsr::get_mmio_address(), value.into())?;
    Ok(())
}

impl ArmDebugSequence for Sf32lb52 {
    fn reset_system(
        &self,
        interface: &mut dyn ArmMemoryInterface,
        core_type: CoreType,
        debug_base: Option<u64>,
    ) -> Result<(), ArmError> {
        let mut pmuc = pmuc::Control::read(interface)?;
        pmuc.set_reboot(true);
        let _ = pmuc.write(interface); // 我们不在意这个错误
        // 等待500ms重新启动
        std::thread::sleep(std::time::Duration::from_millis(500));
        interface.update_core_status(crate::CoreStatus::Unknown);

        // Halt 住CPU
        halt_core(interface)?;
        Ok(())
    }
}
