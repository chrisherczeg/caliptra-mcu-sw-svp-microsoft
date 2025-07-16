/*++

Licensed under the Apache-2.0 license.

File Name:

    gdb_target.rs

Abstract:

    File contains gdb_target module for Caliptra Emulator.

--*/

use caliptra_emu_cpu::xreg_file::XReg;
use caliptra_emu_cpu::{WatchPtrKind};
use caliptra_emu_types::RvSize;
use gdbstub::arch::SingleStepGdbBehavior;
use gdbstub::common::Signal;
use gdbstub::stub::SingleThreadStopReason;
use gdbstub::target;
use gdbstub::target::ext::base::singlethread::{SingleThreadBase, SingleThreadResume};
use gdbstub::target::ext::base::BaseOps;
use gdbstub::target::ext::breakpoints::WatchKind;
use gdbstub::target::Target;
use gdbstub::target::TargetResult;
use gdbstub_arch;

use crate::emulator::{Emulator, SystemStepAction};

pub enum ExecMode {
    Step,
    Continue,
}

pub struct GdbTarget {
    emulator: Emulator,
    exec_mode: ExecMode,
    breakpoints: Vec<u32>,
    interrupt_requested: bool,
}

impl GdbTarget {
    // Create new instance of GdbTarget
    pub fn new(emulator: Emulator) -> Self {
        Self {
            emulator,
            exec_mode: ExecMode::Continue,
            breakpoints: Vec::new(),
            interrupt_requested: false,
        }
    }

    // Conditional Run (Private function)
    fn cond_run(&mut self) -> SingleThreadStopReason<u32> {
        loop {
            // Check for interrupt request (Ctrl+C)
            if self.interrupt_requested {
                self.interrupt_requested = false;
                return SingleThreadStopReason::Signal(Signal::SIGINT);
            }

            match self.emulator.step(None) {
                SystemStepAction::Continue => {
                    if self.breakpoints.contains(&self.emulator.read_pc()) {
                        println!("Hit breakpoint at PC: 0x{:08X}", self.emulator.read_pc());
                        return SingleThreadStopReason::SwBreak(());
                    }
                }
                SystemStepAction::Break => {
                    let watch = self.emulator.mcu_cpu.get_watchptr_hit().unwrap();
                    return SingleThreadStopReason::Watch {
                        tid: (),
                        kind: if watch.kind == WatchPtrKind::Write {
                            WatchKind::Write
                        } else {
                            WatchKind::Read
                        },
                        addr: watch.addr,
                    };
                }
                SystemStepAction::Exit => break,
            }
        }
        SingleThreadStopReason::Exited(0)
    }

    // run the gdb target
    pub fn run(&mut self) -> SingleThreadStopReason<u32> {
        match self.exec_mode {
            ExecMode::Step => {
                self.emulator.step(None);
                SingleThreadStopReason::DoneStep
            }
            ExecMode::Continue => self.cond_run(),
        }
    }

    // Execute a single step and return stop reason if execution should halt
    pub fn run_single_step(&mut self) -> Option<SingleThreadStopReason<u32>> {
        // Check for interrupt request (Ctrl+C) first
        if self.interrupt_requested {
            self.interrupt_requested = false;
            return Some(SingleThreadStopReason::Signal(Signal::SIGINT));
        }

        match self.exec_mode {
            ExecMode::Step => {
                self.emulator.step(None);
                Some(SingleThreadStopReason::DoneStep)
            }
            ExecMode::Continue => {
                match self.emulator.step(None) {
                    SystemStepAction::Continue => {
                        if self.breakpoints.contains(&self.emulator.read_pc()) {
                            println!("Hit breakpoint at PC: 0x{:08X}", self.emulator.read_pc());
                            Some(SingleThreadStopReason::SwBreak(()))
                        } else {
                            None // Continue execution
                        }
                    }
                    SystemStepAction::Break => {
                        let watch = self.emulator.mcu_cpu.get_watchptr_hit().unwrap();
                        Some(SingleThreadStopReason::Watch {
                            tid: (),
                            kind: if watch.kind == WatchPtrKind::Write {
                                WatchKind::Write
                            } else {
                                WatchKind::Read
                            },
                            addr: watch.addr,
                        })
                    }
                    SystemStepAction::Exit => Some(SingleThreadStopReason::Exited(0)),
                }
            }
        }
    }

