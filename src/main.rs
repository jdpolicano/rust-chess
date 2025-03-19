mod engine;
mod evaluation;
mod pgn;
mod search;
mod uci;
mod util;

use crate::engine::Engine;
use crate::uci::UCITestEngine;
use std::env;
//use rust_engine::uci::{UCIEngine, UCITestEngine};

fn main() {
    let test_mode = "RUST_ENG_TEST_MODE";
    let is_test_mode = env::var(test_mode).is_ok();
    println!("test mode {}", is_test_mode);
    if is_test_mode {
        let opts = CommandLineOptions::new();
        let test_eng = UCITestEngine::default();
        let _ = test_eng.run(opts.eng1, opts.eng2);
        return;
    }
    let engine = Engine::new();
    engine.run();
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
