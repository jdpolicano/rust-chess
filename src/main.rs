use rust_engine::engine::get_engine;
use rust_engine::uci::{UCIEngine, UCITestEngine};

fn main() {
    let is_test_engine = "RUST_CHESS_TEST_MODE";
    let env = std::env::var(is_test_engine);
    if env.is_ok() {
        println!("running in chess engine mode...");
        let options = CommandLineOptions::new();
        let test_engine = UCITestEngine::default();
        println!("running eng1 as {}", options.eng1);
        println!("running eng2 as {}", options.eng2);
        if let Err(e) = test_engine.run(options.eng1, options.eng2) {
            eprintln!("Error: {}", e);
        }
        return;
    }
    let mut engine = UCIEngine::new(get_engine);

    if let Err(e) = engine.run() {
        eprintln!("Error: {}", e);
    }
}

struct CommandLineOptions {
    pub eng1: String,
    pub eng2: String,
}

impl CommandLineOptions {
    pub fn new() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let eng1 = args
            .get(1)
            .unwrap_or(&"./target/release/rust-engine".to_string())
            .to_string();
        let eng2 = args
            .get(2)
            .unwrap_or(&"./target/release/rust-engine".to_string())
            .to_string();
        return Self { eng1, eng2 };
    }
}
