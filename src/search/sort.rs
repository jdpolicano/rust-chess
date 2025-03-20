use chess::{Board, ChessMove, MoveGen, NUM_PIECES};

// MVV_VLA[victim][attacker]
pub const MVV_LVA: [[u8; NUM_PIECES + 1]; NUM_PIECES + 1] = [
    [15, 14, 13, 12, 11, 10, 0], // victim P, attacker P, N, B, R, Q, K, None
    [25, 24, 23, 22, 21, 20, 0], // victim N, attacker K, Q, R, B, N, P, None
    [35, 34, 33, 32, 31, 30, 0], // victim B, attacker K, Q, R, B, N, P, None
    [45, 44, 43, 42, 41, 40, 0], // victim R, attacker K, Q, R, B, N, P, None
    [55, 54, 53, 52, 51, 50, 0], // victim Q, attacker K, Q, R, B, N, P, None
    [0, 0, 0, 0, 0, 0, 0],       // victim K, attacker K, Q, R, B, N, P, None
    [0, 0, 0, 0, 0, 0, 0],       // victim None, attacker K, Q, R, B, N, P, None
];

pub fn get_mvv_lva_score(victim: u8, attacker: u8) -> u8 {
    return MVV_LVA[victim as usize][attacker as usize];
}

pub fn get_sorted_moves(board: &Board) -> Vec<ChessMove> {
    let mut moves = MoveGen::new_legal(&board).collect::<Vec<ChessMove>>();
    moves.sort_by(|a, b| {
        let victim_a = board.piece_on(a.get_dest()).map(|p| p as u8).unwrap_or(6);
        let attacker_a = board.piece_on(a.get_source()).map(|p| p as u8).unwrap_or(6);
        let victim_b = board.piece_on(b.get_dest()).map(|p| p as u8).unwrap_or(6);
        let attacker_b = board.piece_on(b.get_source()).map(|p| p as u8).unwrap_or(6);
        get_mvv_lva_score(victim_b, attacker_b).cmp(&get_mvv_lva_score(victim_a, attacker_a))
    });
    return moves;
}
