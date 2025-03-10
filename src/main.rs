use chess::{ChessMove, Game};
use rust_engine::{
    engine::{get_engine, Engine},
    evaluation::NegaMaxOptions,
    pgn::PgnEncoder,
};
use std::collections::HashMap;

fn main() {
    let mut game = Game::new();
    let mut encoder = PgnEncoder::new(game.current_position().clone(), None);
    let eng_opts1 = NegaMaxOptions::new().depth(3);
    let eng_opts2 = NegaMaxOptions::new().depth(6);
    let eng = get_engine(HashMap::new());
    let mut i = 0;
    loop {
        println!("move {i}");
        i += 1;
        let m = eng.next_move(&game.current_position(), eng_opts1.clone());
        game.make_move(m.unwrap());
        encoder.add_move(m.unwrap());
        if game.result().is_some() || game.can_declare_draw() {
            break;
        }
        let m = eng.next_move(&game.current_position(), eng_opts2.clone());
        game.make_move(m.unwrap());
        encoder.add_move(m.unwrap());
        if game.result().is_some() || game.can_declare_draw() {
            break;
        }
    }

    println!("{}", encoder.encode());
}

// fn chess_move_to_pgn(m: ChessMove) -> String {
//     //
// }
