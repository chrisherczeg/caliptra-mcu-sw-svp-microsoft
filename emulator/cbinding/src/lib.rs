/*++

Licensed under the Apache-2.0 license.

File Name:

    lib.rs

Abstract:

    C bindings for the Caliptra MCU Emulator.

--*/

use emulator::{Emulator, EmulatorArgs};
use caliptra_emu_cpu::StepAction;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint, c_uchar};
use std::ptr;
use std::mem::ManuallyDrop;

#[cfg(test)]
mod simple_test;

/// Error codes for C API
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum EmulatorError {
    Success = 0,
    InvalidArgs = -1,
    InitializationFailed = -2,
    NullPointer = -3,
    InvalidEmulator = -4,
}

/// Step action results for C API
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum CStepAction {
    Continue = 0,
    Break = 1,
    ExitSuccess = 2,
    ExitFailure = 3,
}

impl From<StepAction> for CStepAction {
    fn from(action: StepAction) -> Self {
        match action {
            StepAction::Continue => CStepAction::Continue,
            StepAction::Break => CStepAction::Break,
            StepAction::ExitSuccess => CStepAction::ExitSuccess,
            StepAction::ExitFailure => CStepAction::ExitFailure,
        }
    }
}

/// Opaque structure representing the emulator
/// C code should allocate memory for this structure
#[repr(C)]
pub struct CEmulator {
    _private: [u8; 0],
}

/// Configuration structure for emulator initialization
#[repr(C)]
pub struct CEmulatorConfig {
    pub rom_path: *const c_char,
    pub firmware_path: *const c_char,
    pub caliptra_rom_path: *const c_char,
    pub caliptra_firmware_path: *const c_char,
    pub soc_manifest_path: *const c_char,
    pub otp_path: *const c_char,          // Optional, can be null
    pub log_dir_path: *const c_char,      // Optional, can be null
    pub gdb_port: c_uint,                 // 0 means no GDB
    pub i3c_port: c_uint,                 // 0 means no I3C socket
    pub trace_instr: c_uchar,             // 0 = false, 1 = true
    pub stdin_uart: c_uchar,              // 0 = false, 1 = true
    pub manufacturing_mode: c_uchar,      // 0 = false, 1 = true
    pub capture_uart_output: c_uchar,     // 0 = false, 1 = true
    pub vendor_pk_hash: *const c_char,    // Optional, can be null
    pub owner_pk_hash: *const c_char,     // Optional, can be null
    pub streaming_boot_path: *const c_char, // Optional, can be null
    pub primary_flash_image_path: *const c_char, // Optional, can be null
    pub secondary_flash_image_path: *const c_char, // Optional, can be null
    pub hw_revision_major: c_uint,
    pub hw_revision_minor: c_uint,
    pub hw_revision_patch: c_uint,
    
    // Memory layout override parameters (0 means use default)
    pub rom_offset: c_uint,
    pub rom_size: c_uint,
    pub uart_offset: c_uint,
    pub uart_size: c_uint,
    pub ctrl_offset: c_uint,
    pub ctrl_size: c_uint,
    pub spi_offset: c_uint,
    pub spi_size: c_uint,
    pub sram_offset: c_uint,
    pub sram_size: c_uint,
    pub pic_offset: c_uint,
    pub external_test_sram_offset: c_uint,
    pub external_test_sram_size: c_uint,
    pub dccm_offset: c_uint,
    pub dccm_size: c_uint,
    pub i3c_offset: c_uint,
    pub i3c_size: c_uint,
    pub primary_flash_offset: c_uint,
    pub primary_flash_size: c_uint,
    pub secondary_flash_offset: c_uint,
    pub secondary_flash_size: c_uint,
    pub mci_offset: c_uint,
    pub mci_size: c_uint,
    pub dma_offset: c_uint,
    pub dma_size: c_uint,
    pub mbox_offset: c_uint,
    pub mbox_size: c_uint,
    pub soc_offset: c_uint,
    pub soc_size: c_uint,
    pub otp_offset: c_uint,
    pub otp_size: c_uint,
    pub lc_offset: c_uint,
    pub lc_size: c_uint,
}

/// Get the size required to allocate memory for the emulator
/// This allows C code to allocate the right amount of memory
#[no_mangle]
pub extern "C" fn emulator_get_size() -> usize {
    std::mem::size_of::<Emulator>()
}

/// Get the alignment required for the emulator structure
#[no_mangle]
pub extern "C" fn emulator_get_alignment() -> usize {
    std::mem::align_of::<Emulator>()
}

