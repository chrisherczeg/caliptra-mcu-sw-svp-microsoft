# Emulator C Bindings

This crate provides C bindings for the Caliptra MCU Emulator, allowing the emulator to be used from C code while maintaining the emulator's lifetime in C-allocated memory.

## Features

- **Ze#### Connecting with GDB**: The emula#### Important Notes

- When GDB port is set (non-zero), the emulator starts in GDB mode
- `emulator_step()` works in both normal and GDB modes
- In GDB mode, you can choose between C-controlled stepping or GDB-controlled execution
- The GDB server runs on the specified port and accepts standard GDB remote protocol commands
- Use `emulator_run_gdb_server()` for blocking GDB sessions where GDB controls execution
- Use repeated `emulator_step()` calls when you want C code to control execution pacee is stored in C-allocated memory
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
size_t size = emulator_size_required();

// Allocate aligned memory (8-byte alignment)
void* emulator_memory = aligned_alloc(8, size);
```

### 2. Configuration

Configure the emulator using the `EmulatorArgs` structure:

```c
EmulatorArgs args = {
    .soc_address_hi = 0x00000000,      // High 32 bits of SoC address
    .soc_address_lo = 0x40000000,      // Low 32 bits of SoC address
    .soc_size = 0x800000,              // SoC memory size (8MB)
    .uc_address_hi = 0x00000000,       // High 32 bits of uC address
    .uc_address_lo = 0x50000000,       // Low 32 bits of uC address  
    .uc_size = 0x10000,                // uC memory size (64KB)
    .soc_manifest = "/path/to/manifest", // Path to SoC manifest file
    .gdb_port = 0                      // GDB port (0 = disabled)
};
```

### 3. Initialization

Initialize the emulator in the allocated memory:

```c
EmulatorError result = emulator_init((struct CEmulator*)emulator_memory, &args);
if (result != Success) {
    // Handle error
    free(emulator_memory);
    return -1;
}
```

### 4. Execution

Step the emulator in a loop:

```c
EmulatorError result;
do {
    result = emulator_step((struct CEmulator*)emulator_memory);
    
    // Handle different results
    switch (result) {
        case Success:
            // Emulator step completed successfully
            break;
        case StepComplete:
            // Emulator finished execution
            break;
        default:
            // Error occurred
            break;
    }
} while (result == Success);
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
emulator_destroy((struct CEmulator*)emulator_memory);
free(emulator_memory);
```

## GDB Integration

The emulator supports GDB debugging through a built-in GDB server. When enabled, you can connect with `gdb` to debug the emulated firmware.

### Setting up GDB Mode

```c
// Configure emulator with GDB support
EmulatorArgs args = {
    .soc_address_hi = 0x00000000,
    .soc_address_lo = 0x40000000,
    .soc_size = 0x800000,
    .uc_address_hi = 0x00000000,
    .uc_address_lo = 0x50000000,
    .uc_size = 0x10000,
    .soc_manifest = "/path/to/manifest",
    .gdb_port = 3333  // Enable GDB on port 3333
};

EmulatorError result = emulator_init(memory, &args);
```

### GDB Usage Patterns

**Pattern 1: C-Controlled Execution with GDB Available**
```c
// Initialize emulator with GDB port
emulator_init(memory, &args);

// Check if we're in GDB mode
if (emulator_is_gdb_mode(memory)) {
    printf("GDB server available on port %u\n", emulator_get_gdb_port(memory));
    printf("Connect with: gdb -ex 'target remote :3333'\n");
}

// Your C code controls execution stepping
for (int i = 0; i < 1000; i++) {
    result = emulator_step(memory);  // Works in both normal and GDB modes
    if (result != Success) {
        break;
    }
    
    // You can still get UART output, check state, etc.
    char output[256];
    int len = emulator_get_uart_output(memory, output, sizeof(output));
    if (len > 0) {
        printf("UART: %.*s\n", len, output);
    }
}
```

**Pattern 2: GDB-Controlled Execution**
```c
// Initialize emulator with GDB port
emulator_init(memory, &args);

if (emulator_is_gdb_mode(memory)) {
    printf("Starting GDB server on port %u\n", emulator_get_gdb_port(memory));
    printf("Connect with: gdb -ex 'target remote :%u'\n", emulator_get_gdb_port(memory));
    
    // This will block until GDB session ends
    EmulatorError result = emulator_run_gdb_server(memory);
    
    if (result == Success) {
        printf("GDB session completed\n");
    } else {
        printf("GDB session failed\n");
    }
}
```

**Pattern 3: Hybrid Control**
```c
// Start with C-controlled stepping for initialization
emulator_init(memory, &args);

// Run some initialization steps under C control
for (int i = 0; i < 100; i++) {
    emulator_step(memory);
}

printf("Initialization complete. Starting GDB server...\n");

// Then hand over control to GDB for debugging
if (emulator_is_gdb_mode(memory)) {
    emulator_run_gdb_server(memory);
}
```

**Pattern 3: Hybrid Control**
```c
// Start with C-controlled stepping for initialization
emulator_init(memory, &args);

// Run some initialization steps under C control
for (int i = 0; i < 100; i++) {
    emulator_step(memory);
}

printf("Initialization complete. Starting GDB server...\n");

// Then hand over control to GDB for debugging
if (emulator_is_gdb_mode(memory)) {
    emulator_run_gdb_server(memory);
}
```

### Connecting with GDB

Once the emulator is running with GDB support:

```bash
# Start gdb with your firmware binary
gdb firmware.elf

# Connect to the emulator
(gdb) target remote :3333

# Now you can use standard gdb commands:
(gdb) break main
(gdb) continue
(gdb) step
(gdb) info registers
(gdb) x/10x $sp
```

### Important Notes

- When GDB port is set (non-zero), the emulator starts in GDB mode
- `emulator_step()` works in both normal and GDB modes
- In GDB mode, you can choose between C-controlled stepping or GDB-controlled execution
- The GDB server runs on the specified port and accepts standard GDB remote protocol commands
- Use `emulator_run_gdb_server()` for blocking GDB sessions where GDB controls execution
- Use repeated `emulator_step()` calls when you want C code to control execution pace

## Error Handling

The API uses error codes for error handling:

```c
typedef enum {
    Success = 0,
    InvalidArgs = -1,
    InitializationFailed = -2,
    NullPointer = -3,
    InvalidEmulator = -4,
    StepComplete = -5,
    GdbError = -6,
} EmulatorError;
```

## Configuration Parameters

The `EmulatorArgs` structure provides memory layout configuration:

- **soc_address_hi/lo**: High and low 32 bits of the SoC memory base address
- **soc_size**: Size of the SoC memory region
- **uc_address_hi/lo**: High and low 32 bits of the microcontroller memory base address
- **uc_size**: Size of the microcontroller memory region
- **soc_manifest**: Path to the SoC manifest file
- **gdb_port**: GDB server port (0 = disabled)
```

## Integration Notes

### Linking

When linking with the static library, you may need additional system libraries:

```bash
-lemulator_cbinding -lpthread -ldl -lm
```

### Memory Alignment

Always use 8-byte alignment when allocating memory for the emulator. Improper alignment can cause crashes or undefined behavior.

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
- GDB integration
- Proper cleanup

Basic usage:
```bash
./example
```

GDB usage:
```bash
./example --gdb 3333
```
