use crate::engine::ThreadHandler;
use crate::evaluation::{nega_max, NegaMaxResult, SearchContext, MIN_SCORE};
use crate::uci::UciCommand;
use chess::{Board, ChessMove, MoveGen};
use crossbeam::channel::{Receiver, RecvError, SendError, Sender};
use rayon::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{atomic::AtomicBool, Arc};
use std::thread::{spawn, JoinHandle};
use std::time::{Duration, Instant};

pub const DEFAULT_DEPTH: u8 = 5;

pub enum SearchControl {
    Depth(u8),
    Time(Instant),
    Infinite,
}

impl SearchControl {
    pub fn depth(&self) -> Option<u8> {
        match self {
            SearchControl::Depth(d) => Some(*d),
            _ => None,
        }
    }
}

impl TryFrom<UciCommand> for SearchControl {
    type Error = ();

    fn try_from(cmd: UciCommand) -> Result<Self, Self::Error> {
        match cmd {
            UciCommand::Go {
                depth,
                movetime,
                infinite,
            } => {
                if infinite {
                    return Ok(SearchControl::Infinite);
                }

                if let Some(d) = depth {
                    return Ok(SearchControl::Depth(d));
                }
                if let Some(t) = movetime {
                    return Ok(SearchControl::Time(
                        Instant::now() + Duration::from_millis(t),
                    ));
                }

                Err(())
            }
            _ => Err(()),
        }
    }
}

pub enum SearchCommand {
    Search(SearchRequest),
    Quit,
}

pub enum Fen {
    StartPos,
    UciNew(String),
}

pub struct SearchRequest {
    pub ctrl: SearchControl,
    // the position this search is relative to.
    // StartPos maintains the current internal state.
    // UciNew will start from a new position entirely and clear the history.
    pub board: Board,
    // moves to apply on top of the current position.
    pub position_history: Vec<u64>,
    // a control signal to cancel the request at any point.
    pub signal: Arc<AtomicBool>,
}

impl SearchRequest {
    pub fn new(
        ctrl: SearchControl,
        board: Board,
        position_history: Vec<u64>,
        signal: Arc<AtomicBool>,
    ) -> Self {
        Self {
            ctrl,
            board,
            position_history,
            signal,
        }
    }
}

#[derive(Debug)]
pub struct MoveScore {
    // the move considered
    m: ChessMove,
    // the value of that move relative to the current board.
    info: NegaMaxResult,
}

impl MoveScore {
    pub fn new(m: ChessMove, info: NegaMaxResult) -> Self {
        Self { m, info }
    }
}

#[derive(Debug)]
pub struct SearchResponse {
    // the number of nodes searched
    pub nodes: u64,
    // the max depth achieved
    pub depth: u8,
    // the best move found
    pub best_move: Option<ChessMove>,
    // the entire list of moves considered, with their results.
    pub all_moves: Vec<MoveScore>,
}

#[derive(Debug)]
pub enum SearchError {
    InvalidStartPosition,
}

pub type SearchResult = Result<SearchResponse, SearchError>;

pub struct SearchHandler {
    rx: Receiver<SearchResult>,
    tx: Sender<SearchCommand>,
    handle: JoinHandle<()>,
}

impl ThreadHandler<SearchCommand, SearchResult> for SearchHandler {
    fn sender(&self) -> Sender<SearchCommand> {
        self.tx.clone()
    }

    fn receiver(&self) -> Receiver<SearchResult> {
        self.rx.clone()
    }

    fn quit(self) {
        if let Err(e) = self.tx.send(SearchCommand::Quit) {
            println!("search quit error: {:?}", e);
            return;
        }
        let _ = self.handle.join();
    }
}

pub struct Search;

impl Search {
    pub fn init() -> SearchHandler {
        let (search_req_tx, search_req_rx) = crossbeam::channel::unbounded::<SearchCommand>();
        let (search_res_tx, search_res_rx) = crossbeam::channel::unbounded::<SearchResult>();
        let join_handle = spawn(move || {
            let mut quit = false;
            let search_req_rx = search_req_rx.clone();
            let search_res_tx = search_res_tx.clone();
            while !quit {
                let request = search_req_rx.recv();

                if request.is_err() {
                    quit = true;
                    println!("search read channel closed.");
                    continue;
                }

                let request = request.unwrap();

                match request {
                    SearchCommand::Quit => {
                        quit = true;
                        println!("search quitting...");
                        continue;
                    }
                    SearchCommand::Search(request) => {
                        if let Err(response) = Search::search(request, &search_res_tx) {
                            println!("search write channel closed.");
                            println!("final message out: {:?}", response);
                            quit = true;
                        };
                    }
                }
            }
            println!("search is done.");
        });

        SearchHandler {
            tx: search_req_tx,
            rx: search_res_rx,
            handle: join_handle,
        }
    }

    pub fn search(
        request: SearchRequest,
        tx: &Sender<Result<SearchResponse, SearchError>>,
    ) -> Result<(), SendError<SearchResult>> {
        let response = match request.ctrl {
            SearchControl::Depth(d) => Search::search_depth(&request, d),
            SearchControl::Time(_) => Search::search_depth(&request, DEFAULT_DEPTH),
            SearchControl::Infinite => Search::search_depth(&request, u8::MAX),
        };

        tx.send(response)
    }

    pub fn search_depth(request: &SearchRequest, depth: u8) -> Result<SearchResponse, SearchError> {
        let mg = MoveGen::new_legal(&request.board);
        let all_moves: Vec<MoveScore> = mg
            .collect::<Vec<ChessMove>>()
            .par_iter()
            .map(|m| {
                let history = Rc::new(RefCell::new(request.position_history.clone()));
                let ctx = SearchContext::from_board(
                    request.board.make_move_new(*m),
                    history,
                    None,
                    request.signal.clone(),
                );
                let info = -nega_max(ctx, depth, MIN_SCORE, -MIN_SCORE);
                MoveScore::new(*m, info)
            })
            .collect();
        let nodes = all_moves.iter().map(|m| m.info.nodes).sum();
        let best_move = all_moves.iter().max_by_key(|m| m.info.score).map(|m| m.m);

        Ok(SearchResponse {
            nodes,
            depth,
            best_move,
            all_moves,
        })
    }
}
