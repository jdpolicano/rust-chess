use crate::engine::Engine;
use crate::evaluation::NegaMaxOptions;
use crate::pgn::PgnEncoder;
use chess::{Board, ChessMove, Game};
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::io::{stdin, stdout, BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use std::thread::spawn;

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
    pub reg_later: bool,
    pub debug: bool,
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
