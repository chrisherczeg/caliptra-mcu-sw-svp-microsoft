/*++

Licensed under the Apache-2.0 license.

File Name:

    minimal.rs

Abstract:

    Minimal test to verify the C binding concept works.

--*/

use std::os::raw::{c_char, c_int, c_uint};

/// Test structure to verify memory management
#[repr(C)]
pub struct TestStruct {
    pub value: c_int,
    pub data: [c_char; 16],
}

/// Get the size required for TestStruct
#[no_mangle]
pub extern "C" fn test_get_size() -> usize {
    std::mem::size_of::<TestStruct>()
}

/// Get the alignment required for TestStruct
#[no_mangle]
pub extern "C" fn test_get_alignment() -> usize {
    std::mem::align_of::<TestStruct>()
}

/// Initialize a TestStruct in provided memory
#[no_mangle]
pub unsafe extern "C" fn test_init(memory: *mut TestStruct, value: c_int) -> c_int {
    if memory.is_null() {
        return -1;
    }
    
    let test_struct = TestStruct {
        value,
        data: [0; 16],
    };
    
    std::ptr::write(memory, test_struct);
    0
}

/// Get value from TestStruct
#[no_mangle]
pub unsafe extern "C" fn test_get_value(memory: *mut TestStruct) -> c_int {
    if memory.is_null() {
        return -1;
    }
    
    let test_struct = &*memory;
    test_struct.value
}

/// Set value in TestStruct
#[no_mangle]
pub unsafe extern "C" fn test_set_value(memory: *mut TestStruct, value: c_int) -> c_int {
    if memory.is_null() {
        return -1;
    }
    
    let test_struct = &mut *memory;
    test_struct.value = value;
    0
}

/// Cleanup TestStruct
#[no_mangle]
pub unsafe extern "C" fn test_destroy(memory: *mut TestStruct) {
    if !memory.is_null() {
        std::ptr::drop_in_place(memory);
    }
}
