// use crate::engine::Engine;
use crate::engine::ThreadHandler;
use crate::pgn::PgnEncoder;

use chess::{ChessMove, Game};
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::fs::create_dir_all;
use std::io::{stdin, BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use std::thread::{spawn, JoinHandle};

pub enum UciParseError {
    Unsupported,
    MissingArgument,
    InvalidUciMove,
}

#[derive(Debug, Clone)]
pub enum UciCommand {
    Uci,
    IsReady,
    SetOption {
        name: String,
        value: Option<String>,
    },
    Register {
        name: Option<String>,
        code: Option<String>,
        later: bool,
    },
    UciNewGame,
    Position {
        fen: Option<String>,
        moves: Vec<ChessMove>,
    },
    Go {
        depth: Option<u8>,
        movetime: Option<u64>,
        infinite: bool,
    },
    Stop,
    Debug(bool),
    Quit,
}

impl UciCommand {
    fn uci() -> Self {
        UciCommand::Uci
    }

    fn is_ready() -> Self {
        UciCommand::IsReady
    }

    fn set_option(parts: &[&str]) -> Result<Self, UciParseError> {
        let mut iter = parts.iter();
        let name = iter.next().map(|s| s.to_string());
        if name.is_none() {
            return Err(UciParseError::MissingArgument);
        }
        let value = iter.next().map(|s| s.to_string());
        Ok(UciCommand::SetOption {
            name: name.unwrap(),
            value,
        })
    }

    fn register(parts: &[&str]) -> Self {
        let mut name = None;
        let mut code = None;
        let mut later = false;
        let mut iter = parts.iter();
        while let Some(toke) = iter.next() {
            if *toke == "later" {
                later = true;
            } else if *toke == "name" {
                name = iter.next().map(|s| s.to_string());
            } else if *toke == "code" {
                code = iter.next().map(|s| s.to_string());
            }
        }
        UciCommand::Register { name, code, later }
    }

    fn uci_new_game() -> Self {
        UciCommand::UciNewGame
    }

    fn position(parts: &[&str]) -> Result<Self, UciParseError> {
        let mut fen = None;
        let mut moves = Vec::new();
        let mut iter = parts.iter();
        while let Some(toke) = iter.next() {
            if *toke == "fen" {
                fen = iter.next().map(|s| s.to_string());
            } else if *toke == "moves" {
                while let Some(mv) = iter.next() {
                    if let Ok(m) = ChessMove::from_str(mv) {
                        moves.push(m);
                    } else {
                        return Err(UciParseError::InvalidUciMove);
                    }
                }
            }
        }
        Ok(UciCommand::Position { fen, moves })
    }

    fn go(parts: &[&str]) -> Self {
        let mut depth = None;
        let mut movetime = None;
        let mut infinite = false;
        let mut iter = parts.iter();
        while let Some(toke) = iter.next() {
            match *toke {
                "depth" => {
                    depth = iter.next().and_then(|s| s.parse().ok());
                }
                "movetime" => {
                    movetime = iter.next().and_then(|s| s.parse().ok());
                }
                "infinite" => {
                    infinite = true;
                }
                _ => (),
            }
        }
        UciCommand::Go {
            depth,
            movetime,
            infinite,
        }
    }

    fn stop() -> Self {
        UciCommand::Stop
    }

    fn debug(parts: &[&str]) -> Self {
        let mut debug = false;
        if let Some(toke) = parts.get(0) {
            debug = *toke == "on";
        }
        UciCommand::Debug(debug)
    }

    fn quit() -> Self {
        UciCommand::Quit
    }
}

impl FromStr for UciCommand {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.is_empty() {
            return Err(());
        }
        match parts[0] {
            "uci" => Ok(UciCommand::uci()),
            "isready" => Ok(UciCommand::is_ready()),
            "setoption" => UciCommand::set_option(&parts[1..]).map_err(|_| ()),
            "register" => Ok(UciCommand::register(&parts[1..])),
            "ucinewgame" => Ok(UciCommand::uci_new_game()),
            "position" => UciCommand::position(&parts[1..]).map_err(|_| ()),
            "go" => Ok(UciCommand::go(&parts[1..])),
            "stop" => Ok(UciCommand::stop()),
            "debug" => Ok(UciCommand::debug(&parts[1..])),
            "quit" => Ok(UciCommand::quit()),
            _ => Err(()),
        }
    }
}

