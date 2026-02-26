fn main() {
    let mut bits = [0, 0, 0, 0];
    println!("bits = {:?}", &bits);
    let start = 4;
    let end = 11;
    let num = read(&bits, start, end);
    println!("{num}");

    write(&mut bits, 4, 12, 85);
    println!("{:?}", &bits);
}

// 0 01 010 01000000000000000000000001
// 1 001 0001 000000000000000000000001

fn read(bits: &[u8], start: u8, end: u8) -> u64 {
    (start..end)
        .map(|i| {
            let byte_idx = i / 8;
            let bit_pos = i % 8;
            bits[byte_idx as usize] >> (7 - bit_pos) & 1
        })
        .fold(0u64, |acc, e| acc << 1 | e as u64)
}

fn write(bits: &mut [u8], start: u8, end: u8, num: u64) {
    (start..end).for_each(|i| {
        let byte_idx = i / 8;
        let bit_pos = 7 - (i % 8);
        let s = end - 1 - i;
        let v = (num >> s & 1) as u8;
        bits[byte_idx as usize] &= !(1 << bit_pos); // reset
        bits[byte_idx as usize] |= v << bit_pos; // set
    })
}
