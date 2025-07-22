#ifndef EMULATOR_CBINDING_H
#define EMULATOR_CBINDING_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Step action results for C API
 */
typedef enum CStepAction {
  Continue = 0,
  Break = 1,
  ExitSuccess = 2,
  ExitFailure = 3,
} CStepAction;

/**
 * Error codes for C API
 */
typedef enum EmulatorError {
  Success = 0,
  InvalidArgs = -1,
  InitializationFailed = -2,
  NullPointer = -3,
  InvalidEmulator = -4,
} EmulatorError;

/**
 * Opaque structure representing the emulator
 * C code should allocate memory for this structure
 */
typedef struct CEmulator {
  uint8_t _private[0];
} CEmulator;

/**
 * Configuration structure for emulator initialization
 */
typedef struct CEmulatorConfig {
  const char *rom_path;
  const char *firmware_path;
  const char *caliptra_rom_path;
  const char *caliptra_firmware_path;
  const char *soc_manifest_path;
  const char *otp_path;
  const char *log_dir_path;
  unsigned int gdb_port;
  unsigned int i3c_port;
  unsigned char trace_instr;
  unsigned char stdin_uart;
  unsigned char manufacturing_mode;
  unsigned char capture_uart_output;
  const char *vendor_pk_hash;
  const char *owner_pk_hash;
  const char *streaming_boot_path;
  const char *primary_flash_image_path;
  const char *secondary_flash_image_path;
  unsigned int hw_revision_major;
  unsigned int hw_revision_minor;
  unsigned int hw_revision_patch;
  unsigned int rom_offset;
  unsigned int rom_size;
  unsigned int uart_offset;
  unsigned int uart_size;
  unsigned int ctrl_offset;
  unsigned int ctrl_size;
  unsigned int spi_offset;
  unsigned int spi_size;
  unsigned int sram_offset;
  unsigned int sram_size;
  unsigned int pic_offset;
  unsigned int external_test_sram_offset;
  unsigned int external_test_sram_size;
  unsigned int dccm_offset;
  unsigned int dccm_size;
  unsigned int i3c_offset;
  unsigned int i3c_size;
  unsigned int primary_flash_offset;
  unsigned int primary_flash_size;
  unsigned int secondary_flash_offset;
  unsigned int secondary_flash_size;
  unsigned int mci_offset;
  unsigned int mci_size;
  unsigned int dma_offset;
  unsigned int dma_size;
  unsigned int mbox_offset;
  unsigned int mbox_size;
  unsigned int soc_offset;
  unsigned int soc_size;
  unsigned int otp_offset;
  unsigned int otp_size;
  unsigned int lc_offset;
  unsigned int lc_size;
} CEmulatorConfig;

/**
 * Get the size required to allocate memory for the emulator
 * This allows C code to allocate the right amount of memory
 */
uintptr_t emulator_get_size(void);

/**
 * Get the alignment required for the emulator structure
 */
uintptr_t emulator_get_alignment(void);

/**
 * Initialize an emulator in the provided memory location
 *
 * # Arguments
 * * `emulator_memory` - Pointer to allocated memory (must be at least emulator_get_size() bytes)
 * * `config` - Configuration for the emulator
 *
 * # Returns
 * * `EmulatorError::Success` on success
 * * Appropriate error code on failure
 *
 * # Safety
 * * `emulator_memory` must point to valid memory of at least `emulator_get_size()` bytes
 * * `emulator_memory` must be properly aligned (use `emulator_get_alignment()`)
 * * `config` must be a valid pointer to a CEmulatorConfig structure
 * * All string pointers in `config` must be valid null-terminated C strings
 */
enum EmulatorError emulator_init(struct CEmulator *emulator_memory,
                                 const struct CEmulatorConfig *config);

/**
 * Step the emulator once
 *
 * This function works in both normal and GDB modes:
 * - **Normal mode**: Steps the emulator directly
 * - **GDB mode**: Steps the underlying emulator, allowing C to control execution
 *   while GDB server is available for debugging/inspection
 *
 * # Arguments
 * * `emulator_memory` - Pointer to the initialized emulator
 *
 * # Returns
 * * Step action result
 *
 * # Safety
 * * `emulator_memory` must point to a valid, initialized emulator
 */
enum CStepAction emulator_step(struct CEmulator *emulator_memory);

/**
 * Destroy the emulator and clean up resources
 *
 * # Arguments
 * * `emulator_memory` - Pointer to the initialized emulator
 *
 * # Safety
 * * `emulator_memory` must point to a valid, initialized emulator
 * * After calling this function, the emulator memory should not be used
 */
void emulator_destroy(struct CEmulator *emulator_memory);

/**
 * Get UART output if it was captured
 *
 * # Arguments
 * * `emulator_memory` - Pointer to the initialized emulator
 * * `output_buffer` - Buffer to write the output to
 * * `buffer_size` - Size of the output buffer
 *
 * # Returns
 * * Number of bytes written to the buffer, or -1 if no output available
 *
 * # Safety
 * * `emulator_memory` must point to a valid, initialized emulator
 * * `output_buffer` must be a valid buffer of at least `buffer_size` bytes
 */
int emulator_get_uart_output(struct CEmulator *emulator_memory,
                             char *output_buffer,
                             uintptr_t buffer_size);

/**
 * Start GDB server and wait for connection (blocking)
 * This function should only be called if the emulator was initialized with a GDB port.
 *
 * IMPORTANT: There are two ways to use GDB mode:
 *
 * 1. **GDB-controlled execution**: Call this function and let GDB control all stepping.
 *    The GDB server will handle all emulator execution and stepping commands.
 *    Do NOT call emulator_step() while this function is running.
 *
 * 2. **C-controlled execution with GDB debugging**: DON'T call this function.
 *    Instead, call emulator_step() normally to control execution from C.
 *    Connect GDB to the port and use GDB for debugging/inspection only.
 *    In this mode, GDB can inspect state but C controls when steps happen.
 *
 * # Arguments
 * * `emulator_memory` - Pointer to the initialized emulator in GDB mode
 *
 * # Returns
 * * `EmulatorError::Success` when GDB session ends normally
 * * Appropriate error code on failure
 *
 * # Safety
 * * `emulator_memory` must point to a valid, initialized emulator in GDB mode
 */
enum EmulatorError emulator_run_gdb_server(struct CEmulator *emulator_memory);

/**
 * Check if the emulator is in GDB mode
 *
 * # Arguments
 * * `emulator_memory` - Pointer to the initialized emulator
 *
 * # Returns
 * * 1 if in GDB mode, 0 if in normal mode
 *
 * # Safety
 * * `emulator_memory` must point to a valid, initialized emulator
 */
int emulator_is_gdb_mode(struct CEmulator *emulator_memory);

/**
 * Get the GDB port if the emulator is in GDB mode
 *
 * # Arguments
 * * `emulator_memory` - Pointer to the initialized emulator
 *
 * # Returns
 * * GDB port number, or 0 if not in GDB mode
 *
 * # Safety
 * * `emulator_memory` must point to a valid, initialized emulator
 */
unsigned int emulator_get_gdb_port(struct CEmulator *emulator_memory);

/**
 * Get the current program counter (PC) of the MCU CPU
 *
 * # Arguments
 * * `emulator_memory` - Pointer to the initialized emulator
 *
 * # Returns
 * * Current PC value of the MCU CPU
 *
 * # Safety
 * * `emulator_memory` must point to a valid, initialized emulator
 */
unsigned int get_pc(struct CEmulator *emulator_memory);

#endif /* EMULATOR_CBINDING_H */