pub struct UciHandler {
    rx: Receiver<UciCommand>,
    tx: Sender<()>, // this is to send a quit signal to the uci thread.
    handle: JoinHandle<()>,
}

impl ThreadHandler<(), UciCommand> for UciHandler {
    fn receiver(&self) -> Receiver<UciCommand> {
        self.rx.clone()
    }

    fn sender(&self) -> Sender<()> {
        self.tx.clone()
    }

    fn quit(self) {
        if let Err(e) = self.tx.send(()) {
            eprintln!("Error sending quit signal to UCI thread: {:?}", e);
            return;
        }
        let _ = self.handle.join();
    }
}

pub struct Uci;

impl Uci {
    fn update_quit_signal(rx: Receiver<()>, quit: &mut bool) {
        match rx.try_recv() {
            Ok(_) => {
                println!("uci interface detected quit signal");
                *quit = true
            }
            Err(e) if e == TryRecvError::Empty => (),
            Err(e) => {
                eprintln!("Error receiving quit signal: {:?}", e);
                *quit = true;
            }
        }
    }

    pub fn init() -> UciHandler {
        let (uci_tx, uci_rx) = unbounded::<UciCommand>();
        let (quit_tx, quit_rx) = unbounded::<()>();
        let mut quit = false;
        let handle = spawn(move || {
            let mut stdin = BufReader::new(stdin());
            while !quit {
                let mut line = String::new();
                stdin.read_line(&mut line).unwrap();
                if let Ok(command) = UciCommand::from_str(&line) {
                    if let Err(e) = uci_tx.send(command) {
                        eprintln!("Error sending UCI command: {:?}", e);
                        break;
                    }
                }
                Self::update_quit_signal(quit_rx.clone(), &mut quit);
            }
        });

        return UciHandler {
            rx: uci_rx,
            tx: quit_tx,
            handle,
        };
    }
}

#[derive(Debug)]
pub struct UCITestEngine {
    outdir: String,
    iterations: u32,
    mtime: u64, // milliseconds for each engine to think.
}

impl UCITestEngine {
    pub fn new(outdir: String, iterations: u32, mtime: u64) -> Self {
        Self {
            outdir,
            iterations,
            mtime,
        }
    }

