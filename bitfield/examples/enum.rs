enum E {
    One,
    Two,
    Three,
    Four = 9isize,
}

fn main() {
    println!("One {}", E::One as u64);
    println!("Two {}", E::Two as u64);
    println!("Three {}", E::Three as u64);
    println!("Four {}", E::Four as u64);
}
