use rust_engine::{engine::get_engine, uci::UCIEngine};

fn main() {
    let mut uci_engine = UCIEngine::new(get_engine);
    if let Err(e) = uci_engine.run() {
        eprintln!("{}", e);
    }
}

// fn chess_move_to_pgn(m: ChessMove) -> String {
//     //
// }
