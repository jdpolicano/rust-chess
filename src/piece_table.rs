use chess::{Color, File, Piece, Rank, NUM_RANKS};
// basic piece values.
pub const PAWN: i32 = 100;
pub const KNIGHT: i32 = 320;
pub const BISHOP: i32 = 330;
pub const ROOK: i32 = 500;
pub const QUEEN: i32 = 900;
pub const KING: i32 = 20000;

pub struct PieceTable([i32; 64]);

fn flip_rank(r: Rank) -> usize {
    return NUM_RANKS - r.to_index() - 1;
}

impl PieceTable {
    pub fn eval_position(&self, color: Color, rank: Rank, file: File) -> i32 {
        let (r_idx, f_idx) = if color == Color::White {
            (flip_rank(rank), file.to_index())
        } else {
            (rank.to_index(), file.to_index())
        };
        let rc = (r_idx * NUM_RANKS) + f_idx;
        return self.at_index(rc);
    }

    pub fn eval_with_piece(&self, piece: Piece, color: Color, rank: Rank, file: File) -> i32 {
        let piece_value = piece_value(piece);
        let position_value = self.eval_position(color, rank, file);
        return piece_value + position_value;
    }

    pub fn at_index(&self, idx: usize) -> i32 {
        return self.0[idx];
    }
}

pub fn score_piece_position(piece: Piece, color: Color, rank: Rank, file: File) -> i32 {
    match piece {
        Piece::Pawn => PAWN_TABLE.eval_with_piece(piece, color, rank, file),
        Piece::Knight => KNIGHT_TABLE.eval_with_piece(piece, color, rank, file),
        Piece::Bishop => BISHOP_TABLE.eval_with_piece(piece, color, rank, file),
        Piece::Rook => ROOK_TABLE.eval_with_piece(piece, color, rank, file),
        Piece::Queen => QUEEN_TABLE.eval_with_piece(piece, color, rank, file),
        Piece::King => KING_TABLE_MID.eval_with_piece(piece, color, rank, file),
    }
}

pub fn piece_value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => PAWN,
        Piece::Knight => KNIGHT,
        Piece::Bishop => BISHOP,
        Piece::Rook => ROOK,
        Piece::Queen => QUEEN,
        Piece::King => KING,
    }
}

pub const PAWN_TABLE: PieceTable = PieceTable([
    0, 0, 0, 0, 0, 0, 0, 0, 50, 50, 50, 50, 50, 50, 50, 50, 10, 10, 20, 30, 30, 20, 10, 10, 5, 5,
    10, 25, 25, 10, 5, 5, 0, 0, 0, 20, 20, 0, 0, 0, 5, -5, -10, 0, 0, -10, -5, 5, 5, 10, 10, -20,
    -20, 10, 10, 5, 0, 0, 0, 0, 0, 0, 0, 0,
]);

pub const KNIGHT_TABLE: PieceTable = PieceTable([
    -50, -40, -30, -30, -30, -30, -40, -50, -40, -20, 0, 0, 0, 0, -20, -40, -30, 0, 10, 15, 15, 10,
    0, -30, -30, 5, 15, 20, 20, 15, 5, -30, -30, 0, 15, 20, 20, 15, 0, -30, -30, 5, 10, 15, 15, 10,
    5, -30, -40, -20, 0, 5, 5, 0, -20, -40, -50, -40, -30, -30, -30, -30, -40, -50,
]);

pub const BISHOP_TABLE: PieceTable = PieceTable([
    -20, -10, -10, -10, -10, -10, -10, -20, -10, 0, 0, 0, 0, 0, 0, -10, -10, 0, 5, 10, 10, 5, 0,
    -10, -10, 5, 5, 10, 10, 5, 5, -10, -10, 0, 10, 10, 10, 10, 0, -10, -10, 10, 10, 10, 10, 10, 10,
    -10, -10, 5, 0, 0, 0, 0, 5, -10, -20, -10, -10, -10, -10, -10, -10, -20,
]);

pub const ROOK_TABLE: PieceTable = PieceTable([
    0, 0, 0, 0, 0, 0, 0, 0, 5, 10, 10, 10, 10, 10, 10, 5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0,
    0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, 0, 0,
    0, 5, 5, 0, 0, 0,
]);

pub const QUEEN_TABLE: PieceTable = PieceTable([
    -20, -10, -10, -5, -5, -10, -10, -20, -10, 0, 0, 0, 0, 0, 0, -10, -10, 0, 5, 5, 5, 5, 0, -10,
    -5, 0, 5, 5, 5, 5, 0, -5, 0, 0, 5, 5, 5, 5, 0, -5, -10, 5, 5, 5, 5, 5, 0, -10, -10, 0, 5, 0, 0,
    0, 0, -10, -20, -10, -10, -5, -5, -10, -10, -20,
]);

pub const KING_TABLE_MID: PieceTable = PieceTable([
    -30, -40, -40, -50, -50, -40, -40, -30, -30, -40, -40, -50, -50, -40, -40, -30, -30, -40, -40,
    -50, -50, -40, -40, -30, -30, -40, -40, -50, -50, -40, -40, -30, -20, -30, -30, -40, -40, -30,
    -30, -20, -10, -20, -20, -20, -20, -20, -20, -10, 20, 20, 0, 0, 0, 0, 20, 20, 20, 30, 10, 0, 0,
    10, 30, 20,
]);

pub const KING_TABLE_END: PieceTable = PieceTable([
    -50, -40, -30, -20, -20, -30, -40, -50, -30, -20, -10, 0, 0, -10, -20, -30, -30, -10, 20, 30,
    30, 20, -10, -30, -30, -10, 30, 40, 40, 30, -10, -30, -30, -10, 30, 40, 40, 30, -10, -30, -30,
    -10, 20, 30, 30, 20, -10, -30, -30, -30, 0, 0, 0, 0, -30, -30, -50, -30, -30, -30, -30, -30,
    -30, -50,
]);

mod test {
    #[test]
    fn test_piece_table() {
        use super::*;
        let pt = PAWN_TABLE;
        // in normal direction.
        for i in 0..8 {
            for j in 0..8 {
                let rank_white = Rank::from_index(i);
                let file_white = File::from_index(j);
                let rank_black = Rank::from_index(7 - i);
                let file_black = File::from_index(j);
                assert_eq!(
                    pt.eval_position(Color::White, rank_white, file_white),
                    pt.eval_position(Color::Black, rank_black, file_black),
                );
            }
        }
    }

    #[test]
    fn white_rook_seventh_rank() {
        use super::*;
        let pt = ROOK_TABLE;
        let rank = Rank::Seventh;
        let file = File::B;
        assert_eq!(pt.eval_position(Color::White, rank, file), 10);
    }
}
