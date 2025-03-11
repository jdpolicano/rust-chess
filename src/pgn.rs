use chess::{
    get_file, get_rank, Board, BoardStatus, ChessMove, Color, File, GameResult, MoveGen, Piece,
    Rank,
};

use std::fmt::{Display, Formatter, Result};

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
    pub is_capture: bool,
    pub is_check: bool,
    pub is_ambiguous: bool,
    pub is_checkmate: bool,
}

impl PgnMove {
    pub fn new(
        m: ChessMove,
        piece: Piece,
        is_capture: bool,
        is_check: bool,
        is_ambiguous: bool,
        is_checkmate: bool,
    ) -> Self {
        return Self {
            m,
            piece,
            is_capture,
            is_check,
            is_ambiguous,
            is_checkmate,
        };
    }
    /// This function assumes the board is in the state BEFORE the move is executed.
    pub fn from_board(m: ChessMove, board: &Board) -> Self {
        let piece = board.piece_on(m.get_source()).unwrap();
        let is_check = Self::is_check(m, board);
        let is_capture = Self::is_capture(m, board);
        let is_checkmate = Self::is_checkmate(m, board);
        let is_ambiguous = Self::is_ambiguous(m, board, piece);
        return Self {
            m,
            piece,
            is_capture,
            is_check,
            is_ambiguous,
            is_checkmate,
        };
    }

    pub fn to_src_square_str(&self) -> String {
        return format!("{}{}", self.to_src_file_str(), self.to_src_rank_str());
    }

    pub fn to_dest_square_str(&self) -> String {
        return format!("{}{}", self.to_dest_file_str(), self.to_dest_rank_str());
    }

    pub fn to_piece_abbrev_str(&self) -> Option<String> {
        return Self::piece_to_str(self.piece);
    }

    pub fn to_src_file_str(&self) -> String {
        return Self::to_file_str(self.m.get_source().get_file());
    }

    pub fn to_src_rank_str(&self) -> String {
        return Self::to_rank_str(self.m.get_source().get_rank());
    }

    pub fn to_dest_file_str(&self) -> String {
        return Self::to_file_str(self.m.get_dest().get_file());
    }

    pub fn to_dest_rank_str(&self) -> String {
        return Self::to_rank_str(self.m.get_dest().get_rank());
    }

    pub fn piece_to_str(p: Piece) -> Option<String> {
        if p == Piece::Pawn {
            return None;
        }
        return Some(p.to_string(Color::White));
    }

    pub fn to_file_str(f: File) -> String {
        match f {
            File::A => "a".to_string(),
            File::B => "b".to_string(),
            File::C => "c".to_string(),
            File::D => "d".to_string(),
            File::E => "e".to_string(),
            File::F => "f".to_string(),
            File::G => "g".to_string(),
            File::H => "h".to_string(),
        }
    }

    pub fn to_rank_str(r: Rank) -> String {
        return (r.to_index() + 1).to_string();
    }

    pub fn is_castle(&self) -> bool {
        if self.piece != Piece::King {
            return false;
        }
        let src_file = self.m.get_source().get_file();
        let dest_file = self.m.get_dest().get_file();
        // if the king moved more than one file over it has to be a castle.
        return (src_file.to_index() as i8 - dest_file.to_index() as i8).abs() > 1;
    }

    pub fn is_kingside_castle(&self) -> bool {
        return self.is_castle()
            && self.m.get_source().get_file() == File::E
            && self.m.get_dest().get_file() == File::G;
    }

    pub fn is_queenside_castle(&self) -> bool {
        return self.is_castle()
            && self.m.get_source().get_file() == File::E
            && self.m.get_dest().get_file() == File::C;
    }

    pub fn is_promotion(&self) -> bool {
        return self.m.get_promotion().is_some();
    }

    pub fn promotion_unsafe(&self) -> Piece {
        return self.m.get_promotion().unwrap();
    }

    pub fn is_capture(m: ChessMove, b: &Board) -> bool {
        let color = b.color_on(m.get_source()).unwrap();
        if let Some(c) = b.color_on(m.get_dest()) {
            c == color
        } else {
            false
        }
    }

    pub fn is_check(m: ChessMove, b: &Board) -> bool {
        // essentially, if we take the rank and file of the the piece (the exact square its on)
        // then "&" it with the checking pieces on the board AFTER the move, the only way it could still be greater than 0 is if
        // this move was the one that caused the check.
        let dest_bitmask = get_rank(m.get_dest().get_rank()) & get_file(m.get_dest().get_file());
        return (b.make_move_new(m).checkers().clone() & dest_bitmask).0 > 0;
    }

    fn is_checkmate(m: ChessMove, b: &Board) -> bool {
        return b.make_move_new(m).status() == BoardStatus::Checkmate;
    }

