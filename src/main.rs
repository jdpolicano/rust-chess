use chess::{ChessMove, Game};
use rust_engine::engine::get_engine;
use rust_engine::uci::{UCIEngine, UCIEngineOptions, UCITestEngine};

fn main() {
    let is_test_engine = "RUST_CHESS_TEST_MODE";
    let env = std::env::var(is_test_engine);
    if env.is_ok() {
        println!("running in chess engine mode...");
        let test_engine = UCITestEngine::default();
        if let Err(e) = test_engine.run(
            "./target/release/rust-engine".to_string(),
            "./target/release/rust-engine".to_string(),
        ) {
            eprintln!("Error: {}", e);
        }
        return;
    }
    let mut engine = UCIEngine::new(get_engine);

    if let Err(e) = engine.run() {
        eprintln!("Error: {}", e);
    }
}
