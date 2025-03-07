use chess_engine::chess_game::ChessGame;

fn main() {
    let mut game = ChessGame::new();
    game.set_depth(5);
    while !game.is_over() {
        // white
        let next = game.ask_engine();
        game.make_move(next);

        //black
        let next = game.ask_engine();
        game.make_move(next);
    }

    game.print_board_fen();
    game.print_pgn();
}

// fn chess_move_to_pgn(m: ChessMove) -> String {
//     //
// }
