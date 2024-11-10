use std::{env, fs::File, io::Read, process::exit};
use itertools::Itertools;
use lexer::BlockLexer;

mod lexer;
mod token;

fn main() {
    let args :Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage {} <md file path>", args[0]);
        exit(1);
    }

    let mut f = File::open(&args[1])
        .expect("file not found");
    let mut content = String::new();
    f.read_to_string(&mut content)
        .expect("cannot read file");

    let linebreaked_content :Vec<String> = content
        .lines()
        .map(|s| s.to_string())
        .collect(); 
    let mut block_lexer = BlockLexer::new(linebreaked_content);
    let tokens = block_lexer.tokenize();

    let html = tokens.iter()
        .map(|elm| elm.to_html())
        .join("\n");

    println!("{}", html);
}
