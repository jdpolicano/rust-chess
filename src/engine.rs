use crate::{
    search::handler::{
        Search, SearchCommand, SearchControl, SearchHandler, SearchRequest, SearchResponse,
    },
    transposition::TT,
    uci::{Uci, UciCommand, UciHandler},
};
use chess::{Board, ChessMove};
use crossbeam::channel::{select, Receiver, Sender};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub trait ThreadHandler<S, R> {
    fn receiver(&self) -> Receiver<R>;
    // should be a thread handler
    fn sender(&self) -> Sender<S>;
    // should be a command to quit
    fn quit(self);
}

pub struct Engine {
    eng_config: HashMap<String, String>,
    search_handler: SearchHandler,
    search_sig: Arc<AtomicBool>,
    uci_handler: UciHandler,
    board: Option<Board>,
    moves: Vec<ChessMove>,
    positions: Vec<u64>,
    tt: Arc<TT>,
    quit: bool,
}

impl Engine {
    pub fn new() -> Self {
        let eng_config = Engine::init_default_config();
        let search_handler = Search::init();
        let uci_handler = Uci::init();
        let board = None;
        let moves = Vec::new();
        let positions = Vec::new();
        let search_sig = Arc::new(AtomicBool::new(false));
        let tt = Arc::new(TT::new(1 << 10)); // 1MB or 1m entries. each entry is a mutex plus two 64-bit integers
        Engine {
            eng_config,
            search_handler,
            search_sig,
            uci_handler,
            board,
            moves,
            positions,
            tt,
            quit: false,
        }
    }

    pub fn run(mut self) {
        while !self.quit {
            select! {
                recv(self.search_handler.receiver()) -> msg => {
                    if let Err(e) = msg {
                        eprintln!("Error: {:?}", e);
                        break;
                    }
                    self.print_search_response(msg.unwrap());
                }
                recv(self.uci_handler.receiver()) -> msg => {
                    if let Err(e) = msg {
                        eprintln!("Error: {:?}", e);
                        break;
                    }
                    self.handle_uci_command(msg.unwrap());
                }
            }
        }
        self.quit();
    }

    fn print_search_response(&self, msg: SearchResponse) {
        if let Some(bm) = msg.best_move {
            let move_info = msg.all_moves.iter().find(|m| m.move_is(bm)).unwrap();
            println!(
                "info depth {} nodes {} score cp {}",
                msg.depth, msg.nodes, move_info.info.score
            );
            println!("bestmove {}", bm);
        }
    }

    fn handle_uci_command(&mut self, cmd: UciCommand) {
        match cmd {
            UciCommand::Quit => self.quit = true,
            UciCommand::IsReady => self.handle_isready(),
            UciCommand::Uci => self.handle_uci(),
            UciCommand::Position { fen, moves } => self.handle_position(fen, moves),
            UciCommand::Go { .. } => self.handle_go(cmd),
            UciCommand::Stop => self.handle_stop(),
            _ => {}
        }
    }

    fn handle_isready(&self) {
        println!("readyok");
    }

    fn handle_uci(&self) {
        println!("id name {}", self.eng_config.get("name").unwrap());
        println!("id author {}", self.eng_config.get("author").unwrap());
        println!("uciok");
    }

    fn handle_position(&mut self, fen: Option<String>, moves: Vec<ChessMove>) {
        let mut board = Board::default();
        if let Some(fen) = fen {
            let b = Board::from_str(&fen);
            if b.is_err() {
                eprintln!("Error: {:?}", b.err().unwrap());
                return;
            }
            board = b.unwrap();
        }
        self.moves = moves;
        self.positions = vec![board.get_hash()];
        for m in &self.moves {
            board = board.make_move_new(*m);
            self.positions.push(board.get_hash());
        }
        self.board = Some(board);
    }

    fn handle_go(&mut self, cmd: UciCommand) {
        if self.board.is_none() {
            eprintln!("Error: No board position set.");
            return;
        }
        let ctrl: Result<SearchControl, ()> = cmd.try_into();
        if ctrl.is_err() {
            eprintln!("Error: {:?}", ctrl.err().unwrap());
            return;
        }
        let cmd = self.build_search_cmd(ctrl.unwrap());
        if let Err(e) = self.search_handler.sender().send(cmd) {
            eprintln!("Error: {:?}", e);
            self.quit = true;
        }
        self.board = None;
        self.moves.clear();
        self.positions.clear();
    }

    fn handle_stop(&mut self) {
        println!("lets try to stop...");
        self.search_sig.store(true, Ordering::Relaxed);
        // wait for the message back before continuing
        let msg = self.search_handler.receiver().recv();
        if let Err(e) = msg {
            eprintln!("Error: {:?}", e);
            self.quit = true;
            return;
        }
        self.print_search_response(msg.unwrap());
        self.search_sig.store(false, Ordering::Relaxed);
    }

    fn build_search_cmd(&self, ctrl: SearchControl) -> SearchCommand {
        let req = SearchRequest::new(
            ctrl,
            self.board.unwrap().clone(),
            self.positions.clone(),
            self.tt.clone(),
            self.search_sig.clone(),
        );
        SearchCommand::Search(req)
    }

    fn quit(self) {
        self.search_handler.quit();
        self.uci_handler.quit();
    }

    fn init_default_config() -> HashMap<String, String> {
        let mut eng_config = HashMap::new();
        eng_config.insert("name".to_string(), "RustChess".to_string());
        eng_config.insert("author".to_string(), "Jacob Policano".to_string());
        eng_config
    }
}
