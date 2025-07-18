// Licensed under the Apache-2.0 license.
//
// generated by registers_generator with caliptra-ss repo at 39dd3917923c26515ca02c547807706f797428df
//
#[allow(unused_imports)]
use tock_registers::interfaces::{Readable, Writeable};
pub trait El2PicPeripheral {
    fn set_dma_ram(&mut self, _ram: std::rc::Rc<std::cell::RefCell<caliptra_emu_bus::Ram>>) {}
    fn set_dma_rom_sram(&mut self, _ram: std::rc::Rc<std::cell::RefCell<caliptra_emu_bus::Ram>>) {}
    fn register_event_channels(
        &mut self,
        _events_to_caliptra: std::sync::mpsc::Sender<caliptra_emu_bus::Event>,
        _events_from_caliptra: std::sync::mpsc::Receiver<caliptra_emu_bus::Event>,
        _events_to_mcu: std::sync::mpsc::Sender<caliptra_emu_bus::Event>,
        _events_from_mcu: std::sync::mpsc::Receiver<caliptra_emu_bus::Event>,
    ) {
    }
    fn poll(&mut self) {}
    fn warm_reset(&mut self) {}
    fn update_reset(&mut self) {}
    fn read_meipl(
        &mut self,
        _index: usize,
    ) -> caliptra_emu_bus::ReadWriteRegister<
        u32,
        registers_generated::el2_pic_ctrl::bits::Meipl::Register,
    > {
        caliptra_emu_bus::ReadWriteRegister::new(0)
    }
    fn write_meipl(
        &mut self,
        _val: caliptra_emu_bus::ReadWriteRegister<
            u32,
            registers_generated::el2_pic_ctrl::bits::Meipl::Register,
        >,
        _index: usize,
    ) {
    }
    fn read_meip(
        &mut self,
        _index: usize,
    ) -> caliptra_emu_bus::ReadWriteRegister<
        u32,
        registers_generated::el2_pic_ctrl::bits::Meip::Register,
    > {
        caliptra_emu_bus::ReadWriteRegister::new(0)
    }
    fn read_meie(
        &mut self,
        _index: usize,
    ) -> caliptra_emu_bus::ReadWriteRegister<
        u32,
        registers_generated::el2_pic_ctrl::bits::Meie::Register,
    > {
        caliptra_emu_bus::ReadWriteRegister::new(0)
    }
    fn write_meie(
        &mut self,
        _val: caliptra_emu_bus::ReadWriteRegister<
            u32,
            registers_generated::el2_pic_ctrl::bits::Meie::Register,
        >,
        _index: usize,
    ) {
    }
    fn read_mpiccfg(
        &mut self,
    ) -> caliptra_emu_bus::ReadWriteRegister<
        u32,
        registers_generated::el2_pic_ctrl::bits::Mpiccfg::Register,
    > {
        caliptra_emu_bus::ReadWriteRegister::new(0)
    }
    fn write_mpiccfg(
        &mut self,
        _val: caliptra_emu_bus::ReadWriteRegister<
            u32,
            registers_generated::el2_pic_ctrl::bits::Mpiccfg::Register,
        >,
    ) {
    }
    fn read_meigwctrl(
        &mut self,
        _index: usize,
    ) -> caliptra_emu_bus::ReadWriteRegister<
        u32,
        registers_generated::el2_pic_ctrl::bits::Meigwctrl::Register,
    > {
        caliptra_emu_bus::ReadWriteRegister::new(0)
    }
    fn write_meigwctrl(
        &mut self,
        _val: caliptra_emu_bus::ReadWriteRegister<
            u32,
            registers_generated::el2_pic_ctrl::bits::Meigwctrl::Register,
        >,
        _index: usize,
    ) {
    }
    fn read_meigwclr(&mut self, _index: usize) -> caliptra_emu_types::RvData {
        0
    }
    fn write_meigwclr(&mut self, _val: caliptra_emu_types::RvData, _index: usize) {}
}
pub struct El2PicBus {
    pub periph: Box<dyn El2PicPeripheral>,
}
impl caliptra_emu_bus::Bus for El2PicBus {
    fn read(
        &mut self,
        size: caliptra_emu_types::RvSize,
        addr: caliptra_emu_types::RvAddr,
    ) -> Result<caliptra_emu_types::RvData, caliptra_emu_bus::BusError> {
        if addr & 0x3 != 0 || size != caliptra_emu_types::RvSize::Word {
            return Err(caliptra_emu_bus::BusError::LoadAddrMisaligned);
        }
        match addr {
            0..0x400 => Ok(caliptra_emu_types::RvData::from(
                self.periph.read_meipl(addr as usize / 4).reg.get(),
            )),
            0x1000..0x1400 => Ok(caliptra_emu_types::RvData::from(
                self.periph
                    .read_meip((addr as usize - 0x1000) / 4)
                    .reg
                    .get(),
            )),
            0x2000..0x2400 => Ok(caliptra_emu_types::RvData::from(
                self.periph
                    .read_meie((addr as usize - 0x2000) / 4)
                    .reg
                    .get(),
            )),
            0x3000..0x3004 => Ok(caliptra_emu_types::RvData::from(
                self.periph.read_mpiccfg().reg.get(),
            )),
            0x4000..0x4400 => Ok(caliptra_emu_types::RvData::from(
                self.periph
                    .read_meigwctrl((addr as usize - 0x4000) / 4)
                    .reg
                    .get(),
            )),
            0x5000..0x5400 => Ok(self.periph.read_meigwclr((addr as usize - 0x5000) / 4)),
            _ => Err(caliptra_emu_bus::BusError::LoadAccessFault),
        }
    }
    fn write(
        &mut self,
        size: caliptra_emu_types::RvSize,
        addr: caliptra_emu_types::RvAddr,
        val: caliptra_emu_types::RvData,
    ) -> Result<(), caliptra_emu_bus::BusError> {
        if addr & 0x3 != 0 || size != caliptra_emu_types::RvSize::Word {
            return Err(caliptra_emu_bus::BusError::StoreAddrMisaligned);
        }
        match addr {
            0..0x400 => {
                self.periph.write_meipl(
                    caliptra_emu_bus::ReadWriteRegister::new(val),
                    addr as usize / 4,
                );
                Ok(())
            }
            0x2000..0x2400 => {
                self.periph.write_meie(
                    caliptra_emu_bus::ReadWriteRegister::new(val),
                    (addr as usize - 0x2000) / 4,
                );
                Ok(())
            }
            0x3000..0x3004 => {
                self.periph
                    .write_mpiccfg(caliptra_emu_bus::ReadWriteRegister::new(val));
                Ok(())
            }
            0x4000..0x4400 => {
                self.periph.write_meigwctrl(
                    caliptra_emu_bus::ReadWriteRegister::new(val),
                    (addr as usize - 0x4000) / 4,
                );
                Ok(())
            }
            0x5000..0x5400 => {
                self.periph
                    .write_meigwclr(val, (addr as usize - 0x5000) / 4);
                Ok(())
            }
            _ => Err(caliptra_emu_bus::BusError::StoreAccessFault),
        }
    }
    fn poll(&mut self) {
        self.periph.poll();
    }
    fn warm_reset(&mut self) {
        self.periph.warm_reset();
    }
    fn update_reset(&mut self) {
        self.periph.update_reset();
    }
}
