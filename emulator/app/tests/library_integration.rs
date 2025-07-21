//! Integration test demonstrating library usage

use emulator::{Emulator, EmulatorArgs, EMULATOR_RUNNING, wait_for_runtime_start};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

#[test]
fn test_library_api_access() {
    // Test that we can access the public API elements
    
    // Test global variables are accessible
    assert!(EMULATOR_RUNNING.load(Ordering::Relaxed));
    
    // Test that we can create EmulatorArgs (even if we can't fully instantiate without real files)
    let _args = EmulatorArgs {
        rom: PathBuf::from("test_rom.bin"),
        firmware: PathBuf::from("test_firmware.bin"),
        otp: None,
        gdb_port: None,
        log_dir: None,
        trace_instr: false,
        stdin_uart: false,
        _no_stdin_uart: false,
        caliptra_rom: PathBuf::from("test_caliptra_rom.bin"),
        caliptra_firmware: PathBuf::from("test_caliptra_firmware.bin"),
        soc_manifest: PathBuf::from("test_manifest"),
        i3c_port: None,
        manufacturing_mode: false,
        vendor_pk_hash: None,
        owner_pk_hash: None,
        streaming_boot: None,
        primary_flash_image: None,
        secondary_flash_image: None,
        hw_revision: semver::Version::new(2, 0, 0),
        rom_offset: None,
        rom_size: None,
        uart_offset: None,
        uart_size: None,
        ctrl_offset: None,
        ctrl_size: None,
        spi_offset: None,
        spi_size: None,
        sram_offset: None,
        sram_size: None,
        pic_offset: None,
        external_test_sram_offset: None,
        external_test_sram_size: None,
        dccm_offset: None,
        dccm_size: None,
        i3c_offset: None,
        i3c_size: None,
        primary_flash_offset: None,
        primary_flash_size: None,
        secondary_flash_offset: None,
        secondary_flash_size: None,
        mci_offset: None,
        mci_size: None,
        dma_offset: None,
        dma_size: None,
        mbox_offset: None,
        mbox_size: None,
        soc_offset: None,
        soc_size: None,
        otp_offset: None,
        otp_size: None,
        lc_offset: None,
        lc_size: None,
    };
    
    // Test that utility function is accessible
    // Note: We can't actually call wait_for_runtime_start() here as it would block the test
    // but we can verify it's available in the public API
}

#[test]
fn test_submodule_access() {
    // Test that we can access public submodules
    // This verifies the module structure is correctly exported
    
    // These should compile if the modules are properly exported
    use emulator::gdb;
    use emulator::tests;
    use emulator::doe_mbox_fsm;
    use emulator::dis;
    use emulator::elf;
    use emulator::i3c_socket;
    use emulator::mctp_transport;
}
