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

use clap::{ArgAction, Parser};
use clap_num::maybe_hex;
use crossterm::event::{Event, KeyCode, KeyEvent};
use std::cell::RefCell;
use std::fs::File;
use std::io;
use std::io::{IsTerminal, Read};
use std::path::PathBuf;
use std::process::exit;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub static MCU_RUNTIME_STARTED: AtomicBool = AtomicBool::new(false);
pub static EMULATOR_RUNNING: AtomicBool = AtomicBool::new(true);

pub fn wait_for_runtime_start() {
    while EMULATOR_RUNNING.load(Ordering::Relaxed) && !MCU_RUNTIME_STARTED.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None, name = "Caliptra MCU Emulator")]
struct Args {
    /// ROM binary path
    #[arg(short, long)]
    rom: PathBuf,

    #[arg(short, long)]
    firmware: PathBuf,

    /// Optional file to store OTP / fuses between runs.
    #[arg(short, long)]
    otp: Option<PathBuf>,

    /// GDB Debugger Port
    #[arg(short, long)]
    gdb_port: Option<u16>,

    /// Directory in which to log execution artifacts.
    #[arg(short, long)]
    log_dir: Option<PathBuf>,

    /// Trace instructions.
    #[arg(short, long, default_value_t = false)]
    trace_instr: bool,

    // These look backwards, but this is necessary so that the default is to capture stdin.
    /// Pass stdin to the MCU UART Rx.
    #[arg(long = "no-stdin-uart", action = ArgAction::SetFalse)]
    stdin_uart: bool,

    // this is used only to set stdin_uart to false
    #[arg(long = "stdin-uart", overrides_with = "stdin_uart")]
    _no_stdin_uart: bool,

    /// The ROM path for the Caliptra CPU.
    #[arg(long)]
    caliptra_rom: PathBuf,

    /// The Firmware path for the Caliptra CPU.
    #[arg(long)]
    caliptra_firmware: PathBuf,

    #[arg(long)]
    soc_manifest: PathBuf,

    #[arg(long)]
    i3c_port: Option<u16>,

    /// This is only needed if the IDevID CSR needed to be generated in the Caliptra Core.
    #[arg(long)]
    manufacturing_mode: bool,

    #[arg(long)]
    vendor_pk_hash: Option<String>,

    #[arg(long)]
    owner_pk_hash: Option<String>,

    /// Path to the streaming boot PLDM firmware package
    #[arg(long)]
    streaming_boot: Option<PathBuf>,

    #[arg(long)]
    primary_flash_image: Option<PathBuf>,

    #[arg(long)]
    secondary_flash_image: Option<PathBuf>,

    /// HW revision in semver format (e.g., "2.0.0")
    #[arg(long, value_parser = semver::Version::parse, default_value = "2.0.0")]
    hw_revision: semver::Version,

    /// Override ROM offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    rom_offset: Option<u32>,
    /// Override ROM size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    rom_size: Option<u32>,
    /// Override UART offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    uart_offset: Option<u32>,
    /// Override UART size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    uart_size: Option<u32>,
    /// Override emulator control offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    ctrl_offset: Option<u32>,
    /// Override emulator control size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    ctrl_size: Option<u32>,
    /// Override SPI offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    spi_offset: Option<u32>,
    /// Override SPI size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    spi_size: Option<u32>,
    /// Override SRAM offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    sram_offset: Option<u32>,
    /// Override SRAM size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    sram_size: Option<u32>,
    /// Override PIC offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pic_offset: Option<u32>,
    /// Override external test SRAM offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    external_test_sram_offset: Option<u32>,
    /// Override external test SRAM size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    external_test_sram_size: Option<u32>,
    /// Override DCCM offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    dccm_offset: Option<u32>,
    /// Override DCCM size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    dccm_size: Option<u32>,
    /// Override I3C offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    i3c_offset: Option<u32>,
    /// Override I3C size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    i3c_size: Option<u32>,
    /// Override primary flash offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    primary_flash_offset: Option<u32>,
    /// Override primary flash size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    primary_flash_size: Option<u32>,
    /// Override secondary flash offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    secondary_flash_offset: Option<u32>,
    /// Override secondary flash size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    secondary_flash_size: Option<u32>,
    /// Override MCI offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    mci_offset: Option<u32>,
    /// Override MCI size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    mci_size: Option<u32>,
    /// Override DMA offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    dma_offset: Option<u32>,
    /// Override DMA size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    dma_size: Option<u32>,
    /// Override Caliptra mailbox offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    mbox_offset: Option<u32>,
    /// Override Caliptra mailbox size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    mbox_size: Option<u32>,
    /// Override Caliptra SoC interface offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    soc_offset: Option<u32>,
    /// Override Caliptra SoC interface size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    soc_size: Option<u32>,
    /// Override OTP offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    otp_offset: Option<u32>,
    /// Override OTP size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    otp_size: Option<u32>,
    /// Override LC offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    lc_offset: Option<u32>,
    /// Override LC size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    lc_size: Option<u32>,
}

fn read_console(stdin_uart: Option<Arc<Mutex<Option<u8>>>>) {
    let mut buffer = vec![];
    if let Some(ref stdin_uart) = stdin_uart {
        while EMULATOR_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
            if buffer.is_empty() {
                match crossterm::event::read() {
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Char(ch),
                        ..
                    })) => {
                        buffer.extend_from_slice(ch.to_string().as_bytes());
                    }
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Enter,
                        ..
                    })) => {
                        buffer.push(b'\n');
                    }
                    Ok(Event::Key(KeyEvent {
                        code: KeyCode::Backspace,
                        ..
                    })) => {
                        if !buffer.is_empty() {
                            buffer.pop();
                        } else {
                            buffer.push(8);
                        }
                    }
                    _ => {} // ignore other keys
                }
            } else {
                let mut stdin_uart = stdin_uart.lock().unwrap();
                if stdin_uart.is_none() {
                    *stdin_uart = Some(buffer.remove(0));
                }
            }
            std::thread::yield_now();
        }
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
    let cli = Args::parse();
    run(cli, false).map(|_| ())
}

fn read_binary(path: &PathBuf, expect_load_addr: u32) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Check if this is an ELF
    if buffer.starts_with(&[0x7f, 0x45, 0x4c, 0x46]) {
        println!("Loading ELF executable {}", path.display());
        let elf = elf::ElfExecutable::new(&buffer).unwrap();
        if elf.load_addr() != expect_load_addr {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "ELF executable has non-0x{:x} load address, which is not supported (got 0x{:x})",
                    expect_load_addr, elf.load_addr()
                ),
            ))?;
        }
        // TBF files have an entry point offset by 0x20
        if elf.entry_point() != expect_load_addr && elf.entry_point() != elf.load_addr() + 0x20 {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("ELF executable has non-0x{:x} entry point, which is not supported (got 0x{:x})", expect_load_addr, elf.entry_point()),
            ))?;
        }
        buffer = elf.content().clone();
    }

    Ok(buffer)
}

fn run(cli: Args, capture_uart_output: bool) -> io::Result<Vec<u8>> {
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
            println!("GDB mode not supported with new Emulator struct");
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
