/*++

Licensed under the Apache-2.0 license.

File Name:

    lib.rs

Abstract:

    C bindings for the Caliptra MCU Emulator.

--*/

use emulator::{Emulator, EmulatorArgs, ExternalReadCallback, ExternalWriteCallback, gdb, EMULATOR_RUNNING};
use caliptra_emu_cpu::StepAction;
use caliptra_emu_types::RvSize;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uint, c_uchar, c_longlong};
use std::ptr;
use std::sync::atomic::Ordering;

#[cfg(test)]
mod simple_test;

/// Internal emulator wrapper that can be in normal or GDB mode
enum EmulatorWrapper {
    Normal(Emulator),
    Gdb(gdb::gdb_target::GdbTarget),
}

/// Internal state for the C emulator instance
struct CEmulatorState {
    wrapper: EmulatorWrapper,
    gdb_port: Option<u16>, // Store GDB port for later use
}

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
            StepAction::Fatal => CStepAction::ExitFailure,
        }
    }
}

/// C function pointer type for external read callbacks
/// 
/// # Arguments
/// * `context` - Context pointer passed to the callback
/// * `size` - Size of the read operation (1, 2, or 4 bytes)
/// * `addr` - Address being read from  
/// * `buffer` - Pointer to write the read data to
/// 
/// # Returns
/// * 1 for success, 0 for failure
pub type CExternalReadCallback = unsafe extern "C" fn(
    context: *const std::ffi::c_void,  // Context pointer
    size: c_uint,    // RvSize as u32
    addr: c_uint,    // RvAddr as u32
    buffer: *mut c_uint,  // Output buffer for read data
) -> c_int;

/// C function pointer type for external write callbacks
/// 
/// # Arguments
/// * `context` - Context pointer passed to the callback
/// * `size` - Size of the write operation (1, 2, or 4 bytes)
/// * `addr` - Address being written to
/// * `data` - Data being written
/// 
/// # Returns
/// * 1 for success, 0 for failure
pub type CExternalWriteCallback = unsafe extern "C" fn(
    context: *const std::ffi::c_void,  // Context pointer
    size: c_uint,    // RvSize as u32
    addr: c_uint,    // RvAddr as u32
    data: c_uint,    // RvData as u32
) -> c_int;

/// Opaque structure representing the emulator
/// C code should allocate memory for this structure
#[repr(C)]
pub struct CEmulator {
    _private: [u8; 0],
}

/// Configuration structure for emulator initialization
/// 
/// Memory layout override parameters use int64_t values where:
/// - `-1` means use the default value
/// - Valid positive values (0 to UINT32_MAX) will be used as-is
/// - Invalid values (negative except -1, or > UINT32_MAX) will be treated as default
/// 
/// Example usage in C:
/// ```c
/// CEmulatorConfig config = {
///     .rom_path = "rom.bin",
///     .firmware_path = "firmware.bin",
///     // ... other required fields ...
///     .rom_offset = 0x40000000,  // Use custom ROM offset
///     .rom_size = -1,            // Use default ROM size
///     .sram_offset = -1,         // Use default SRAM offset
///     .sram_size = 0x100000,     // Use custom SRAM size (1MB)
///     // ... other memory layout fields all set to -1 for defaults ...
/// };
/// ```
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
    
    // Memory layout override parameters (-1 means use default)
    pub rom_offset: c_longlong,
    pub rom_size: c_longlong,
    pub uart_offset: c_longlong,
    pub uart_size: c_longlong,
    pub ctrl_offset: c_longlong,
    pub ctrl_size: c_longlong,
    pub spi_offset: c_longlong,
    pub spi_size: c_longlong,
    pub sram_offset: c_longlong,
    pub sram_size: c_longlong,
    pub pic_offset: c_longlong,
    pub external_test_sram_offset: c_longlong,
    pub external_test_sram_size: c_longlong,
    pub dccm_offset: c_longlong,
    pub dccm_size: c_longlong,
    pub i3c_offset: c_longlong,
    pub i3c_size: c_longlong,
    pub primary_flash_offset: c_longlong,
    pub primary_flash_size: c_longlong,
    pub secondary_flash_offset: c_longlong,
    pub secondary_flash_size: c_longlong,
    pub mci_offset: c_longlong,
    pub mci_size: c_longlong,
    pub dma_offset: c_longlong,
    pub dma_size: c_longlong,
    pub mbox_offset: c_longlong,
    pub mbox_size: c_longlong,
    pub soc_offset: c_longlong,
    pub soc_size: c_longlong,
    pub otp_offset: c_longlong,
    pub otp_size: c_longlong,
    pub lc_offset: c_longlong,
    pub lc_size: c_longlong,
    
    // External device callbacks (can be null)
    pub external_read_callback: *const std::ffi::c_void,
    pub external_write_callback: *const std::ffi::c_void,
    pub callback_context: *const std::ffi::c_void,  // Context pointer for callbacks
}

