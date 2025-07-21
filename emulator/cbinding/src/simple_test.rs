/*++

Licensed under the Apache-2.0 license.

File Name:

    simple_test.rs

Abstract:

    Simple test to verify the C bindings can be compiled.

--*/

use emulator::{Emulator, EmulatorArgs};

#[test]
fn test_can_import_emulator() {
    // This test just verifies we can import the emulator types
    let size = std::mem::size_of::<Emulator>();
    let align = std::mem::align_of::<Emulator>();
    
    println!("Emulator size: {}, alignment: {}", size, align);
    
    assert!(size > 0);
    assert!(align > 0);
}

#[test]
fn test_emulator_args_creation() {
    // Test that we can create EmulatorArgs
    use std::path::PathBuf;
    
    let args = EmulatorArgs {
        rom: PathBuf::from("test_rom.bin"),
        firmware: PathBuf::from("test_firmware.bin"),
        caliptra_rom: PathBuf::from("test_caliptra_rom.bin"),
        caliptra_firmware: PathBuf::from("test_caliptra_firmware.bin"),
        soc_manifest: PathBuf::from("test_soc_manifest.bin"),
        otp: None,
        gdb_port: None,
        log_dir: None,
        trace_instr: false,
        stdin_uart: false,
        _no_stdin_uart: false,
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
    
    println!("EmulatorArgs created successfully");
}
