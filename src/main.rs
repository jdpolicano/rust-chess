use chess_engine::engine::Engine;

fn main() {
    let mut engine = Engine::new();
    println!("{}", engine.white_pt_score());
    println!("{}", engine.black_pt_score());
    println!("{}", engine.board_string());

    for _ in 0..30 {
        let mut next = engine.next_move();
        println!("----WHITE {:?}----", next);
        engine.do_move(next);
        println!("{}", engine.white_pt_score());
        println!("{}", engine.black_pt_score());
        println!("{}", engine.board_string());

        next = engine.next_move();
        println!("----BLACK {:?}----", next);
        engine.do_move(next);
        println!("{}", engine.white_pt_score());
        println!("{}", engine.black_pt_score());
        println!("{}", engine.board_string());
    }
}
