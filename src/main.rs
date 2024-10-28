use std::{env, process::exit};

mod token;
mod parser;

fn main() {
    let args :Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <markdown content>", args[0]);
        exit(1);
    }

    println!("<p>{}</p>", args[1]);
}
