mod error;
mod lexer;
mod parser;

use crate::lexer::Lexer;
use crate::lexer::TokenKind;
use std::io::{self, Write};

fn main() {
    env_logger::init();

    if std::env::args().any(|arg| arg == "--version" || arg == "-v") {
        println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    println!("Axiomatic v{}", env!("CARGO_PKG_VERSION"));
    println!("Type :help for command\n");

    let mut input = String::new();
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        input.clear();
        io::stdin().read_line(&mut input).expect("Error of input!");
        input = input.trim().to_string();

        if input.to_lowercase() == ":exit" {
            println!("Goodbye!");
            break;
        }

        if input.starts_with(":lexer") {
            let code = input.trim_start_matches(":lexer").to_string();
            let lex = Lexer::new(&code);
            let (tokens, errors) = lex.tokenize();

            println!();

            for i in &errors {
                print!("{}", i.pretty_print(&code));
            }

            for tk in &tokens {
                if !matches!(tk.kind, TokenKind::EOF) {
                    println!("{tk}");
                }
            }
        }

        println!("You pressed: {input}");
        println!();
    }
}
