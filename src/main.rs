use rust_engine::engine::Engine;
//use rust_engine::uci::{UCIEngine, UCITestEngine};

fn main() {
    let options = CommandLineOptions::new();
    let mut eng = Engine::new();
    eng.run();
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
