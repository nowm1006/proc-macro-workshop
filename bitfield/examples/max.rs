const _: () = {
    let m = &2u64;
    if *m < 1u64 {
        panic!("panic");
    }
};
fn main() {
    let a = [0u64, 1, 2, 1];
    let m = a.iter().max().unwrap_or(&0u64);

    if *m > 1u64 {
        println!("max {}", m);
    }
}