/// Initialize an emulator in the provided memory location
/// 
/// # Arguments
/// * `emulator_memory` - Pointer to allocated memory (must be at least emulator_get_size() bytes)
/// * `config` - Configuration for the emulator
/// 
/// # Returns
/// * `EmulatorError::Success` on success
/// * Appropriate error code on failure
/// 
/// # Safety
/// * `emulator_memory` must point to valid memory of at least `emulator_get_size()` bytes
/// * `emulator_memory` must be properly aligned (use `emulator_get_alignment()`)
/// * `config` must be a valid pointer to a CEmulatorConfig structure
/// * All string pointers in `config` must be valid null-terminated C strings
#[no_mangle]
pub unsafe extern "C" fn emulator_init(
    emulator_memory: *mut CEmulator,
    config: *const CEmulatorConfig,
) -> EmulatorError {
    if emulator_memory.is_null() || config.is_null() {
        return EmulatorError::NullPointer;
    }

    let config = &*config;

    // Convert C strings to Rust strings
    let rom_path = match convert_c_string(config.rom_path) {
        Ok(path) => path,
        Err(_) => return EmulatorError::InvalidArgs,
    };

    let firmware_path = match convert_c_string(config.firmware_path) {
        Ok(path) => path,
        Err(_) => return EmulatorError::InvalidArgs,
    };

    let caliptra_rom_path = match convert_c_string(config.caliptra_rom_path) {
        Ok(path) => path,
        Err(_) => return EmulatorError::InvalidArgs,
    };

    let caliptra_firmware_path = match convert_c_string(config.caliptra_firmware_path) {
        Ok(path) => path,
        Err(_) => return EmulatorError::InvalidArgs,
    };

    let soc_manifest_path = match convert_c_string(config.soc_manifest_path) {
        Ok(path) => path,
        Err(_) => return EmulatorError::InvalidArgs,
    };

    // Build EmulatorArgs
    let args = EmulatorArgs {
        rom: rom_path.into(),
        firmware: firmware_path.into(),
        caliptra_rom: caliptra_rom_path.into(),
        caliptra_firmware: caliptra_firmware_path.into(),
        soc_manifest: soc_manifest_path.into(),
        otp: convert_optional_c_string(config.otp_path).map(|s| s.into()),
        gdb_port: if config.gdb_port == 0 { None } else { Some(config.gdb_port as u16) },
        log_dir: convert_optional_c_string(config.log_dir_path).map(|s| s.into()),
        trace_instr: config.trace_instr != 0,
        stdin_uart: config.stdin_uart != 0,
        _no_stdin_uart: false,
        i3c_port: if config.i3c_port == 0 { None } else { Some(config.i3c_port as u16) },
        manufacturing_mode: config.manufacturing_mode != 0,
        vendor_pk_hash: convert_optional_c_string(config.vendor_pk_hash),
        owner_pk_hash: convert_optional_c_string(config.owner_pk_hash),
        streaming_boot: convert_optional_c_string(config.streaming_boot_path).map(|s| s.into()),
        primary_flash_image: convert_optional_c_string(config.primary_flash_image_path).map(|s| s.into()),
        secondary_flash_image: convert_optional_c_string(config.secondary_flash_image_path).map(|s| s.into()),
        hw_revision: semver::Version::new(
            config.hw_revision_major as u64,
            config.hw_revision_minor as u64,
            config.hw_revision_patch as u64,
        ),
        // Use provided offset and size override parameters (0 means use default)
        rom_offset: convert_optional_offset_size(config.rom_offset),
        rom_size: convert_optional_offset_size(config.rom_size),
        uart_offset: convert_optional_offset_size(config.uart_offset),
        uart_size: convert_optional_offset_size(config.uart_size),
        ctrl_offset: convert_optional_offset_size(config.ctrl_offset),
        ctrl_size: convert_optional_offset_size(config.ctrl_size),
        spi_offset: convert_optional_offset_size(config.spi_offset),
        spi_size: convert_optional_offset_size(config.spi_size),
        sram_offset: convert_optional_offset_size(config.sram_offset),
        sram_size: convert_optional_offset_size(config.sram_size),
        pic_offset: convert_optional_offset_size(config.pic_offset),
        external_test_sram_offset: convert_optional_offset_size(config.external_test_sram_offset),
        external_test_sram_size: convert_optional_offset_size(config.external_test_sram_size),
        dccm_offset: convert_optional_offset_size(config.dccm_offset),
        dccm_size: convert_optional_offset_size(config.dccm_size),
        i3c_offset: convert_optional_offset_size(config.i3c_offset),
        i3c_size: convert_optional_offset_size(config.i3c_size),
        primary_flash_offset: convert_optional_offset_size(config.primary_flash_offset),
        primary_flash_size: convert_optional_offset_size(config.primary_flash_size),
        secondary_flash_offset: convert_optional_offset_size(config.secondary_flash_offset),
        secondary_flash_size: convert_optional_offset_size(config.secondary_flash_size),
        mci_offset: convert_optional_offset_size(config.mci_offset),
        mci_size: convert_optional_offset_size(config.mci_size),
        dma_offset: convert_optional_offset_size(config.dma_offset),
        dma_size: convert_optional_offset_size(config.dma_size),
        mbox_offset: convert_optional_offset_size(config.mbox_offset),
        mbox_size: convert_optional_offset_size(config.mbox_size),
        soc_offset: convert_optional_offset_size(config.soc_offset),
        soc_size: convert_optional_offset_size(config.soc_size),
        otp_offset: convert_optional_offset_size(config.otp_offset),
        otp_size: convert_optional_offset_size(config.otp_size),
        lc_offset: convert_optional_offset_size(config.lc_offset),
        lc_size: convert_optional_offset_size(config.lc_size),
    };

    // Create the emulator
    let emulator = match Emulator::from_args(args, config.capture_uart_output != 0) {
        Ok(emu) => emu,
        Err(_) => return EmulatorError::InitializationFailed,
    };

    // Place the emulator in the provided memory
    let emulator_ptr = emulator_memory as *mut Emulator;
    ptr::write(emulator_ptr, emulator);

    EmulatorError::Success
}

