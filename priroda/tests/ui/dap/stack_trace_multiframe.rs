//@ compile-flags: --dap

fn inner() {
    let marker = 1;
    let _ = marker;
}

fn outer() {
    inner();
}

fn main() {
    outer();
}
