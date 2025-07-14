use std::cell::RefCell;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use caliptra_emu_bus::{Bus, Clock, Timer};
use caliptra_emu_cpu::{Cpu, Pic, RvInstr, StepAction};
use caliptra_emu_cpu::{Cpu as CaliptraMainCpu, StepAction as CaliptraMainStepAction};
use caliptra_emu_periph::CaliptraRootBus as CaliptraMainRootBus;
use clap::{ArgAction, Parser};
use clap_num::maybe_hex;
use emulator_bmc::Bmc;
use emulator_caliptra::{start_caliptra, StartCaliptraArgs};
use emulator_consts::DEFAULT_CPU_ARGS;
use emulator_periph::{
    DoeMboxPeriph, DummyDoeMbox, DummyFlashCtrl, I3c, I3cController, Mci, McuRootBus,
    McuRootBusArgs, McuRootBusOffsets, Otp,
};
use emulator_registers_generated::dma::DmaPeripheral;
use emulator_registers_generated::root_bus::{AutoRootBus, AutoRootBusOffsets};
use crate::elf;

// Helper struct to return bus system components and recovery data
struct BusSystem {
    auto_root_bus: AutoRootBus,
    bmc: Option<Bmc>,
    rom_offset: u32,
    recovery_images: Option<(Vec<u8>, Vec<u8>, Vec<u8>)>, // (caliptra_firmware, soc_manifest, mcu_firmware)
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

pub struct Emulator {
    pub mcu_cpu: Cpu<AutoRootBus>,
    pub caliptra_cpu: CaliptraMainCpu<CaliptraMainRootBus>,
    pub bmc: Option<Bmc>,
    pub timer: Timer,
    pub stdin_uart: Option<Arc<Mutex<Option<u8>>>>,
    pub uart_output: Option<Rc<RefCell<Vec<u8>>>>,
}

#[derive(Debug)]
pub enum SystemStepAction {
    Continue,
    Break,
    Exit,
}

impl Emulator {
    pub fn new(
        cli: EmulatorArgs,
        capture_uart_output: bool,
        stdin_uart: Option<Arc<Mutex<Option<u8>>>>,
    ) -> io::Result<Self> {
        let device_lifecycle: Option<String> = if cli.manufacturing_mode {
            Some("manufacturing".into())
        } else {
            Some("production".into())
        };

        let req_idevid_csr: Option<bool> = if cli.manufacturing_mode {
            Some(true)
        } else {
            None
        };

        let use_mcu_recovery_interface;
        #[cfg(feature = "test-flash-based-boot")]
        {
            use_mcu_recovery_interface = true;
        }
        #[cfg(not(feature = "test-flash-based-boot"))]
        {
            use_mcu_recovery_interface = false;
        }

        // Clone the paths before moving them
        let caliptra_rom = cli.caliptra_rom.clone();
        let rom_path = cli.rom.clone();
        let firmware_path = cli.firmware.clone();

        let (mut caliptra_cpu, soc_to_caliptra) = start_caliptra(&StartCaliptraArgs {
            rom: caliptra_rom,
            device_lifecycle,
            req_idevid_csr,
            use_mcu_recovery_interface,
        })
        .expect("Failed to start Caliptra CPU");

        let rom_buffer = Self::read_binary(&rom_path, 0)?;
        let mcu_firmware = Self::read_binary(&firmware_path, 0x4000_0000)?;

        let clock = Rc::new(Clock::new());
        let timer = Timer::new(&clock);

        let uart_output = if capture_uart_output {
            Some(Rc::new(RefCell::new(Vec::new())))
        } else {
            None
        };

        let pic = Rc::new(Pic::new());

        // Build the bus system (this is the complex part from run())
        let bus_system = Self::build_bus_system(
            cli,
            rom_buffer,
            &clock,
            &pic,
            uart_output.clone(),
            stdin_uart.clone(),
            soc_to_caliptra,
            mcu_firmware,
        )?;

        let mut mcu_cpu = Cpu::new(bus_system.auto_root_bus, clock, pic, DEFAULT_CPU_ARGS);
        mcu_cpu.write_pc(bus_system.rom_offset);
        
        // Set up BMC with proper event channels after CPUs are created
        let mut bmc = bus_system.bmc;
        
        #[cfg(feature = "test-flash-based-boot")]
        {
            println!("Emulator is using MCU recovery interface");
            let (caliptra_event_sender, caliptra_event_receiver) = caliptra_cpu.register_events();
            let (mcu_event_sender, mcu_event_receiver) = mcu_cpu.register_events();
            mcu_cpu.bus
                .i3c_periph
                .as_mut()
                .unwrap()
                .periph
                .register_event_channels(
                    caliptra_event_sender,
                    caliptra_event_receiver,
                    mcu_event_sender,
                    mcu_event_receiver,
                );
        }
        #[cfg(not(feature = "test-flash-based-boot"))]
        {
            let (caliptra_event_sender, caliptra_event_receiver) = caliptra_cpu.register_events();
            let (mcu_event_sender, mcu_event_receiver) = mcu_cpu.register_events();
            
            // Create the BMC recovery interface emulator
            bmc = Some(Bmc::new(
                caliptra_event_sender,
                caliptra_event_receiver,
                mcu_event_sender,
                mcu_event_receiver,
            ));

            // Load the firmware images and SoC manifest into the recovery interface emulator
            if let Some((caliptra_firmware, soc_manifest, mcu_firmware)) = bus_system.recovery_images {
                let bmc_ref = bmc.as_mut().unwrap();
                bmc_ref.push_recovery_image(caliptra_firmware);
                bmc_ref.push_recovery_image(soc_manifest);
                bmc_ref.push_recovery_image(mcu_firmware);
                println!("Active mode enabled with 3 recovery images");
            }
        }

        Ok(Emulator {
            mcu_cpu,
            caliptra_cpu,
            bmc,
            timer,
            stdin_uart,
            uart_output,
        })
    }

