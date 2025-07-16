// Licensed under the Apache-2.0 license

//! Emulator Library
//! 
//! This crate provides the core emulator functionality as a reusable library.
//! It includes the `Emulator` struct and related functionality that can be used
//! by multiple programs.

pub mod elf;
pub mod emulator;

// Re-export the main types for convenience
pub use emulator::{Emulator, EmulatorArgs, SystemStepAction};
pub use elf::ElfExecutable;