/// Step the emulator once
/// 
/// # Arguments
/// * `emulator_memory` - Pointer to the initialized emulator
/// 
/// # Returns
/// * Step action result
/// 
/// # Safety
/// * `emulator_memory` must point to a valid, initialized emulator
#[no_mangle]
pub unsafe extern "C" fn emulator_step(emulator_memory: *mut CEmulator) -> CStepAction {
    if emulator_memory.is_null() {
        return CStepAction::ExitFailure;
    }

    let emulator_ptr = emulator_memory as *mut Emulator;
    let emulator = &mut *emulator_ptr;
    let action = emulator.step();
    action.into()
}

/// Destroy the emulator and clean up resources
/// 
/// # Arguments
/// * `emulator_memory` - Pointer to the initialized emulator
/// 
/// # Safety
/// * `emulator_memory` must point to a valid, initialized emulator
/// * After calling this function, the emulator memory should not be used
#[no_mangle]
pub unsafe extern "C" fn emulator_destroy(emulator_memory: *mut CEmulator) {
    if !emulator_memory.is_null() {
        let emulator_ptr = emulator_memory as *mut Emulator;
        ptr::drop_in_place(emulator_ptr);
    }
}

/// Get UART output if it was captured
/// 
/// # Arguments
/// * `emulator_memory` - Pointer to the initialized emulator
/// * `output_buffer` - Buffer to write the output to
/// * `buffer_size` - Size of the output buffer
/// 
/// # Returns
/// * Number of bytes written to the buffer, or -1 if no output available
/// 
/// # Safety
/// * `emulator_memory` must point to a valid, initialized emulator
/// * `output_buffer` must be a valid buffer of at least `buffer_size` bytes
#[no_mangle]
pub unsafe extern "C" fn emulator_get_uart_output(
    emulator_memory: *mut CEmulator,
    output_buffer: *mut c_char,
    buffer_size: usize,
) -> c_int {
    if emulator_memory.is_null() || output_buffer.is_null() || buffer_size == 0 {
        return -1;
    }

    let emulator_ptr = emulator_memory as *mut Emulator;
    let emulator = &mut *emulator_ptr;

    if let Some(ref uart_output) = emulator.uart_output {
        let uart_data = uart_output.borrow();
        let copy_len = std::cmp::min(uart_data.len(), buffer_size - 1);
        
        if copy_len > 0 {
            ptr::copy_nonoverlapping(
                uart_data.as_ptr() as *const c_char,
                output_buffer,
                copy_len,
            );
        }
        
        // Null terminate
        *output_buffer.add(copy_len) = 0;
        copy_len as c_int
    } else {
        -1
    }
}

// Helper functions

unsafe fn convert_c_string(c_str: *const c_char) -> Result<String, std::str::Utf8Error> {
    if c_str.is_null() {
        return Ok(String::new());
    }
    let cstr = CStr::from_ptr(c_str);
    cstr.to_str().map(|s| s.to_owned())
}

unsafe fn convert_optional_c_string(c_str: *const c_char) -> Option<String> {
    if c_str.is_null() {
        None
    } else {
        convert_c_string(c_str).ok()
    }
}

fn convert_optional_offset_size(value: c_uint) -> Option<u32> {
    if value == 0 {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_and_alignment() {
        // Ensure we can get size and alignment
        let size = emulator_get_size();
        let align = emulator_get_alignment();
        
        assert!(size > 0);
        assert!(align > 0);
        assert!(align.is_power_of_two());
    }
}