    pub fn step(&mut self, trace_fn: Option<&mut dyn FnMut(u32, RvInstr)>) -> SystemStepAction {

        // Step MCU CPU
        let mcu_action = self.mcu_cpu.step(trace_fn);

        // Step Caliptra CPU
        let _caliptra_action = match self.caliptra_cpu.step(None) {
            CaliptraMainStepAction::Continue => SystemStepAction::Continue,
            _ => {
                println!("Caliptra CPU Halted");
                SystemStepAction::Continue // Don't exit system if only Caliptra halts
            }
        };

        // Step BMC if present
        if let Some(bmc) = self.bmc.as_mut() {
            bmc.step();
        }

        // Check stdin UART
        if let Some(ref stdin_uart) = self.stdin_uart {
            if stdin_uart.lock().unwrap().is_some() {
                self.timer.schedule_poll_in(1);
            }
        }

        // Return the most significant action
        match mcu_action {
            StepAction::Continue => SystemStepAction::Continue,
            StepAction::Break => SystemStepAction::Break,
            _ => SystemStepAction::Exit,
        }
    }

    pub fn get_uart_output(&self) -> Option<Vec<u8>> {
        self.uart_output.as_ref().map(|o| o.borrow().clone())
    }

    pub fn read_pc(&self) -> u32 {
        self.mcu_cpu.read_pc()
    }

