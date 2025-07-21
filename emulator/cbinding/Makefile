# Licensed under the Apache-2.0 license
#
# Makefile for building the emulator C binding example

# Default target
.PHONY: all clean lib example

all: lib example

# Build the Rust static library
lib:
	cargo build --release

# Generate the header file
header:
	cargo build --release

# Build the C example
example: lib header
	gcc -std=c11 -Wall -Wextra -O2 \
		-I. \
		-o example \
		example.c \
		-L./target/release \
		-lemulator_cbinding \
		-lpthread -ldl -lm

# Clean build artifacts
clean:
	cargo clean
	rm -f example emulator_cbinding.h

# Help target
help:
	@echo "Available targets:"
	@echo "  all     - Build library and example (default)"
	@echo "  lib     - Build the Rust static library"
	@echo "  header  - Generate the C header file"
	@echo "  example - Build the C example program"
	@echo "  clean   - Clean build artifacts"
	@echo "  help    - Show this help message"
