use std::{env, fs::File, io::Read, process::exit};
use itertools::Itertools;
use lexer::InlineLexer;

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

    let mut lexer = InlineLexer::new(content.chars().collect());
    let tokens = lexer.tokenize();
    dbg!(&tokens);

    let html = tokens.iter()
        .map(|elm| elm.to_html())
        .join("");

    println!("<p>{}</p>", html);
}
