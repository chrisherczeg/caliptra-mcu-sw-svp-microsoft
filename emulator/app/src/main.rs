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

use crate::i3c_socket::start_i3c_socket;
use caliptra_emu_bus::{Bus, Clock, Timer};
use caliptra_emu_cpu::{Cpu, Pic, RvInstr, StepAction};
use caliptra_emu_cpu::{Cpu as CaliptraMainCpu, StepAction as CaliptraMainStepAction};
use caliptra_emu_periph::CaliptraRootBus as CaliptraMainRootBus;
use clap::{ArgAction, Parser};
use clap_num::maybe_hex;
use crossterm::event::{Event, KeyCode, KeyEvent};
use emulator_bmc::Bmc;
use emulator_caliptra::{start_caliptra, StartCaliptraArgs};
use emulator_consts::DEFAULT_CPU_ARGS;
use emulator_consts::{RAM_ORG, ROM_SIZE};
use emulator_periph::{
    DoeMboxPeriph, DummyDoeMbox, DummyFlashCtrl, I3c, I3cController, Mci, McuRootBus,
    McuRootBusArgs, McuRootBusOffsets, Otp,
};
use emulator_registers_generated::dma::DmaPeripheral;
use emulator_registers_generated::root_bus::{AutoRootBus, AutoRootBusOffsets};
use gdb::gdb_state;
use gdb::gdb_target::GdbTarget;
use mctp_transport::MctpTransport;
use pldm_fw_pkg::FirmwareManifest;
use pldm_ua::daemon::PldmDaemon;
use pldm_ua::transport::{EndpointId, PldmTransport};
use std::cell::RefCell;
use std::fs::File;
use std::io;
use std::io::{IsTerminal, Read, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tests::mctp_util::base_protocol::LOCAL_TEST_ENDPOINT_EID;
use tests::pldm_request_response_test::PldmRequestResponseTest;
use emulator::Emulator;

pub static MCU_RUNTIME_STARTED: AtomicBool = AtomicBool::new(false);
pub static EMULATOR_RUNNING: AtomicBool = AtomicBool::new(true);

pub fn wait_for_runtime_start() {
    while EMULATOR_RUNNING.load(Ordering::Relaxed) && !MCU_RUNTIME_STARTED.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None, name = "Caliptra MCU Emulator")]
pub struct EmulatorArgs {
    /// ROM binary path
    #[arg(short, long)]
    pub rom: PathBuf,

    #[arg(short, long)]
    pub firmware: PathBuf,

    /// Optional file to store OTP / fuses between runs.
    #[arg(short, long)]
    pub otp: Option<PathBuf>,

    /// GDB Debugger Port
    #[arg(short, long)]
    pub gdb_port: Option<u16>,

    /// Directory in which to log execution artifacts.
    #[arg(short, long)]
    pub log_dir: Option<PathBuf>,

    /// Trace instructions.
    #[arg(short, long, default_value_t = false)]
    pub trace_instr: bool,

    // These look backwards, but this is necessary so that the default is to capture stdin.
    /// Pass stdin to the MCU UART Rx.
    #[arg(long = "no-stdin-uart", action = ArgAction::SetFalse)]
    pub stdin_uart: bool,

    // this is used only to set stdin_uart to false
    #[arg(long = "stdin-uart", overrides_with = "stdin_uart")]
    pub _no_stdin_uart: bool,

    /// The ROM path for the Caliptra CPU.
    #[arg(long)]
    pub caliptra_rom: PathBuf,

    /// The Firmware path for the Caliptra CPU.
    #[arg(long)]
    pub caliptra_firmware: PathBuf,

    #[arg(long)]
    pub soc_manifest: PathBuf,

    #[arg(long)]
    pub i3c_port: Option<u16>,

    /// This is only needed if the IDevID CSR needed to be generated in the Caliptra Core.
    #[arg(long)]
    pub manufacturing_mode: bool,

    #[arg(long)]
    pub vendor_pk_hash: Option<String>,

    #[arg(long)]
    pub owner_pk_hash: Option<String>,

    /// Path to the streaming boot PLDM firmware package
    #[arg(long)]
    pub streaming_boot: Option<PathBuf>,

    #[arg(long)]
    pub primary_flash_image: Option<PathBuf>,

    #[arg(long)]
    pub secondary_flash_image: Option<PathBuf>,

    /// HW revision in semver format (e.g., "2.0.0")
    #[arg(long, value_parser = semver::Version::parse, default_value = "2.0.0")]
    pub hw_revision: semver::Version,

