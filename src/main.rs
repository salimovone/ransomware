mod app;
mod clone;
mod file_manager;
mod first;
mod utils;

use std::env;
fn main() {
    // kutilyotgan argumentlar: -first; -clone; // yoki argumentsiz
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    if args.len() < 2 {
        println!("Argumentsiz ishga tushirildi");
        app::main();
    } else {
        if args.contains(&String::from("-first")) {
            println!("-first argumenti bilan ishga tushirildi");
            first::main();
        }
        if args.contains(&String::from("-clone")) {
            println!("-clone argumenti bilan ishga tushirildi");
            clone::main();
        }
    }
}