    // Signal an interrupt request (called when Ctrl+C is received)
    pub fn request_interrupt(&mut self) {
        self.interrupt_requested = true;
    }

    // Check if an interrupt has been requested
    pub fn is_interrupt_requested(&self) -> bool {
        self.interrupt_requested
    }

    // Execute the target with responsive interrupt checking
    pub fn run_responsive(&mut self) -> SingleThreadStopReason<u32> {
        match self.exec_mode {
            ExecMode::Step => {
                self.emulator.step(None);
                SingleThreadStopReason::DoneStep
            }
            ExecMode::Continue => {
                // Execute with interrupt checking every few steps
                for _ in 0..1000 {  // Check for interrupts every 1000 steps
                    // Check for interrupt request (Ctrl+C) first
                    if self.interrupt_requested {
                        self.interrupt_requested = false;
                        println!("Interrupt request detected, stopping execution");
                        return SingleThreadStopReason::Signal(Signal::SIGINT);
                    }

                    match self.emulator.step(None) {
                        SystemStepAction::Continue => {
                            if self.breakpoints.contains(&self.emulator.read_pc()) {
                                println!("Hit breakpoint at PC: 0x{:08X}", self.emulator.read_pc());
                                return SingleThreadStopReason::SwBreak(());
                            }
                        }
                        SystemStepAction::Break => {
                            let watch = self.emulator.mcu_cpu.get_watchptr_hit().unwrap();
                            return SingleThreadStopReason::Watch {
                                tid: (),
                                kind: if watch.kind == WatchPtrKind::Write {
                                    WatchKind::Write
                                } else {
                                    WatchKind::Read
                                },
                                addr: watch.addr,
                            };
                        }
                        SystemStepAction::Exit => return SingleThreadStopReason::Exited(0),
                    }
                }
                
                // If we reach here, we've executed 1000 steps without hitting a breakpoint
                // Return a temporary stop to allow gdbstub to check for interrupts
                // This creates a responsive execution loop
                SingleThreadStopReason::Signal(Signal::SIGALRM)
            }
        }
    }
}

impl Target for GdbTarget {
    type Arch = gdbstub_arch::riscv::Riscv32;
    type Error = &'static str;

    fn base_ops(&mut self) -> BaseOps<Self::Arch, Self::Error> {
        BaseOps::SingleThread(self)
    }

    fn guard_rail_implicit_sw_breakpoints(&self) -> bool {
        true
    }

    fn guard_rail_single_step_gdb_behavior(&self) -> SingleStepGdbBehavior {
        SingleStepGdbBehavior::Optional
    }

    fn support_breakpoints(
        &mut self,
    ) -> Option<target::ext::breakpoints::BreakpointsOps<'_, Self>> {
        Some(self)
    }
}

impl SingleThreadBase for GdbTarget {
    fn read_registers(
        &mut self,
        regs: &mut gdbstub_arch::riscv::reg::RiscvCoreRegs<u32>,
    ) -> TargetResult<(), Self> {
        // Read PC
        regs.pc = self.emulator.read_pc();

        // Read XReg
        for idx in 0..regs.x.len() {
            regs.x[idx] = self.emulator.mcu_cpu.read_xreg(XReg::from(idx as u16)).unwrap();
        }

        Ok(())
    }

    fn write_registers(
        &mut self,
        regs: &gdbstub_arch::riscv::reg::RiscvCoreRegs<u32>,
    ) -> TargetResult<(), Self> {
        // Write PC
        self.emulator.write_pc(regs.pc);

        // Write XReg
        for idx in 0..regs.x.len() {
            self.emulator.mcu_cpu
                .write_xreg(XReg::from(idx as u16), regs.x[idx])
                .unwrap();
        }

        Ok(())
    }

    fn read_addrs(&mut self, start_addr: u32, data: &mut [u8]) -> TargetResult<(), Self> {
        #[allow(clippy::needless_range_loop)]
        for i in 0..data.len() {
            data[i] = self.emulator.mcu_cpu
                .read_bus(RvSize::Byte, start_addr.wrapping_add(i as u32))
                .unwrap_or_default() as u8;
        }
        Ok(())
    }

