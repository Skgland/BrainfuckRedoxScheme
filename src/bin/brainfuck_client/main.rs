use std::fs::File;
use std::io::{Read, Write};
use brainfuck_redox_scheme::examples::CAT;


fn main() {

    let mut vec_file = File::open(format!("/scheme/brainfuck/{CAT}"))
        .expect("Failed to open vec file");

    vec_file.write_all(b" Hello")
        .expect("Failed to write to vec");

    let mut read_into = String::new();
    vec_file.read_to_string(&mut read_into)
        .expect("Failed to read from vec");

    println!("{}", read_into); // olleH ih/
}