    pub fn run(&self, eng1_path: String, eng2_path: String) -> Result<(), std::io::Error> {
        let eng1 = std::process::Command::new(eng1_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .env_remove("RUST_CHESS_TEST_MODE")
            .spawn()?;
        let eng2 = Command::new(eng2_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .env_remove("RUST_CHESS_TEST_MODE")
            .spawn()?;
        return self.run_tests(eng1, eng2);
    }

    pub fn run_tests(&self, eng1: Child, eng2: Child) -> Result<(), std::io::Error> {
        create_dir_all(&self.outdir)?;
        let eng1_id = eng1.id();
        let mut white = eng1;
        let mut black = eng2;
        self.setup_engine(&mut white)?;
        self.setup_engine(&mut black)?;
        let mut game = Game::new();
        let mut encoder = PgnEncoder::new(game.current_position().clone(), None);
        let mut eng1_wins = 0;
        let mut eng2_wins = 0;
        let mut white_wins = 0;
        let mut black_wins = 0;
        let mut draws = 0;

        for game_num in 0..self.iterations {
            while game.result().is_none() && !game.can_declare_draw() {
                self.send_postion_fen(&mut white, &game.current_position().to_string())?;
                self.send_go(&mut white)?;
                let white_move = self.wait_for_bestmove(&mut white)?;
                game.make_move(white_move);
                encoder.add_move(white_move);

                if game.result().is_some() || game.can_declare_draw() {
                    break;
                }

                self.send_postion_fen(&mut black, &game.current_position().to_string())?;
                self.send_go(&mut black)?;
                let black_move = self.wait_for_bestmove(&mut black)?;
                game.make_move(black_move);
                encoder.add_move(black_move);

                if game.result().is_some() || game.can_declare_draw() {
                    break;
                }
            }

            if let Some(result) = game.result() {
                match result {
                    chess::GameResult::WhiteCheckmates => {
                        white_wins += 1;
                        if white.id() == eng1_id {
                            eng1_wins += 1;
                        } else {
                            eng2_wins += 1;
                        }
                    }
                    chess::GameResult::BlackCheckmates => {
                        black_wins += 1;
                        if black.id() == eng1_id {
                            eng1_wins += 1;
                        } else {
                            eng2_wins += 1;
                        }
                    }
                    _ => draws += 1,
                };
            } else {
                draws += 1;
            }

            let pgn = encoder.encode();
            let filename = format!("{}/game_{}.pgn", self.outdir, game_num);
            Self::write_pgn_evidence(filename, pgn)?;

            let tmp = black;
            black = white;
            white = tmp;
            game = Game::new();
            encoder = PgnEncoder::new(game.current_position().clone(), None);
            println!("Game {} complete", game_num);
            println!("Engine 1 wins: {}", eng1_wins);
            println!("Engine 2 wins: {}", eng2_wins);
            println!("Draws: {}", draws);
            println!("White wins: {}", white_wins);
            println!("Black wins: {}", black_wins);
        }

        let results_filename = format!("{}/results.txt", self.outdir);
        Self::write_results(
            results_filename,
            eng1_wins,
            eng2_wins,
            draws,
            self.iterations,
        )?;
        Ok(())
    }

    fn setup_engine(&self, engine: &mut Child) -> Result<(), std::io::Error> {
        let mut stdin = engine.stdin.as_ref().unwrap();
        let mut stdout = BufReader::new(engine.stdout.as_mut().unwrap());
        writeln!(stdin, "uci")?;
        stdin.flush()?;
        let mut line = String::new();
        stdout.read_line(&mut line)?;
        while !line.contains("uciok") {
            print!("engout -> {}", line);
            line.clear();
            if stdout.read_line(&mut line)? == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Unexpected EOF",
                ));
            }
        }
        print!("engout -> {}", line);
        Ok(())
    }

    fn send_postion_fen(&self, engine: &mut Child, position: &str) -> Result<(), std::io::Error> {
        let mut stdin = engine.stdin.as_ref().unwrap();
        println!("sending position \"{}\"", position);
        writeln!(stdin, "position fen {}", position)?;
        stdin.flush()?;
        Ok(())
    }

    fn send_go(&self, engine: &mut Child) -> Result<(), std::io::Error> {
        let mut stdin = engine.stdin.as_ref().unwrap();
        writeln!(stdin, "go movetime {}", self.mtime)?;
        stdin.flush()?;
        Ok(())
    }

    fn wait_for_bestmove(&self, engine: &mut Child) -> Result<ChessMove, std::io::Error> {
        let mut stdout = BufReader::new(engine.stdout.as_mut().unwrap());
        let mut line = String::new();
        stdout.read_line(&mut line)?;
        while !line.contains("bestmove") {
            print!("engout -> {}", line);
            line.clear();
            if stdout.read_line(&mut line)? == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Unexpected EOF",
                ));
            }
        }
        print!("engout -> {}", line);
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.len() < 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid bestmove response",
            ));
        }

        ChessMove::from_str(parts[1]).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid bestmove response")
        })
    }

    fn write_results(
        path: String,
        eng1_wins: i32,
        eng2_wins: i32,
        draws: i32,
        game_num: u32,
    ) -> Result<(), std::io::Error> {
        let mut file = std::fs::File::create(path)?;
        writeln!(file, "Results for {} games", game_num)?;
        writeln!(file, "Engine 1 wins: {}", eng1_wins)?;
        writeln!(file, "Engine 2 wins: {}", eng2_wins)?;
        writeln!(file, "Draws: {}", draws)?;
        Ok(())
    }

    fn write_pgn_evidence(path: String, pgn: String) -> Result<(), std::io::Error> {
        let mut file = std::fs::File::create(path)?;
        writeln!(file, "{}", pgn)?;
        Ok(())
    }

    pub fn set_outdir(&mut self, outdir: String) {
        self.outdir = outdir;
    }

    pub fn set_iterations(&mut self, iterations: u32) {
        self.iterations = iterations;
    }

    pub fn set_mtime(&mut self, mtime: u64) {
        self.mtime = mtime;
    }
}

impl Default for UCITestEngine {
    fn default() -> Self {
        Self {
            outdir: "./tmp/games".to_string(),
            iterations: 10,
            mtime: 2500,
        }
    }
}
