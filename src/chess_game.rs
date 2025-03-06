use crate::engine::Engine;
use chess::{Board, ChessMove, Color, File, Game, Piece, Rank};
use std::fmt::{Display, Formatter, Result};

pub struct ChessGame {
    game: Game,
    engine: Engine,
    debug: bool,
}

impl ChessGame {
    pub fn new() -> Self {
        let game = Game::new();
        let engine = Engine::new(3);
        return Self { game, engine };
    }

    pub fn new_with_game(game: Game) -> Self {
        let engine = Engine::new(3);
        return Self { game, engine };
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

pub struct Tag {
    name: String,
    value: String,
}

impl Tag {
    pub fn new(name: String, value: String) -> Self {
        return Self { name, value };
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "[{} \"{}\"]\n", self.name, self.value)
    }
}

pub struct PgnMove {
    m: ChessMove,
    piece: Piece,
}

impl PgnMove {
    pub fn new(m: ChessMove, piece: Piece) -> Self {
        return Self { m, piece };
    }
}

pub struct PgnEncoder {
    tags: Vec<Tag>,
    moves: Vec<ChessMove>,
    initial_pos: Board,
}

impl PgnEncoder {
    pub fn new(initial_pos: Board) -> Self {
        return Self {
            tags: Vec::new(),
            moves: Vec::new(),
            initial_pos,
        };
    }

    pub fn add_tag(&mut self, name: String, value: String) {
        self.tags.push(Tag::new(name, value));
    }

    pub fn add_move(&mut self, m: ChessMove) {
        self.moves.push(m);
    }

    pub fn encode(&self) -> String {
        let mut pgn = String::new();
        for tag in &self.tags {
            pgn.push_str(&tag.to_string());
        }

        pgn.push_str("1. ");
        for (i, m) in self.moves.iter().enumerate() {
            if i % 2 == 0 {
                pgn.push_str(&format!("{}. ", i / 2 + 1));
            }
            pgn.push_str(&m.to_string());
            pgn.push(' ');
        }
        pgn.push_str("1-0");
        return pgn;
    }

    fn chess_move_to_san(m: ChessMove) -> String {
        return String::new();
    }

    fn move_is_king_side_castle(&self, color: Color, m: ChessMove) -> bool {
        let source = m.get_source();
        let dest = m.get_dest();
        let source_piece = self.initial_pos.piece_on(source);
        let source_rank = source.get_rank();
        let source_file = source.get_file();
        let dest_rank = dest.get_rank();
        let dest_file = dest.get_file();

        if source_piece == Some(Piece::King) {
            if color == Color::White {
                if source_rank == Rank::First && source_file == File::E {
                    if dest_rank == Rank::First && dest_file == File::G {
                        return true;
                    }
                }
            } else {
                if source_rank == Rank::Eighth && source_file == File::E {
                    if dest_rank == Rank::Eighth && dest_file == File::G {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    fn move_is_queen_side_castle(&self, color: Color, m: ChessMove) -> bool {
        let source = m.get_source();
        let dest = m.get_dest();
        let source_piece = self.initial_pos.piece_on(source);
        let source_rank = source.get_rank();
        let source_file = source.get_file();
        let dest_rank = dest.get_rank();
        let dest_file = dest.get_file();

        if source_piece == Some(Piece::King) {
            if color == Color::White {
                if source_rank == Rank::First && source_file == File::E {
                    if dest_rank == Rank::First && dest_file == File::C {
                        return true;
                    }
                }
            } else {
                if source_rank == Rank::Eighth && source_file == File::E {
                    if dest_rank == Rank::Eighth && dest_file == File::C {
                        return true;
                    }
                }
            }
        }
        return false;
    }
}
