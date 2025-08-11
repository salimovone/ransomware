use std::env;
fn main() {
    // kutilyotgan argumentlar: -first; -clone; // yoki argumentsiz
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        println!("Argumentsiz ishga tushirildi");
    }else {
        if args.contains(&String::from("-first")) {
            println!("-first argumenti bilan ishga tushirildi");
        }
        if args.contains(&String::from("-clone")) {
            println!("-clone argumenti bilan ishga tushirildi");
        }
    }
}