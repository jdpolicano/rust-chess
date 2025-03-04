use crate::piece_table::score_piece;
use chess::{Board, ChessMove, Color, File, GameResult, MoveGen, Piece, Rank, Square};
use rayon::prelude::*;

type Score = i32;

#[derive(Debug, Clone)]
struct MoveState {
    pub board: Board,
    pub black_pt_score: i32, // the strength of the white piece table eval
    pub white_pt_score: i32, // the strength of the black piece tabel eval
}

pub struct Engine {
    board: Board, // the current game board
    depth: u8,
    white_pt_score: i32, // the strength of the white piece table eval
    black_pt_score: i32, // the strength of the black piece tabel eval
}

impl Engine {
    pub fn new() -> Self {
        let board = Board::default();
        return Self {
            board: Board::default(),
            depth: 5,
            white_pt_score: init_pt_score(Color::White, &board),
            black_pt_score: init_pt_score(Color::Black, &board),
        };
    }

    pub fn set_depth(&mut self, d: u8) {
        self.depth = d;
    }

    pub fn do_move(&mut self, m: ChessMove) {
        self.board = self.board.make_move_new(m)
    }

    pub fn next_move(&mut self) -> ChessMove {
        // for each move we should do something
        //let legal_moves: Vec<ChessMove> = MoveGen::new_legal(&self.board).into_iter().collect();
        let best = MoveGen::new_legal(&self.board)
            .par_bridge()
            .map(|m| {
                let curr_state = self.get_curr_state();
                let next_state = score_move(curr_state, m);
                let modifier = if next_state.board.side_to_move() == Color::Black {
                    -1
                } else {
                    1
                };
                let score = neg_max(next_state, self.depth - 1, modifier);
                return (score, m);
            })
            .max_by(|(score1, _), (score2, _)| score1.cmp(score2));
        return best.unwrap().1;
    }

    fn get_curr_state(&self) -> MoveState {
        return MoveState {
            board: self.board,
            white_pt_score: self.white_pt_score,
            black_pt_score: self.black_pt_score,
        };
    }

    pub fn white_pt_score(&self) -> i32 {
        return self.white_pt_score;
    }

    pub fn black_pt_score(&self) -> i32 {
        return self.black_pt_score;
    }

    pub fn board_string(&self) -> String {
        return self.board.to_string();
    }
}

fn init_pt_score(color: Color, board: &Board) -> i32 {
    let mut sum = 0;
    for r in 0..8 {
        for f in 0..8 {
            let rank = Rank::from_index(r);
            let file = File::from_index(f);
            let square = Square::make_square(rank, file);
            if let Some(piece) = board.piece_on(square) {
                sum += score_piece(piece, color, rank, file)
            }
        }
    }
    return sum;
}

fn score_move(mut init_state: MoveState, m: ChessMove) -> MoveState {
    // there must a piece there to move, right? Also need to check if its a promotion.
    let color = init_state.board.side_to_move();
    let from_piece = init_state.board.piece_on(m.get_source()).unwrap();
    let to_piece = if let Some(p) = m.get_promotion() {
        p
    } else {
        from_piece
    };
    let fr = m.get_source().get_rank();
    let ff = m.get_source().get_file();
    let tr = m.get_dest().get_rank();
    let tf = m.get_dest().get_file();
    let from_score = score_piece(from_piece, color, fr, ff);
    let to_score = score_piece(to_piece, color, tr, tf);
    // if from_piece == Piece::King {
    //     println!(
    //         "{:?} from {:?} to {:?} is worth {}",
    //         from_piece,
    //         m.get_source(),
    //         m.get_dest(),
    //         to_score - from_score
    //     );
    // }
    // remove the piece from its source (and the score it was giving us) and add in the new locations value.
    let my_score_diff = to_score - from_score;
    let mut their_score_diff = 0;
    if is_capture(init_state.board, m) {
        // there must a piece there to move, right?
        let captured_piece = init_state.board.piece_on(m.get_source()).unwrap();
        let captured_color = init_state.board.color_on(m.get_dest()).unwrap();
        let captured_score = score_piece(captured_piece, captured_color, tr, tf);
        their_score_diff = -captured_score;
    }

    init_state.board = init_state.board.make_move_new(m);
    if color == Color::Black {
        init_state.black_pt_score += my_score_diff;
        init_state.white_pt_score += their_score_diff;
    } else {
        init_state.white_pt_score += my_score_diff;
        init_state.black_pt_score += their_score_diff;
    }
    return init_state;
}

fn neg_max(state: MoveState, depth: u8, modifier: i32) -> Score {
    if depth == 0 {
        return modifier * state.white_pt_score - state.black_pt_score;
    }
    let mut max = i32::MIN;
    for m in MoveGen::new_legal(&state.board) {
        let next_state = score_move(state.clone(), m);
        let score = neg_max(next_state, depth - 1, -modifier);
        max = max.max(score);
    }
    return max;
}

fn is_capture(b: Board, m: ChessMove) -> bool {
    // if the move is literally on top of another piece, it has to be a capture.
    if let Some(_) = b.color_on(m.get_dest()) {
        return true;
    }
    return false;
}
