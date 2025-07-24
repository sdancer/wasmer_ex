#![no_std] 
#![cfg_attr(not(test), no_main)]  // No main function when not testing

// Use the core library instead of std
use core::panic::PanicInfo;

// Define panic handler since we have no_std
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// Export a function that will be callable from WebAssembly
#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}