    pub fn write_pc(&mut self, pc: u32) {
        self.mcu_cpu.write_pc(pc);
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
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "ELF executable has non-0x{:x} load address, which is not supported (got 0x{:x})",
                        expect_load_addr, elf.load_addr()
                    ),
                ));
            }
            // TBF files have an entry point offset by 0x20
            if elf.entry_point() != expect_load_addr && elf.entry_point() != elf.load_addr() + 0x20 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("ELF executable has non-0x{:x} entry point, which is not supported (got 0x{:x})", expect_load_addr, elf.entry_point()),
                ));
            }
            buffer = elf.content().clone();
        }

        Ok(buffer)
    }

    fn build_bus_system(
        cli: EmulatorArgs,
        rom_buffer: Vec<u8>,
        clock: &Rc<Clock>,
        pic: &Rc<Pic>,
        uart_output: Option<Rc<RefCell<Vec<u8>>>>,
        stdin_uart: Option<Arc<Mutex<Option<u8>>>>,
        soc_to_caliptra: impl Bus + 'static,
        mcu_firmware: Vec<u8>,
    ) -> io::Result<BusSystem> {
        use emulator_consts::ROM_SIZE;
        use std::process::exit;
        
        if rom_buffer.len() > ROM_SIZE as usize {
            println!("ROM File Size must not exceed {} bytes", ROM_SIZE);
            exit(-1);
        }

        let mut mcu_root_bus_offsets = McuRootBusOffsets::default();
        let auto_root_bus_offsets = AutoRootBusOffsets::default();

        // Apply CLI overrides to offsets (simplified for brevity)
        if let Some(rom_offset) = cli.rom_offset {
            mcu_root_bus_offsets.rom_offset = rom_offset;
        }

        let bus_args = McuRootBusArgs {
            offsets: mcu_root_bus_offsets.clone(),
            rom: rom_buffer,
            log_dir: cli.log_dir.unwrap_or_else(|| PathBuf::from("/tmp")),
            uart_output: uart_output.clone(),
            uart_rx: stdin_uart.clone(),
            pic: pic.clone(),
            clock: clock.clone(),
        };
        
        let root_bus = McuRootBus::new(bus_args).unwrap();

        let dma_ram = root_bus.ram.clone();
        let dma_rom_sram = root_bus.rom_sram.clone();

        // Create peripherals
        let i3c_error_irq = pic.register_irq(McuRootBus::I3C_ERROR_IRQ);
        let i3c_notif_irq = pic.register_irq(McuRootBus::I3C_NOTIF_IRQ);
        
        let mut i3c_controller = if let Some(i3c_port) = cli.i3c_port {
            use crate::i3c_socket::start_i3c_socket;
            let (rx, tx) = start_i3c_socket(i3c_port);
            I3cController::new(rx, tx)
        } else {
            I3cController::default()
        };
        
        let i3c = I3c::new(
            clock,
            &mut i3c_controller,
            i3c_error_irq,
            i3c_notif_irq,
            cli.hw_revision.clone(),
        );

        // Create other peripherals
        let doe_event_irq = pic.register_irq(McuRootBus::DOE_MBOX_EVENT_IRQ);
        let doe_mbox_periph = DoeMboxPeriph::default();
        let doe_mbox = DummyDoeMbox::new(clock, doe_event_irq, doe_mbox_periph);

        let create_flash_controller = |default_path: &str, error_irq: u8, event_irq: u8, initial_content: Option<&[u8]>| {
            let flash_file = Some(PathBuf::from(default_path));
            DummyFlashCtrl::new(
                clock,
                flash_file,
                pic.register_irq(error_irq),
                pic.register_irq(event_irq),
                initial_content,
            ).unwrap()
        };

        let primary_flash_controller = create_flash_controller(
            "primary_flash",
            McuRootBus::PRIMARY_FLASH_CTRL_ERROR_IRQ,
            McuRootBus::PRIMARY_FLASH_CTRL_EVENT_IRQ,
            None,
        );

        let secondary_flash_controller = create_flash_controller(
            "secondary_flash",
            McuRootBus::SECONDARY_FLASH_CTRL_ERROR_IRQ,
            McuRootBus::SECONDARY_FLASH_CTRL_EVENT_IRQ,
            None,
        );

        let mut dma_ctrl = emulator_periph::DummyDmaCtrl::new(
            clock,
            pic.register_irq(McuRootBus::DMA_ERROR_IRQ),
            pic.register_irq(McuRootBus::DMA_EVENT_IRQ),
            Some(root_bus.external_test_sram.clone()),
        ).unwrap();

        // Set DMA RAM using the trait method
        dma_ctrl.set_dma_ram(dma_ram.clone());

        let delegates: Vec<Box<dyn Bus>> = vec![Box::new(root_bus), Box::new(soc_to_caliptra)];

        let vendor_pk_hash = cli.vendor_pk_hash.map(|hash| {
            let v = hex::decode(hash).unwrap();
            v.try_into().unwrap()
        });
        let owner_pk_hash = cli.owner_pk_hash.map(|hash| {
            let v = hex::decode(hash).unwrap();
            v.try_into().unwrap()
        });

        let otp = Otp::new(clock, cli.otp, owner_pk_hash, vendor_pk_hash)?;
        let mci = Mci::new(clock);
        
        let mut auto_root_bus = AutoRootBus::new(
            delegates,
            Some(auto_root_bus_offsets),
            Some(Box::new(i3c)),
            Some(Box::new(primary_flash_controller)),
            Some(Box::new(secondary_flash_controller)),
            Some(Box::new(mci)),
            Some(Box::new(doe_mbox)),
            Some(Box::new(dma_ctrl)),
            None,
            Some(Box::new(otp)),
            None,
            None,
            None,
            None,
        );

        // Set the DMA RAM for Primary Flash Controller after AutoRootBus creation
        auto_root_bus
            .primary_flash_periph
            .as_mut()
            .unwrap()
            .periph
            .set_dma_ram(dma_ram.clone());

        // Set DMA RAM for ROM access to Primary Flash Controller
        auto_root_bus
            .primary_flash_periph
            .as_mut()
            .unwrap()
            .periph
            .set_dma_rom_sram(dma_rom_sram.clone());

        // Set the DMA RAM for Secondary Flash Controller
        auto_root_bus
            .secondary_flash_periph
            .as_mut()
            .unwrap()
            .periph
            .set_dma_ram(dma_ram.clone());

        // Set the DMA RAM for ROM access to Secondary Flash Controller
        auto_root_bus
            .secondary_flash_periph
            .as_mut()
            .unwrap()
            .periph
            .set_dma_rom_sram(dma_rom_sram.clone());

        // Create BMC if not using flash-based boot
        let bmc: Option<Bmc> = None;
        // Prepare recovery images for BMC if not using flash-based boot
        let mut recovery_images = None;
        #[cfg(feature = "test-flash-based-boot")]
        {
            println!("Emulator is using MCU recovery interface");
        }
        #[cfg(not(feature = "test-flash-based-boot"))]
        {
            // Load the firmware images and SoC manifest for recovery interface
            let caliptra_firmware = Self::read_binary(&cli.caliptra_firmware, emulator_consts::RAM_ORG)?;
            let soc_manifest = Self::read_binary(&cli.soc_manifest, 0)?;
            
            println!("Loaded recovery images:");
            println!("  - Caliptra firmware: {} bytes", caliptra_firmware.len());
            println!("  - SoC manifest: {} bytes", soc_manifest.len());
            println!("  - MCU firmware: {} bytes", mcu_firmware.len());
            
            recovery_images = Some((caliptra_firmware, soc_manifest, mcu_firmware));
        }

        Ok(BusSystem {
            auto_root_bus,
            bmc: None, // BMC will be created after CPU initialization
            rom_offset: mcu_root_bus_offsets.rom_offset,
            recovery_images,
        })
    }
}