# Emulator C Bindings

This crate provides C bindings for the Caliptra MCU Emulator, allowing the emulator to be used from C code while maintaining the emulator's lifetime in C-allocated memory.

## Features

- **Zero-copy integration**: The emulator state is stored in C-allocated memory
- **Static library output**: Compiles to a static library for easy integration
- **Memory safety**: Proper lifetime management with C-controlled allocation
- **No changes to emulator.rs**: The original emulator code remains untouched
- **Full configuration support**: All emulator configuration options are exposed

## Building

### Prerequisites

- Rust toolchain
- `cbindgen` for header generation (automatically installed as build dependency)
- C compiler (gcc/clang)

### Build the static library

```bash
cd emulator/cbinding
cargo build --release
```

This will generate:
- `target/release/libemulator_cbinding.a` - Static library
- `emulator_cbinding.h` - C header file (generated during build)

### Build the example

```bash
make example
```

Or manually:
```bash
gcc -std=c11 -Wall -Wextra -O2 \
    -I. \
    -o example \
    example.c \
    -L./target/release \
    -lemulator_cbinding \
    -lpthread -ldl -lm
```

## Usage

### 1. Memory Allocation

The C code is responsible for allocating memory for the emulator:

```c
#include "emulator_cbinding.h"

// Get memory requirements
size_t size = emulator_get_size();
size_t alignment = emulator_get_alignment();

// Allocate aligned memory
void* emulator_memory = aligned_alloc(alignment, size);
```

### 2. Configuration

Configure the emulator using the `CEmulatorConfig` structure:

```c
CEmulatorConfig config = {
    .rom_path = "path/to/rom.bin",
    .firmware_path = "path/to/firmware.bin",
    .caliptra_rom_path = "path/to/caliptra_rom.bin",
    .caliptra_firmware_path = "path/to/caliptra_firmware.bin",
    .soc_manifest_path = "path/to/soc_manifest.bin",
    .otp_path = NULL,                    // Optional
    .log_dir_path = NULL,               // Optional
    .gdb_port = 0,                      // 0 = disabled
    .i3c_port = 0,                      // 0 = disabled
    .trace_instr = 0,                   // 0 = false, 1 = true
    .stdin_uart = 0,                    // 0 = false, 1 = true
    .manufacturing_mode = 0,            // 0 = false, 1 = true
    .capture_uart_output = 1,           // 0 = false, 1 = true
    .vendor_pk_hash = NULL,             // Optional
    .owner_pk_hash = NULL,              // Optional
    .streaming_boot_path = NULL,        // Optional
    .primary_flash_image_path = NULL,   // Optional
    .secondary_flash_image_path = NULL, // Optional
    .hw_revision_major = 2,
    .hw_revision_minor = 0,
    .hw_revision_patch = 0,
    
    // Memory layout overrides (0 = use defaults)
    .rom_offset = 0,                    // Custom ROM base address
    .rom_size = 0,                      // Custom ROM size
    .uart_offset = 0,                   // Custom UART base address
    .uart_size = 0,                     // Custom UART size
    .ctrl_offset = 0,                   // Custom control register base
    .ctrl_size = 0,                     // Custom control register size
    .spi_offset = 0,                    // Custom SPI base address
    .spi_size = 0,                      // Custom SPI size
    .sram_offset = 0,                   // Custom SRAM base address
    .sram_size = 0,                     // Custom SRAM size
    .pic_offset = 0,                    // Custom PIC base address
    .external_test_sram_offset = 0,     // Custom external test SRAM base
    .external_test_sram_size = 0,       // Custom external test SRAM size
    .dccm_offset = 0,                   // Custom DCCM base address
    .dccm_size = 0,                     // Custom DCCM size
    .i3c_offset = 0,                    // Custom I3C base address
    .i3c_size = 0,                      // Custom I3C size
    .primary_flash_offset = 0,          // Custom primary flash base
    .primary_flash_size = 0,            // Custom primary flash size
    .secondary_flash_offset = 0,        // Custom secondary flash base
    .secondary_flash_size = 0,          // Custom secondary flash size
    .mci_offset = 0,                    // Custom MCI base address
    .mci_size = 0,                      // Custom MCI size
    .dma_offset = 0,                    // Custom DMA base address
    .dma_size = 0,                      // Custom DMA size
    .mbox_offset = 0,                   // Custom mailbox base address
    .mbox_size = 0,                     // Custom mailbox size
    .soc_offset = 0,                    // Custom SoC interface base
    .soc_size = 0,                      // Custom SoC interface size
    .otp_offset = 0,                    // Custom OTP base address
    .otp_size = 0,                      // Custom OTP size
    .lc_offset = 0,                     // Custom LC base address
    .lc_size = 0,                       // Custom LC size
};
```

