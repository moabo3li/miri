//@ compile-flags: --dap

fn inner() {
    let inner_marker = 1;
    let _ = inner_marker;
}

fn outer() {
    let outer_marker = 2;
    inner();
    let _ = outer_marker;
}

fn main() {
    outer();
}
