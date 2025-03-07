use crate::engine::Engine;
use chess::{
    get_file, get_rank, Board, BoardStatus, ChessMove, Color, File, Game, MoveGen, Piece, Rank,
};
use std::fmt::{Display, Formatter, Result};
pub struct ChessGame {
    game: Game,
    engine: Engine,
    debug: bool,
}

impl ChessGame {
    pub fn new(debug: bool) -> Self {
        let game = Game::new();
        let engine = Engine::new(3);
        return Self {
            game,
            engine,
            debug,
        };
    }

    pub fn new_with_game(game: Game, debug: bool) -> Self {
        let engine = Engine::new(3);
        return Self {
            game,
            engine,
            debug,
        };
    }

    pub fn set_depth(&mut self, d: u8) {
        self.engine.set_depth(d);
    }

    pub fn set_debug(&mut self, b: bool) {
        self.debug = b;
    }

    pub fn next_move(&mut self) {
        let next = self.engine.next_move(&self.game.current_position());
        self.game.make_move(next);
    }

    pub fn print_board(&self) {
        println!("{}", self.game.current_position().to_string());
    }

    pub fn print_move(&self, m: ChessMove) {
        let source = m.get_source();
        let dest = m.get_dest();
        println!(
            "----{:?}{:?} {:?}{:?}----",
            source.get_rank(),
            source.get_file(),
            dest.get_rank(),
            dest.get_file(),
        );
        println!("{}", m);
    }
}
