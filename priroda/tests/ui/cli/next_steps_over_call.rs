fn callee() {
    let inner = 1;
    let _ = inner;
}

fn main() {
    let setup = 0;
    let _ = setup;
    callee();
    let after = 2;
    let _ = after;
}
