use crate::{
    engine::Engine,
    pgn::{PgnEncoder, PgnMove},
};
use chess::{ChessMove, Game};

pub struct ChessGame {
    game: Game,
    engine: Engine,
    debug: bool,
    pgn_encoder: PgnEncoder,
}

impl ChessGame {
    pub fn new() -> Self {
        let game = Game::new();
        let engine = Engine::new(3);
        let pgn_encoder = PgnEncoder::new(game.current_position(), None);
        let debug = false;
        return Self {
            game,
            engine,
            debug,
            pgn_encoder,
        };
    }

    pub fn is_over(&mut self) -> bool {
        if self.game.result().is_some() {
            return true;
        }

        if self.game.can_declare_draw() {
            self.game.declare_draw();
            return true;
        }

        return false;
    }

    pub fn set_depth(&mut self, d: u8) {
        self.engine.set_depth(d);
    }

    pub fn set_debug(&mut self, b: bool) {
        self.debug = b;
    }

    pub fn make_move(&mut self, m: ChessMove) {
        self.pgn_encoder.add_move(m);
        self.game.make_move(m);
    }

    pub fn ask_engine(&self) -> ChessMove {
        return self.engine.next_move(&self.game.current_position());
    }

    pub fn print_board_fen(&self) {
        println!("{}", self.game.current_position().to_string());
    }

    pub fn print_pgn(&mut self) {
        let gr = self.game.result();
        self.pgn_encoder.set_outcome(gr.into());
        println!("{}", self.pgn_encoder.encode());
    }

    pub fn print_move(&self, m: ChessMove) {
        let board = self.game.current_position();
        let pgn_move = PgnMove::from_board(m, &board);
        println!("{}", pgn_move)
    }
}