/// Get the size required to allocate memory for the emulator
/// This allows C code to allocate the right amount of memory
#[no_mangle]
pub extern "C" fn emulator_get_size() -> usize {
    std::mem::size_of::<CEmulatorState>()
}

/// Get the alignment required for the emulator structure
#[no_mangle]
pub extern "C" fn emulator_get_alignment() -> usize {
    std::mem::align_of::<CEmulatorState>()
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
        // Use provided offset and size override parameters (-1 means use default)
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

    // Convert C callbacks to Rust callbacks if provided
    let read_callback = if config.external_read_callback.is_null() {
        None
    } else {
        let c_callback: CExternalReadCallback = unsafe {
            std::mem::transmute(config.external_read_callback)
        };
        let context = config.callback_context;
        Some(convert_c_read_callback(c_callback, context))
    };
    
    let write_callback = if config.external_write_callback.is_null() {
        None
    } else {
        let c_callback: CExternalWriteCallback = unsafe {
            std::mem::transmute(config.external_write_callback)
        };
        let context = config.callback_context;
        Some(convert_c_write_callback(c_callback, context))
    };

    println!("args: {:?}", args);
    // Create the emulator with callbacks
    let emulator = match Emulator::from_args_with_callbacks(
        args, 
        config.capture_uart_output != 0,
        read_callback,
        write_callback
    ) {
        Ok(emu) => emu,
        Err(_) => return EmulatorError::InitializationFailed,
    };

    // Determine if we should be in GDB mode based on config
    let gdb_port = if config.gdb_port == 0 { None } else { Some(config.gdb_port as u16) };
    
    // Create the emulator state - if GDB port specified, start in GDB mode
    let emulator_state = if let Some(port) = gdb_port {
        CEmulatorState {
            wrapper: EmulatorWrapper::Gdb(gdb::gdb_target::GdbTarget::new(emulator)),
            gdb_port: Some(port),
        }
    } else {
        CEmulatorState {
            wrapper: EmulatorWrapper::Normal(emulator),
            gdb_port: None,
        }
    };

    // Place the emulator state in the provided memory
    let emulator_ptr = emulator_memory as *mut CEmulatorState;
    ptr::write(emulator_ptr, emulator_state);

    EmulatorError::Success
}

