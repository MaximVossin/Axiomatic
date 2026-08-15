use std::io::{self, Write};

fn main(){
    let mut input = String::new();
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        input.clear();
        io::stdin()
            .read_line(&mut input)
            .expect("Error of input!");
        input = input.trim_end().to_string();

        if input.to_lowercase() == "exit" {
            println!("Goodbye!");
            break;
        }

        println!("You pressed: {}", input);
        println!();
    }
}
