//@ compile-flags: --dap

use std::thread;

fn main() {
    let handle = thread::spawn(|| 42);
    let marker = 0;
    let _ = marker;
    let _ = handle.join().unwrap();
}
