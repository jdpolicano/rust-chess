use crate::piece_table::score_piece;
use chess::{Board, BoardStatus, ChessMove, Color, File, GameResult, MoveGen, Piece, Rank, Square};
use rayon::prelude::*;

type Score = i32;

#[derive(Debug, Clone)]
struct MoveState {
    pub board: Board,
    pub black_pt_score: i32, // the strength of the white piece table eval
    pub white_pt_score: i32, // the strength of the black piece tabel eval
}

impl MoveState {
    pub fn new(board: Board, white_pt_score: i32, black_pt_score: i32) -> Self {
        return Self {
            board,
            white_pt_score,
            black_pt_score,
        };
    }

    pub fn eval(&self) -> Score {
        return self.white_pt_score - self.black_pt_score;
    }

    pub fn mod_eval(&self, modifier: i32) -> Score {
        return modifier * self.eval();
    }
}

pub struct Engine {
    depth: u8, // the depth to search
}

impl Engine {
    pub fn new(depth: u8) -> Self {
        return Self { depth };
    }

    pub fn set_depth(&mut self, d: u8) {
        self.depth = d;
    }

    pub fn next_move(&self, board: &Board) -> ChessMove {
        // for each move we should do something
        //let legal_moves: Vec<ChessMove> = MoveGen::new_legal(&self.board).into_iter().collect();
        let best = MoveGen::new_legal(board)
            .par_bridge()
            .map(|m| {
                let curr_state = self.get_curr_state(board);
                let next_state = score_move(curr_state, m);
                let modifier = if next_state.board.side_to_move() == Color::Black {
                    -1
                } else {
                    1
                };
                let score = nega_max(next_state, self.depth, modifier);
                return (score, m);
            })
            .max_by(|(score1, _), (score2, _)| score1.cmp(score2));
        return best.unwrap().1;
    }

    fn get_curr_state(&self, board: &Board) -> MoveState {
        let (white_pt_score, black_pt_score) = score_board(board);
        return MoveState::new(board.clone(), white_pt_score, black_pt_score);
    }
}

fn score_board(board: &Board) -> (i32, i32) {
    let mut white = 0;
    let mut black = 0;
    for r in 0..8 {
        for f in 0..8 {
            let rank = Rank::from_index(r);
            let file = File::from_index(f);
            let square = Square::make_square(rank, file);
            board.piece_on(square).map(|piece| {
                board.color_on(square).map(|c| {
                    let score = score_piece(piece, c, rank, file);
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

fn score_move(mut init_state: MoveState, m: ChessMove) -> MoveState {
    let color = init_state.board.side_to_move();
    let from_piece = init_state.board.piece_on(m.get_source()).unwrap();
    let to_piece = get_to_piece(m, from_piece);
    let my_score_diff = calculate_my_score_diff(m, from_piece, to_piece, color);
    let their_score_diff = calculate_their_score_diff(&init_state.board, m);

    init_state.board = init_state.board.make_move_new(m);
    update_scores(&mut init_state, color, my_score_diff, their_score_diff);
    return init_state;
}

fn get_to_piece(m: ChessMove, from_piece: Piece) -> Piece {
    if let Some(p) = m.get_promotion() {
        p
    } else {
        from_piece
    }
}

fn calculate_my_score_diff(m: ChessMove, from_piece: Piece, to_piece: Piece, color: Color) -> i32 {
    let from_rank = m.get_source().get_rank();
    let from_file = m.get_source().get_file();
    let to_rank = m.get_dest().get_rank();
    let to_file = m.get_dest().get_file();
    let from_score = score_piece(from_piece, color, from_rank, from_file);
    let to_score = score_piece(to_piece, color, to_rank, to_file);
    to_score - from_score
}

fn calculate_their_score_diff(board: &Board, m: ChessMove) -> i32 {
    if is_capture(board, m) {
        let captured_piece = board.piece_on(m.get_dest()).unwrap();
        let captured_color = board.color_on(m.get_dest()).unwrap();
        let to_rank = m.get_dest().get_rank();
        let to_file = m.get_dest().get_file();
        let captured_score = score_piece(captured_piece, captured_color, to_rank, to_file);
        -captured_score
    } else {
        0
    }
}

fn update_scores(
    init_state: &mut MoveState,
    color: Color,
    my_score_diff: i32,
    their_score_diff: i32,
) {
    if color == Color::Black {
        init_state.black_pt_score += my_score_diff;
        init_state.white_pt_score += their_score_diff;
    } else {
        init_state.white_pt_score += my_score_diff;
        init_state.black_pt_score += their_score_diff;
    }
}

fn nega_max(state: MoveState, depth: u8, modifier: i32) -> Score {
    if depth == 0 {
        return state.mod_eval(modifier);
    }
    let mut max = i32::MIN;
    for m in MoveGen::new_legal(&state.board) {
        let next_state = score_move(state.clone(), m);
        let score = nega_max(next_state, depth - 1, -modifier);
        max = max.max(score);
    }
    return max;
}

fn is_capture(b: &Board, m: ChessMove) -> bool {
    // if the move is literally on top of another piece, it has to be a capture.
    if let Some(_) = b.color_on(m.get_dest()) {
        return true;
    }
    return false;
}