    /// Override ROM offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub rom_offset: Option<u32>,
    /// Override ROM size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub rom_size: Option<u32>,
    /// Override UART offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub uart_offset: Option<u32>,
    /// Override UART size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub uart_size: Option<u32>,
    /// Override emulator control offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub ctrl_offset: Option<u32>,
    /// Override emulator control size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub ctrl_size: Option<u32>,
    /// Override SPI offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub spi_offset: Option<u32>,
    /// Override SPI size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub spi_size: Option<u32>,
    /// Override SRAM offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub sram_offset: Option<u32>,
    /// Override SRAM size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub sram_size: Option<u32>,
    /// Override PIC offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub pic_offset: Option<u32>,
    /// Override external test SRAM offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub external_test_sram_offset: Option<u32>,
    /// Override external test SRAM size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub external_test_sram_size: Option<u32>,
    /// Override DCCM offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub dccm_offset: Option<u32>,
    /// Override DCCM size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub dccm_size: Option<u32>,
    /// Override I3C offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub i3c_offset: Option<u32>,
    /// Override I3C size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub i3c_size: Option<u32>,
    /// Override primary flash offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub primary_flash_offset: Option<u32>,
    /// Override primary flash size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub primary_flash_size: Option<u32>,
    /// Override secondary flash offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub secondary_flash_offset: Option<u32>,
    /// Override secondary flash size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub secondary_flash_size: Option<u32>,
    /// Override MCI offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub mci_offset: Option<u32>,
    /// Override MCI size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub mci_size: Option<u32>,
    /// Override DMA offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub dma_offset: Option<u32>,
    /// Override DMA size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub dma_size: Option<u32>,
    /// Override Caliptra mailbox offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub mbox_offset: Option<u32>,
    /// Override Caliptra mailbox size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub mbox_size: Option<u32>,
    /// Override Caliptra SoC interface offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub soc_offset: Option<u32>,
    /// Override Caliptra SoC interface size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub soc_size: Option<u32>,
    /// Override OTP offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub otp_offset: Option<u32>,
    /// Override OTP size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub otp_size: Option<u32>,
    /// Override LC offset
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub lc_offset: Option<u32>,
    /// Override LC size
    #[arg(long, value_parser=maybe_hex::<u32>)]
    pub lc_size: Option<u32>,
}

fn disassemble(pc: u32, instr: u32) -> String {
    let mut out = vec![];
    // TODO: we should replace this with something more efficient.
    let dis = dis::disasm_inst(dis::RvIsa::Rv32, pc as u64, instr as u64);
    write!(&mut out, "0x{:08x}   {}", pc, dis).unwrap();

    String::from_utf8(out).unwrap()
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
pub fn free_run(
    running: Arc<AtomicBool>,
    mut emulator: Emulator,
    trace_path: Option<PathBuf>,
) {
    // read from the console in a separate thread to prevent blocking
    let stdin_uart_clone = emulator.stdin_uart.clone();
    std::thread::spawn(move || read_console(stdin_uart_clone));

    if let Some(path) = trace_path {
        let mut f = File::create(path).unwrap();
        let trace_fn: &mut dyn FnMut(u32, RvInstr) = &mut |pc, instr| match instr {
            RvInstr::Instr32(instr32) => {
                let _ = writeln!(&mut f, "{}", disassemble(pc, instr32));
                println!("{{mcu cpu}}      {}", disassemble(pc, instr32));
            }
            RvInstr::Instr16(instr16) => {
                let _ = writeln!(&mut f, "{}", disassemble(pc, instr16 as u32));
                println!("{{mcu cpu}}      {}", disassemble(pc, instr16 as u32));
            }
        };

        while running.load(std::sync::atomic::Ordering::Relaxed) {

            match emulator.step(Some(trace_fn)) {
                emulator::SystemStepAction::Continue => {}
                emulator::SystemStepAction::Break => break,
                emulator::SystemStepAction::Exit => break,
            }
        }
    } else {
        while running.load(std::sync::atomic::Ordering::Relaxed) {

            match emulator.step(None) {
                emulator::SystemStepAction::Continue => {}
                emulator::SystemStepAction::Break => break,
                emulator::SystemStepAction::Exit => break,
            }
        }
    }
}

fn main() -> io::Result<()> {
    let cli = EmulatorArgs::parse();
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

pub fn run(cli: EmulatorArgs, capture_uart_output: bool) -> io::Result<Vec<u8>> {
    println!("{:?}", cli);

    // exit cleanly on Ctrl-C so that we save any state.
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    if io::stdout().is_terminal() {
        ctrlc::set_handler(move || {
            running_clone.store(false, std::sync::atomic::Ordering::Relaxed);
        })
        .unwrap();
    }

    let stdin_uart = if cli.stdin_uart && std::io::stdin().is_terminal() {
        Some(Arc::new(Mutex::new(None)))
    } else {
        None
    };

    // Create the unified emulator system
    let emulator = Emulator::new(cli.clone(), capture_uart_output, stdin_uart)?;
    let uart_output = emulator.get_uart_output();

    // Check if Optional GDB Port is passed
    match cli.gdb_port {
        Some(port) => {
            // Create GDB Target Instance
            let mut gdb_target = gdb::gdb_target::GdbTarget::new(emulator);

            // Execute CPU through GDB State Machine
            gdb::gdb_state::wait_for_gdb_run(&mut gdb_target, port);
            
            Ok(uart_output.unwrap_or_default())
        }
        _ => {
            let instr_trace = if cli.trace_instr {
                Some(PathBuf::from("/tmp").join("caliptra_instr_trace.txt"))
            } else {
                None
            };

            // If no GDB Port is passed, Free Run
            free_run(
                running.clone(),
                emulator,
                instr_trace,
            );
            
            Ok(uart_output.unwrap_or_default())
        }
    }
}