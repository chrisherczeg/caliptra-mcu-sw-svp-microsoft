/*++

Licensed under the Apache-2.0 license.

File Name:

    external_shim.rs

Abstract:

    File contains external shim to access external peripherals.

--*/
use caliptra_emu_bus::{Bus, BusError};
use caliptra_emu_types::{RvAddr, RvData, RvSize};

type ReadCallback = Box<dyn Fn(RvSize, RvAddr, &mut u32) -> bool>;
type WriteCallback = Box<dyn Fn(RvSize, RvAddr, RvData) -> bool>;

pub struct Shim {
    read_callback: Option<ReadCallback>,
    write_callback: Option<WriteCallback>,
}

impl Default for Shim {
    fn default() -> Self {
        Self::new()
    }
}

impl Shim {
    pub fn new() -> Self {
        Self {
            read_callback: None,
            write_callback: None,
        }
    }

    /// Register a read callback
    pub fn set_read_callback<F>(&mut self, callback: F)
    where
        F: Fn(RvSize, RvAddr, &mut u32) -> bool + 'static,
    {
        self.read_callback = Some(Box::new(callback));
    }

    /// Register a write callback
    pub fn set_write_callback<F>(&mut self, callback: F)
    where
        F: Fn(RvSize, RvAddr, RvData) -> bool + 'static,
    {
        self.write_callback = Some(Box::new(callback));
    }
}

impl Bus for Shim {
    /// Read data of specified size from given address
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the read
    /// * `addr` - Address to read from
    ///
    /// # Error
    ///
    /// * `RvException` - Exception with cause `RvExceptionCause::LoadAccessFault`
    ///   or `RvExceptionCause::LoadAddrMisaligned`
    fn read(&mut self, size: RvSize, addr: RvAddr) -> Result<RvData, BusError> {
        if let Some(callback) = &self.read_callback {
            let mut buffer: u32 = 0;
            if callback(size, addr, &mut buffer) {
                return Ok(buffer);
            } else {
                return Err(BusError::LoadAccessFault);
            }
        }
        Err(BusError::LoadAccessFault)
    }

    /// Write data of specified size to given address
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the write
    /// * `addr` - Address to write
    /// * `data` - Data to write
    ///
    /// # Error
    ///
    /// * `RvException` - Exception with cause `RvExceptionCause::StoreAccessFault`
    ///   or `RvExceptionCause::StoreAddrMisaligned`
    fn write(&mut self, size: RvSize, addr: RvAddr, value: RvData) -> Result<(), BusError> {
        if let Some(callback) = &self.write_callback {
            if callback(size, addr, value) {
                return Ok(());
            } else {
                return Err(BusError::StoreAccessFault);
            }
        }
        Err(BusError::StoreAccessFault)
    }
}