### 3. Initialization

Initialize the emulator in the allocated memory:

```c
EmulatorError result = emulator_init((CEmulator*)emulator_memory, &config);
if (result != Success) {
    // Handle error
    free(emulator_memory);
    return -1;
}
```

### 4. Execution

Step the emulator in a loop:

```c
CStepAction action;
do {
    action = emulator_step((CEmulator*)emulator_memory);
    
    // Handle different step actions
    switch (action) {
        case Continue:
            // Emulator continues normally
            break;
        case Break:
            // Breakpoint or debug break
            break;
        case ExitSuccess:
            // Emulator exited successfully
            break;
        case ExitFailure:
            // Emulator exited with error
            break;
    }
} while (action == Continue);
```

### 5. UART Output

If UART output capture is enabled, retrieve it periodically:

```c
char uart_buffer[1024];
int bytes_read = emulator_get_uart_output(
    (CEmulator*)emulator_memory,
    uart_buffer,
    sizeof(uart_buffer)
);

if (bytes_read > 0) {
    printf("UART: %s\n", uart_buffer);
}
```

### 6. Cleanup

Always clean up when done:

```c
emulator_destroy((CEmulator*)emulator_memory);
free(emulator_memory);
```

## Error Handling

The API uses error codes for error handling:

```c
typedef enum {
    Success = 0,
    InvalidArgs = -1,
    InitializationFailed = -2,
    NullPointer = -3,
    InvalidEmulator = -4,
} EmulatorError;
```

## Step Actions

The emulator step function returns action codes:

```c
typedef enum {
    Continue = 0,
    Break = 1,
    ExitSuccess = 2,
    ExitFailure = 3,
} CStepAction;
```

## Configuration Options

The `CEmulatorConfig` structure exposes all emulator configuration:

- **Required paths**: ROM, firmware, Caliptra ROM/firmware, SoC manifest
- **Optional paths**: OTP file, log directory, flash images
- **Network**: GDB port, I3C port
- **Behavior**: Instruction tracing, UART capture, manufacturing mode
- **Hardware**: Version numbers, address overrides

### Memory Layout Customization

All memory layout parameters support custom offset and size values:

- **Set to 0**: Use default values from the emulator
- **Set to non-zero**: Override with custom values

Supported memory regions:
- **ROM**: `rom_offset`, `rom_size`
- **UART**: `uart_offset`, `uart_size`
- **Control Registers**: `ctrl_offset`, `ctrl_size`
- **SPI**: `spi_offset`, `spi_size`
- **SRAM**: `sram_offset`, `sram_size`
- **PIC**: `pic_offset`
- **External Test SRAM**: `external_test_sram_offset`, `external_test_sram_size`
- **DCCM**: `dccm_offset`, `dccm_size`
- **I3C**: `i3c_offset`, `i3c_size`
- **Primary Flash**: `primary_flash_offset`, `primary_flash_size`
- **Secondary Flash**: `secondary_flash_offset`, `secondary_flash_size`
- **MCI**: `mci_offset`, `mci_size`
- **DMA**: `dma_offset`, `dma_size`
- **Mailbox**: `mbox_offset`, `mbox_size`
- **SoC Interface**: `soc_offset`, `soc_size`
- **OTP**: `otp_offset`, `otp_size`
- **LC (Lifecycle)**: `lc_offset`, `lc_size`

Example of custom memory layout:
```c
CEmulatorConfig config = {
    // ... other configuration ...
    .rom_offset = 0x10000000,      // Custom ROM at 256MB
    .rom_size = 0x100000,          // 1MB ROM size
    .sram_offset = 0x20000000,     // Custom SRAM at 512MB
    .sram_size = 0x800000,         // 8MB SRAM size
    // ... other parameters set to 0 for defaults ...
};
```

## Integration Notes

### Linking

When linking with the static library, you may need additional system libraries:

```bash
-lemulator_cbinding -lpthread -ldl -lm
```

### Memory Alignment

Always use `emulator_get_alignment()` to ensure proper memory alignment. Improper alignment can cause crashes or undefined behavior.

### Thread Safety

The emulator is not thread-safe. If using in a multi-threaded environment, ensure proper synchronization.

### Platform Support

The bindings support the same platforms as the underlying Rust emulator:
- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)  
- Windows (x86_64)

## Example

See `example.c` for a complete working example that demonstrates:
- Memory allocation and configuration
- Emulator initialization and execution
- UART output capture
- Proper cleanup

Run the example:
```bash
./example rom.bin firmware.bin caliptra_rom.bin caliptra_firmware.bin soc_manifest.bin
```
