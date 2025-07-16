// Licensed under the Apache-2.0 license

//! Caliptra MCU Emulator Library
//! 
//! This crate provides the core emulator functionality that can be used
//! by other programs to embed the Caliptra MCU emulator.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub mod dis;
pub mod doe_mbox_fsm;
pub mod elf;
pub mod emulator;
pub mod gdb;
pub mod i3c_socket;
pub mod mctp_transport;
pub mod mctp_util;

#[cfg(test)]
pub mod tests;

// Global state for runtime coordination
pub static MCU_RUNTIME_STARTED: AtomicBool = AtomicBool::new(false);
pub static EMULATOR_RUNNING: AtomicBool = AtomicBool::new(true);

pub fn wait_for_runtime_start() {
    while EMULATOR_RUNNING.load(Ordering::Relaxed) && !MCU_RUNTIME_STARTED.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(10));
    }
}

// Re-export the main types for convenience
pub use emulator::{Emulator, EmulatorArgs, SystemStepAction};

// Re-export commonly used types from dependencies
pub use caliptra_emu_bus::Bus;
pub use caliptra_emu_cpu::Cpu;
pub use caliptra_emu_types::RvAddr;
pub use emulator_consts::ROM_SIZE;