    fn write_addrs(&mut self, start_addr: u32, data: &[u8]) -> TargetResult<(), Self> {
        #[allow(clippy::needless_range_loop)]
        for i in 0..data.len() {
            self.emulator.mcu_cpu
                .write_bus(
                    RvSize::Byte,
                    start_addr.wrapping_add(i as u32),
                    data[i] as u32,
                )
                .unwrap_or_default();
        }
        Ok(())
    }

    fn support_resume(
        &mut self,
    ) -> Option<target::ext::base::singlethread::SingleThreadResumeOps<'_, Self>> {
        Some(self)
    }
}

impl target::ext::base::singlethread::SingleThreadSingleStep for GdbTarget {
    fn step(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        // Handle signals appropriately
        match signal {
            None => {
                // Normal single step without signal
                self.exec_mode = ExecMode::Step;
            }
            Some(Signal::SIGINT) => {
                // SIGINT can be safely ignored when stepping - just step normally
                println!("Single stepping after SIGINT");
                self.exec_mode = ExecMode::Step;
            }
            Some(Signal::SIGALRM) => {
                // SIGALRM is our internal signal for responsive execution - step normally
                self.exec_mode = ExecMode::Step;
            }
            Some(_other_signal) => {
                // For other signals, we don't support signal injection
                return Err("no support for stepping with signal");
            }
        }

        Ok(())
    }
}

impl SingleThreadResume for GdbTarget {
    fn resume(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        // Handle signals appropriately
        match signal {
            None => {
                // Normal continue without signal
                self.exec_mode = ExecMode::Continue;
            }
            Some(Signal::SIGINT) => {
                // SIGINT can be safely ignored when resuming - just continue normally
                println!("Resuming execution after SIGINT");
                self.exec_mode = ExecMode::Continue;
            }
            Some(Signal::SIGALRM) => {
                // SIGALRM is our internal signal for responsive execution - continue normally
                self.exec_mode = ExecMode::Continue;
            }
            Some(_other_signal) => {
                // For other signals, we don't support signal injection
                return Err("no support for continuing with signal");
            }
        }

        Ok(())
    }

    #[inline(always)]
    fn support_single_step(
        &mut self,
    ) -> Option<target::ext::base::singlethread::SingleThreadSingleStepOps<'_, Self>> {
        Some(self)
    }
}

impl target::ext::breakpoints::Breakpoints for GdbTarget {
    #[inline(always)]
    fn support_sw_breakpoint(
        &mut self,
    ) -> Option<target::ext::breakpoints::SwBreakpointOps<'_, Self>> {
        Some(self)
    }
    #[inline(always)]
    fn support_hw_watchpoint(
        &mut self,
    ) -> Option<target::ext::breakpoints::HwWatchpointOps<'_, Self>> {
        Some(self)
    }
}

impl target::ext::breakpoints::SwBreakpoint for GdbTarget {
    fn add_sw_breakpoint(&mut self, addr: u32, _kind: usize) -> TargetResult<bool, Self> {
        self.breakpoints.push(addr);
        Ok(true)
    }

    fn remove_sw_breakpoint(&mut self, addr: u32, _kind: usize) -> TargetResult<bool, Self> {
        match self.breakpoints.iter().position(|x| *x == addr) {
            None => return Ok(false),
            Some(pos) => self.breakpoints.remove(pos),
        };

        Ok(true)
    }
}

impl target::ext::breakpoints::HwWatchpoint for GdbTarget {
    fn add_hw_watchpoint(
        &mut self,
        addr: u32,
        len: u32,
        kind: WatchKind,
    ) -> TargetResult<bool, Self> {
        // Add Watchpointer (and transform WatchKind to WatchPtrKind)
        self.emulator.mcu_cpu.add_watchptr(
            addr,
            len,
            if kind == WatchKind::Write {
                WatchPtrKind::Write
            } else {
                WatchPtrKind::Read
            },
        );

        Ok(true)
    }

    fn remove_hw_watchpoint(
        &mut self,
        addr: u32,
        len: u32,
        kind: WatchKind,
    ) -> TargetResult<bool, Self> {
        // Remove Watchpointer (and transform WatchKind to WatchPtrKind)
        self.emulator.mcu_cpu.remove_watchptr(
            addr,
            len,
            if kind == WatchKind::Write {
                WatchPtrKind::Write
            } else {
                WatchPtrKind::Read
            },
        );
        Ok(true)
    }
}
