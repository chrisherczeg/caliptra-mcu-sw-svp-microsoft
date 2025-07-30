/*++

Licensed under the Apache-2.0 license.

File Name:

    lib.rs

Abstract:

    File contains exports for for Caliptra Emulator Peripheral library.

--*/
mod dma_ctrl;
mod doe_mbox;
mod emu_ctrl;
mod flash_ctrl;
mod i3c;
pub(crate) mod i3c_protocol;
mod lc_ctrl;
mod mci;
mod otp;
mod otp_digest;
mod reset_reason;
mod root_bus;
mod spi_flash;
mod spi_host;
mod uart;

pub use dma_ctrl::DummyDmaCtrl;
pub use doe_mbox::{DoeMboxPeriph, DummyDoeMbox};
pub use emu_ctrl::EmuCtrl;
pub use flash_ctrl::DummyFlashCtrl;
pub use i3c::I3c;
pub use i3c_protocol::*;
pub use lc_ctrl::LcCtrl;
pub use mci::Mci;
pub use otp::Otp;
pub use reset_reason::ResetReasonEmulator;
pub use root_bus::{McuRootBus, McuRootBusArgs, McuRootBusOffsets};
pub use spi_flash::IoMode;
pub use uart::Uart;