    fn is_ambiguous(m: ChessMove, b: &Board, p: Piece) -> bool {
        let target_sq = get_rank(m.get_dest().get_rank()) & get_file(m.get_dest().get_file());
        let mut moves = MoveGen::new_legal(b);
        moves.set_iterator_mask(target_sq);
        for other_move in moves {
            let other_piece = b.piece_on(other_move.get_source()).unwrap();
            if other_move != m && other_piece == p {
                return true;
            }
        }
        return false;
    }
}

impl Display for PgnMove {
    fn fmt(&self, f: &mut Formatter) -> Result {
        if self.is_castle() {
            if self.is_kingside_castle() {
                write!(f, "{}", "O-O")?;
            }

            if self.is_queenside_castle() {
                write!(f, "{}", "O-O-O")?;
            }

            if self.is_checkmate {
                return write!(f, "{}", "#");
            }

            if self.is_check {
                return write!(f, "{}", "+");
            }

            return Ok(());
        }

        if let Some(pname) = self.to_piece_abbrev_str() {
            write!(f, "{}", pname)?;
        }
        // now handle if you need to write the name of the piece
        // should be "Ne6xf8" for example if its ambiguous capture.
        if self.is_ambiguous {
            write!(f, "{}", self.to_src_square_str())?;
        }

        // now write the x if it was a capture
        if self.is_capture {
            write!(f, "{}", "x")?;
        }

        write!(f, "{}", self.to_dest_square_str())?;

        if self.is_promotion() {
            write!(f, "={}", self.promotion_unsafe())?;
        }

        if self.is_checkmate {
            return write!(f, "{}", "#");
        }

        if self.is_check {
            return write!(f, "{}", "+");
        }

        return Ok(());
    }
}

pub struct PgnOutcome(Option<GameResult>);

impl Display for PgnOutcome {
    fn fmt(&self, f: &mut Formatter) -> Result {
        if let Some(r) = self.0 {
            match r {
                GameResult::WhiteResigns | GameResult::BlackCheckmates => return write!(f, "0-1"),
                GameResult::BlackResigns | GameResult::WhiteCheckmates => return write!(f, "1-0"),
                GameResult::DrawDeclared | GameResult::DrawAccepted | GameResult::Stalemate => {
                    return write!(f, "1/2-1/2")
                }
            }
        }
        return write!(f, "*");
    }
}

impl From<GameResult> for PgnOutcome {
    fn from(g: GameResult) -> Self {
        PgnOutcome(Some(g))
    }
}

impl From<Option<GameResult>> for PgnOutcome {
    fn from(g: Option<GameResult>) -> Self {
        PgnOutcome(g)
    }
}

pub struct PgnEncoder {
    tags: Vec<Tag>,
    moves: Vec<ChessMove>,
    initial_pos: Board,
    outcome: Option<PgnOutcome>,
}

impl PgnEncoder {
    pub fn new(initial_pos: Board, outcome: Option<PgnOutcome>) -> Self {
        return Self {
            tags: Vec::new(),
            moves: Vec::new(),
            initial_pos,
            outcome,
        };
    }

    pub fn add_tag(&mut self, name: String, value: String) {
        self.tags.push(Tag::new(name, value));
    }

    pub fn add_move(&mut self, m: ChessMove) {
        self.moves.push(m);
    }

    pub fn set_outcome(&mut self, o: PgnOutcome) {
        self.outcome = Some(o);
    }

    pub fn encode(&self) -> String {
        let mut board: Board = self.initial_pos.clone();
        let mut pgn = String::new();
        for tag in &self.tags {
            pgn.push_str(&tag.to_string());
        }
        for (i, m) in self.moves.iter().enumerate() {
            if i % 2 == 0 {
                pgn.push_str(&format!("{}.", i / 2 + 1));
            }
            // encode the move relative to the board.
            let pgn_move = PgnMove::from_board(*m, &board);
            pgn.push_str(&format!("{} ", &pgn_move.to_string()));
            // now make the move to change the board.
            board = board.make_move_new(*m);
        }

        if let Some(ref o) = self.outcome {
            pgn.push_str(&o.to_string());
        }

        return pgn;
    }
}

mod test {
    

    
    

    #[test]
    fn test_ambiguity() {
        let board =
            Board::from_str("r4rk1/ppp2ppp/8/2b1Nbq1/2BnQ3/4B3/PPP2PPP/R4RK1 w - - 1 12").unwrap();
        let ambiguous_move = ChessMove::from_san(&board, "Rad1").unwrap();
        let pgn = PgnMove::from_board(ambiguous_move, &board);
        assert_eq!(pgn.is_ambiguous, true);
    }
}
