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

use caliptra_emu_cpu::RvInstr;
use clap::{Parser};
use crossterm::event::{Event, KeyCode, KeyEvent};
use std::fs::File;
use std::io;
use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use emulator::{EmulatorArgs, Emulator};

pub static MCU_RUNTIME_STARTED: AtomicBool = AtomicBool::new(false);
pub static EMULATOR_RUNNING: AtomicBool = AtomicBool::new(true);

pub fn wait_for_runtime_start() {
    while EMULATOR_RUNNING.load(Ordering::Relaxed) && !MCU_RUNTIME_STARTED.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(10));
    }
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