/// Step the emulator once
/// 
/// This function works in both normal and GDB modes:
/// - **Normal mode**: Steps the emulator directly
/// - **GDB mode**: Steps the underlying emulator, allowing C to control execution
///   while GDB server is available for debugging/inspection
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

    let emulator_ptr = emulator_memory as *mut CEmulatorState;
    let emulator_state = &mut *emulator_ptr;
    
    match &mut emulator_state.wrapper {
        EmulatorWrapper::Normal(emulator) => {
            let action = emulator.step();
            action.into()
        }
        EmulatorWrapper::Gdb(gdb_target) => {
            // In GDB mode, step the underlying emulator directly
            let action = gdb_target.emulator_mut().step();
            action.into()
        }
    }
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
        let emulator_ptr = emulator_memory as *mut CEmulatorState;
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

    let emulator_ptr = emulator_memory as *mut CEmulatorState;
    let emulator_state = &mut *emulator_ptr;

    let uart_output = match &emulator_state.wrapper {
        EmulatorWrapper::Normal(emulator) => &emulator.uart_output,
        EmulatorWrapper::Gdb(gdb_target) => &gdb_target.emulator().uart_output,
    };

    if let Some(ref uart_output_rc) = uart_output {
        let uart_data = uart_output_rc.borrow();
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

/// Start GDB server and wait for connection (blocking)
/// This function should only be called if the emulator was initialized with a GDB port.
/// 
/// IMPORTANT: There are two ways to use GDB mode:
/// 
/// 1. **GDB-controlled execution**: Call this function and let GDB control all stepping.
///    The GDB server will handle all emulator execution and stepping commands.
///    Do NOT call emulator_step() while this function is running.
/// 
/// 2. **C-controlled execution with GDB debugging**: DON'T call this function.
///    Instead, call emulator_step() normally to control execution from C.
///    Connect GDB to the port and use GDB for debugging/inspection only.
///    In this mode, GDB can inspect state but C controls when steps happen.
/// 
/// # Arguments
/// * `emulator_memory` - Pointer to the initialized emulator in GDB mode
/// 
/// # Returns
/// * `EmulatorError::Success` when GDB session ends normally
/// * Appropriate error code on failure
/// 
/// # Safety
/// * `emulator_memory` must point to a valid, initialized emulator in GDB mode
#[no_mangle]
pub unsafe extern "C" fn emulator_run_gdb_server(
    emulator_memory: *mut CEmulator,
) -> EmulatorError {
    if emulator_memory.is_null() {
        return EmulatorError::NullPointer;
    }

    let emulator_ptr = emulator_memory as *mut CEmulatorState;
    let emulator_state = &mut *emulator_ptr;

    match (&mut emulator_state.wrapper, emulator_state.gdb_port) {
        (EmulatorWrapper::Gdb(gdb_target), Some(port)) => {
            gdb::gdb_state::wait_for_gdb_run(gdb_target, port);
            EmulatorError::Success
        }
        (EmulatorWrapper::Normal(_), _) => EmulatorError::InvalidArgs,
        (EmulatorWrapper::Gdb(_), None) => EmulatorError::InvalidArgs,
    }
}

/// Check if the emulator is in GDB mode
/// 
/// # Arguments
/// * `emulator_memory` - Pointer to the initialized emulator
/// 
/// # Returns
/// * 1 if in GDB mode, 0 if in normal mode
/// 
/// # Safety
/// * `emulator_memory` must point to a valid, initialized emulator
#[no_mangle]
pub unsafe extern "C" fn emulator_is_gdb_mode(emulator_memory: *mut CEmulator) -> c_int {
    if emulator_memory.is_null() {
        return 0;
    }

    let emulator_ptr = emulator_memory as *mut CEmulatorState;
    let emulator_state = &*emulator_ptr;
    
    match emulator_state.wrapper {
        EmulatorWrapper::Gdb(_) => 1,
        EmulatorWrapper::Normal(_) => 0,
    }
}

/// Get the GDB port if the emulator is in GDB mode
/// 
/// # Arguments
/// * `emulator_memory` - Pointer to the initialized emulator
/// 
/// # Returns
/// * GDB port number, or 0 if not in GDB mode
/// 
/// # Safety
/// * `emulator_memory` must point to a valid, initialized emulator
#[no_mangle]
pub unsafe extern "C" fn emulator_get_gdb_port(emulator_memory: *mut CEmulator) -> c_uint {
    if emulator_memory.is_null() {
        return 0;
    }

    let emulator_ptr = emulator_memory as *mut CEmulatorState;
    let emulator_state = &*emulator_ptr;
    
    emulator_state.gdb_port.unwrap_or(0) as c_uint
}

/// Get the current program counter (PC) of the MCU CPU
/// 
/// # Arguments
/// * `emulator_memory` - Pointer to the initialized emulator
/// 
/// # Returns
/// * Current PC value of the MCU CPU
/// 
/// # Safety
/// * `emulator_memory` must point to a valid, initialized emulator
#[no_mangle]
pub unsafe extern "C" fn get_pc(emulator_memory: *mut CEmulator) -> c_uint {
    if emulator_memory.is_null() {
        return 0;
    }

    let emulator_ptr = emulator_memory as *mut CEmulatorState;
    let emulator_state = &*emulator_ptr;
    
    match &emulator_state.wrapper {
        EmulatorWrapper::Normal(emulator) => emulator.get_pc(),
        EmulatorWrapper::Gdb(gdb_target) => gdb_target.emulator().get_pc(),
    }
}

/// Trigger an exit request by setting EMULATOR_RUNNING to false
/// This will cause any loops waiting on EMULATOR_RUNNING to exit
/// 
/// # Returns
/// * `EmulatorError::Success` on success
#[no_mangle]
pub extern "C" fn trigger_exit_request() -> EmulatorError {
    EMULATOR_RUNNING.store(false, Ordering::Relaxed);
    EmulatorError::Success
}

/// Example external read callback that returns the address as data
/// This is a simple test callback that C code can use for testing
/// 
/// # Arguments
/// * `context` - Context pointer (unused in this example)
/// * `size` - Size of the read operation (1, 2, or 4 bytes)
/// * `addr` - Address being read from
/// * `buffer` - Pointer to write the read data to
/// 
/// # Returns
/// * 1 for success
#[no_mangle]
pub unsafe extern "C" fn example_external_read_callback(
    _context: *const std::ffi::c_void,
    _size: c_uint,
    addr: c_uint,
    buffer: *mut c_uint,
) -> c_int {
    if buffer.is_null() {
        return 0;
    }
    
    // Simple example: return the address as the read data
    *buffer = addr;
    1 // Success
}

/// Example external write callback that logs the operation
/// This is a simple test callback that C code can use for testing
/// 
/// # Arguments  
/// * `context` - Context pointer (unused in this example)
/// * `size` - Size of the write operation (1, 2, or 4 bytes)
/// * `addr` - Address being written to
/// * `data` - Data being written
/// 
/// # Returns
/// * 1 for success
#[no_mangle]
pub unsafe extern "C" fn example_external_write_callback(
    _context: *const std::ffi::c_void,
    size: c_uint,
    addr: c_uint,
    data: c_uint,
) -> c_int {
    println!("External write: size={}, addr=0x{:08x}, data=0x{:08x}", size, addr, data);
    1 // Success
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

/// Convert C external read callback to Rust callback
fn convert_c_read_callback(c_callback: CExternalReadCallback, context: *const std::ffi::c_void) -> ExternalReadCallback {
    Box::new(move |size, addr, buffer| {
        // Convert RvSize to u32
        let size_u32 = match size {
            RvSize::Byte => 1,
            RvSize::HalfWord => 2,
            RvSize::Word => 4,
            RvSize::Invalid => return false, // Invalid size
        };
        
        let result = unsafe { c_callback(context, size_u32, addr, buffer as *mut c_uint) };
        result != 0
    })
}

/// Convert C external write callback to Rust callback  
fn convert_c_write_callback(c_callback: CExternalWriteCallback, context: *const std::ffi::c_void) -> ExternalWriteCallback {
    Box::new(move |size, addr, data| {
        // Convert RvSize to u32
        let size_u32 = match size {
            RvSize::Byte => 1,
            RvSize::HalfWord => 2,
            RvSize::Word => 4,
            RvSize::Invalid => return false, // Invalid size
        };
        
        let result = unsafe { c_callback(context, size_u32, addr, data) };
        result != 0
    })
}

pub(crate) fn convert_optional_offset_size(value: c_longlong) -> Option<u32> {
    if value == -1 {
        None
    } else {
        // Convert to u32, but validate range
        if value < 0 || value > u32::MAX as c_longlong {
            None // Invalid range, treat as default
        } else {
            Some(value as u32)
        }
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
