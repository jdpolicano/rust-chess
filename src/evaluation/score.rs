use super::psqt::{
    BISHOP, BISHOP_TABLE, KING, KING_TABLE_MID, KNIGHT, KNIGHT_TABLE, PAWN, PAWN_TABLE, QUEEN,
    QUEEN_TABLE, ROOK, ROOK_TABLE,
};
use chess::{Board, ChessMove, Color, File, Piece, Rank, Square};

pub struct PieceEvent {
    pub piece: Piece,
    pub sq: Square,
}

impl PieceEvent {
    pub fn new(piece: Piece, sq: Square) -> Self {
        return Self { piece, sq };
    }
}

pub struct MoveEvents {
    // if you promote
    pub promotion: Option<PieceEvent>,
    // if you capture a piece, your side is better, right?
    pub capture: Option<PieceEvent>,
}

impl MoveEvents {
    pub fn new() -> Self {
        return Self {
            promotion: None,
            capture: None,
        };
    }

    pub fn add_promotion(&mut self, piece: Option<Piece>, sq: Square) {
        if let Some(p) = piece {
            self.promotion = Some(PieceEvent::new(p, sq));
        }
    }

    pub fn add_capture(&mut self, piece: Option<Piece>, sq: Square) {
        if let Some(p) = piece {
            self.capture = Some(PieceEvent::new(p, sq));
        }
    }
}

pub struct MoveInfo {
    pub color_to_move: Color,
    pub move_events: MoveEvents,
    pub from: Square,
    pub to: Square,
    pub piece: Piece,
}

impl MoveInfo {
    pub fn new(
        color_to_move: Color,
        move_events: MoveEvents,
        from: Square,
        to: Square,
        piece: Piece,
    ) -> Self {
        return Self {
            color_to_move,
            move_events,
            from,
            to,
            piece,
        };
    }

    pub fn from_move(m: &ChessMove, b: &Board) -> Self {
        let from = m.get_source();
        let to = m.get_dest();
        let piece = b.piece_on(from).unwrap();
        let color_to_move = b.color_on(from).unwrap();
        let mut move_events = MoveEvents::new();
        move_events.add_promotion(m.get_promotion(), to);
        move_events.add_capture(b.piece_on(to), to);
        return Self::new(color_to_move, move_events, from, to, piece);
    }
}

pub fn piece_value(piece: Piece) -> i16 {
    match piece {
        Piece::Pawn => PAWN,
        Piece::Knight => KNIGHT,
        Piece::Bishop => BISHOP,
        Piece::Rook => ROOK,
        Piece::Queen => QUEEN,
        Piece::King => KING,
    }
}

pub fn score_piece_position(piece: Piece, color: Color, rank: Rank, file: File) -> i16 {
    match piece {
        Piece::Pawn => PAWN_TABLE.eval_with_piece(piece, color, rank, file),
        Piece::Knight => KNIGHT_TABLE.eval_with_piece(piece, color, rank, file),
        Piece::Bishop => BISHOP_TABLE.eval_with_piece(piece, color, rank, file),
        Piece::Rook => ROOK_TABLE.eval_with_piece(piece, color, rank, file),
        Piece::Queen => QUEEN_TABLE.eval_with_piece(piece, color, rank, file),
        Piece::King => KING_TABLE_MID.eval_with_piece(piece, color, rank, file),
    }
}

pub fn score_board_position(board: &Board) -> (i16, i16) {
    let mut white = 0;
    let mut black = 0;
    for r in 0..8 {
        for f in 0..8 {
            let rank = Rank::from_index(r);
            let file = File::from_index(f);
            let square = Square::make_square(rank, file);
            board.piece_on(square).map(|piece| {
                board.color_on(square).map(|c| {
                    let score = score_piece_position(piece, c, rank, file);
                    if c == Color::White {
                        white += score;
                    } else {
                        black += score;
                    }
                });
            });
        }
    }
    return (white, black);
}

/// returns the change in positional score after a capture relative to the opponent
pub fn score_capture_diff(info: &MoveInfo) -> i16 {
    let capture_score = info.move_events.capture.as_ref().map(|c| {
        score_piece_position(
            c.piece,
            !info.color_to_move,
            c.sq.get_rank(),
            c.sq.get_file(),
        )
    });
    return -capture_score.unwrap_or(0);
}

/// Returns the position change from the perspective of the color to move
pub fn score_position_diff(info: &MoveInfo) -> i16 {
    // the score of the original position of the piece.
    let start_score = score_piece_position(
        info.piece,
        info.color_to_move,
        info.from.get_rank(),
        info.from.get_file(),
    );

    // if it is a promotion, we need to calculate the score of the new piece
    // at the new square
    if let Some(ref promo) = info.move_events.promotion {
        let promotion_score = score_piece_position(
            promo.piece,
            info.color_to_move,
            promo.sq.get_rank(),
            promo.sq.get_file(),
        );
        return promotion_score - start_score;
    }

    let end_score = score_piece_position(
        info.piece,
        info.color_to_move,
        info.to.get_rank(),
        info.to.get_file(),
    );

    return end_score - start_score;
}
