/*++

Licensed under the Apache-2.0 license.

File Name:

    main.rs

Abstract:

    File contains main entrypoint for Caliptra MCU Emulator.

--*/

mod dis;
mod dis_test;
mod doe_mbox_fsm;
mod elf;
mod emulator;
mod gdb;
mod i3c_socket;
mod mctp_transport;
mod tests;

use clap::Parser;
use std::cell::RefCell;
use std::io;
use std::io::IsTerminal;
use std::process::exit;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::emulator::EmulatorArgs;

pub static MCU_RUNTIME_STARTED: AtomicBool = AtomicBool::new(false);
pub static EMULATOR_RUNNING: AtomicBool = AtomicBool::new(true);

pub fn wait_for_runtime_start() {
    while EMULATOR_RUNNING.load(Ordering::Relaxed) && !MCU_RUNTIME_STARTED.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(10));
    }
}

// CPU Main Loop (free_run no GDB)
fn free_run(mut emulator: crate::emulator::Emulator) {
    while EMULATOR_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        if !emulator.step() {
            break;
        }
    }
}

fn main() -> io::Result<()> {
    let cli = EmulatorArgs::parse();
    run(cli, false).map(|_| ())
}

fn run(cli: EmulatorArgs, capture_uart_output: bool) -> io::Result<Vec<u8>> {
    // exit cleanly on Ctrl-C so that we save any state.
    if io::stdout().is_terminal() {
        ctrlc::set_handler(move || {
            EMULATOR_RUNNING.store(false, std::sync::atomic::Ordering::Relaxed);
        })
        .unwrap();
    }

    let uart_output = if capture_uart_output {
        Some(Rc::new(RefCell::new(Vec::new())))
    } else {
        None
    };

    // Check if Optional GDB Port is passed
    match cli.gdb_port {
        Some(_port) => {
            println!("Caliptra CPU cannot be started with GDB enabled");
            exit(-1);
        }
        _ => {
            // Create the emulator with all the setup
            let emulator = crate::emulator::Emulator::from_args(cli, capture_uart_output)?;
            free_run(emulator);
        }
    }

    Ok(uart_output.map(|o| o.borrow().clone()).unwrap_or_default())
}
