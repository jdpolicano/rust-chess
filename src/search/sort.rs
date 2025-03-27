use chess::{Board, ChessMove, MoveGen, NUM_PIECES};

// MVV_VLA[victim][attacker]
pub const MVV_LVA: [[u8; NUM_PIECES + 1]; NUM_PIECES + 1] = [
    [15, 14, 13, 12, 11, 10, 0], // victim P, attacker P, N, B, R, Q, K, None
    [25, 24, 23, 22, 21, 20, 0], // victim N, attacker P, N, B, R, Q, K, None
    [35, 34, 33, 32, 31, 30, 0], // victim B, attacker P, N, B, R, Q, K, None
    [45, 44, 43, 42, 41, 40, 0], // victim R, attacker P, N, B, R, Q, K, None
    [55, 54, 53, 52, 51, 50, 0], // victim Q, attacker P, N, B, R, Q, K, None
    [0, 0, 0, 0, 0, 0, 0],       // victim K, attacker P, N, B, R, Q, K, None
    [0, 0, 0, 0, 0, 0, 0],       // victim None, attacker P, N, B, R, Q, K, None
];

pub fn get_mvv_lva_score(victim: u8, attacker: u8) -> u8 {
    return MVV_LVA[victim as usize][attacker as usize];
}

pub fn sort_moves(board: &Board, moves: &mut Vec<ChessMove>) {
    moves.sort_by(|a, b| {
        let victim_a = board.piece_on(a.get_dest()).map(|p| p as u8).unwrap_or(6);
        let attacker_a = board.piece_on(a.get_source()).map(|p| p as u8).unwrap();
        let victim_b = board.piece_on(b.get_dest()).map(|p| p as u8).unwrap_or(6);
        let attacker_b = board.piece_on(b.get_source()).map(|p| p as u8).unwrap();
        get_mvv_lva_score(victim_b, attacker_b).cmp(&get_mvv_lva_score(victim_a, attacker_a))
    });
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;
    use chess::{get_file, get_rank, Board};

    #[test]
    fn test_sort_moves() {
        let board =
            Board::from_str("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
                .unwrap();
        let mut moves = MoveGen::new_legal(&board).collect::<Vec<ChessMove>>();
        println!("---- Move List Original ----");
        for m in &moves {
            let dest_piece = board.piece_on(m.get_dest());
            let src = m.get_source();
            let dest = m.get_dest();
            println!("Move: {}{} captures {:?}", src, dest, dest_piece);
        }
        sort_moves(&board, &mut moves);
        println!("---- Move List Sorted ----");
        for m in &moves {
            let dest_piece = board.piece_on(m.get_dest());
            let src = m.get_source();
            let dest = m.get_dest();
            println!("Move: {}{} captures {:?}", src, dest, dest_piece);
        }
    }
}
