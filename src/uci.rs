use crate::engine::Engine;
use crate::evaluation::NegaMaxOptions;
use chess::{Board, ChessMove};
use std::collections::HashMap;
use std::io::{stdin, stdout, BufRead, BufReader, Write};
use std::str::FromStr;
use std::thread::spawn;
use std::time::{Duration, Instant};

pub type UCIEngineOptions = HashMap<String, String>;

pub struct UCIEngine<T: Engine + Send + 'static> {
    stdin: BufReader<std::io::Stdin>,
    stdout: std::io::Stdout,
    engine: Option<T>,
    initializer: fn(opts: UCIEngineOptions) -> T,
    opts: UCIEngineOptions,
    name: String,
    author: String,
    expect_ucinewgame: bool,
    reg_later: bool,
    debug: bool,
    board: Board,
}

impl<T: Engine + Send + 'static> UCIEngine<T> {
    /// Create a new UCIEngine with an initializer function that builds an engine instance from a set of options.
    pub fn new(initializer: fn(opts: UCIEngineOptions) -> T) -> Self {
        Self {
            stdin: BufReader::new(stdin()),
            stdout: stdout(),
            engine: None,
            initializer,
            opts: HashMap::new(),
            name: "Rust Engine".to_string(),
            author: "Jacob Policano".to_string(),
            expect_ucinewgame: false,
            reg_later: false,
            debug: false,
            board: Board::default(),
        }
    }

    /// Main loop: read lines from stdin and dispatch UCI commands.
    pub fn run(&mut self) -> Result<(), std::io::Error> {
        loop {
            let mut line = String::new();

            if self.stdin.read_line(&mut line)? == 0 {
                break; // EOF reached
            }

            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.trim().split_whitespace().collect();
            match parts[0] {
                "uci" => self.handle_uci()?,
                "isready" => self.handle_isready()?,
                "setoption" => self.handle_setoption(&parts[1..])?,
                "register" => self.handle_register(&parts[1..])?,
                "ucinewgame" => self.handle_ucinewgame()?,
                "position" => self.handle_position(&parts[1..])?,
                "go" => self.handle_go(&parts[1..])?,
                "stop" => self.handle_stop()?,
                "ponderhit" => self.handle_ponderhit()?,
                "debug" => self.handle_debug(&parts[1..])?,
                "quit" => break,
                _ => (),
            }
        }
        Ok(())
    }

    /// Handles the "uci" command by sending the id, available options, and finally "uciok".
    fn handle_uci(&mut self) -> Result<(), std::io::Error> {
        writeln!(self.stdout, "id name {}", self.name)?;
        writeln!(self.stdout, "id author {}", self.author)?;
        // Here you can output any engine options you support.
        writeln!(self.stdout, "option name Debug type check default false")?;
        writeln!(
            self.stdout,
            "option name Hash type spin default 16 min 1 max 128"
        )?;
        // (Add additional options here as desired.)
        writeln!(self.stdout, "uciok")?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Responds to "isready" by immediately sending "readyok".
    fn handle_isready(&mut self) -> Result<(), std::io::Error> {
        writeln!(self.stdout, "readyok")?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Parses the "setoption" command of the form:
    ///     setoption name <id> [value <x>]
    /// and stores the option in the opts map.
    fn handle_setoption(&mut self, line: &[&str]) -> Result<(), std::io::Error> {
        let mut toke_iter = line.iter();
        let mut name_pieces = Vec::new();
        let mut value_pieces = Vec::new();
        while let Some(toke) = toke_iter.next() {
            if *toke == "name" {
                while let Some(toke) = toke_iter.next() {
                    if *toke == "value" {
                        break;
                    }
                    name_pieces.push(*toke);
                }
            } else if *toke == "value" {
                while let Some(toke) = toke_iter.next() {
                    value_pieces.push(*toke);
                }
            }
        }
        let name = name_pieces.join(" ");
        let value = value_pieces.join(" ");
        if name.is_empty() || value.is_empty() {
            return Ok(());
        }
        self.opts.insert(name, value);
        Ok(())
    }

    /// Handles the "register" command (here simply a no-op).
    fn handle_register(&mut self, _line: &[&str]) -> Result<(), std::io::Error> {
        // Registration logic could be added here if needed.
        Ok(())
    }

    /// Handles the "ucinewgame" command by reinitializing the engine.
    fn handle_ucinewgame(&mut self) -> Result<(), std::io::Error> {
        self.expect_ucinewgame = true;
        Ok(())
    }

    /// Parses the "position" command, supporting both "startpos" and "fen" specifications,
    /// and then applies any moves provided.
    fn handle_position(&mut self, mut tokens: &[&str]) -> Result<(), std::io::Error> {
        // "position [fen <fenstring> | startpos] [moves <move1> ... <movei>]
        if tokens.len() == 0 {
            return Ok(());
        }
        // Initialize engine if not already done.
        if self.engine.is_none() {
            self.engine = Some((self.initializer)(self.opts.clone()));
        }

        if tokens[0] == "startpos" {
            // Start from the default starting position.
            if tokens.len() > 1 && tokens[1] == "moves" {
                self.apply_moves(&tokens[2..]);
            }
        } else if tokens[0] == "fen" {
            // The FEN string may contain spaces – it is taken until the optional "moves" token.
            let mut fen_parts = Vec::new();
            tokens = &tokens[1..];
            while tokens.len() != 0 && tokens[0] != "moves" {
                fen_parts.push(tokens[0]);
                tokens = &tokens[1..];
            }
            let fen = fen_parts.join(" ");
            self.board = Board::from_str(&fen).unwrap();
            if tokens.len() != 0 && tokens[0] == "moves" {
                self.apply_moves(&tokens[1..]);
            }
        }
        Ok(())
    }

    /// Applies a list of move strings to the engine by parsing them into ChessMove objects.
    fn apply_moves(&mut self, moves: &[&str]) {
        for mv in moves {
            match ChessMove::from_str(mv) {
                Ok(m) => self.board = self.board.make_move_new(m),
                Err(e) => {
                    let _ = write!(self.stdout, "Err({e}) invalid uci move {mv}");
                    break;
                }
            }
        }
    }

    /// Handles the "go" command. This implementation ignores extra parameters (e.g. time, depth)
    /// and simply queries the engine for its next move.
    fn handle_go(&mut self, tokens: &[&str]) -> Result<(), std::io::Error> {
        let mut time: Option<u64> = None;
        let mut depth: Option<i8> = None;
        let mut iter = tokens.iter();

        while let Some(toke) = iter.next() {
            match *toke {
                "wtime" => {
                    // white time remaining
                    let _ = iter.next();
                }
                "btime" => {
                    // black time remaining
                    let _ = iter.next();
                }
                "winc" => {
                    // white increment
                    let _ = iter.next();
                }
                "binc" => {
                    // black increment
                    let _ = iter.next();
                }
                "movestogo" => {
                    // moves to go
                    let _ = iter.next();
                }
                "depth" => {
                    // depth
                    if let Some(d) = iter.next() {
                        if let Ok(d) = d.parse::<i8>() {
                            depth = Some(d);
                        }
                    }
                }
                "nodes" => {
                    // nodes
                    let _ = iter.next();
                }
                "mate" => {
                    // mate in x
                    let _ = iter.next();
                }
                "movetime" => {
                    // move time
                    if let Some(ms_str) = iter.next() {
                        if let Ok(ms) = ms_str.parse::<u64>() {
                            time = Some(ms);
                        }
                    };
                }
                "infinite" => {
                    // infinite search
                }
                "ponder" => {
                    // ponder
                }
                _ => {
                    // unknown
                }
            }
        }

        if let Some(engine) = self.engine.take() {
            let mut opts = NegaMaxOptions::new();
            if let Some(t) = time {
                opts = opts.mtime(t);
            }
            if let Some(d) = depth {
                opts = opts.depth(d);
            }
            self.spawn_engine_thread(engine, self.board, opts);
        } else {
            writeln!(self.stdout, "bestmove 0000")?;
        }
        self.stdout.flush()?;
        Ok(())
    }

    fn spawn_engine_thread(&mut self, engine: T, board: Board, opts: NegaMaxOptions) {
        spawn(move || {
            if let Some(mv) = engine.next_move(&board, opts) {
                println!("bestmove {}", mv);
            } else {
                println!("bestmove 0000");
            }
        });
    }

    /// Handles the "stop" command.
    /// In this synchronous implementation (where the search is blocking), this is a no-op.
    fn handle_stop(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }

    /// Handles the "ponderhit" command.
    /// For this simple example, pondering is not supported.
    fn handle_ponderhit(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }

    /// Handles the "debug" command (e.g. "debug on" or "debug off") and updates the internal flag.
    fn handle_debug(&mut self, _tokens: &[&str]) -> Result<(), std::io::Error> {
        Ok(())
    }
}
