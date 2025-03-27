use chess::{ChessMove, Piece, Square};
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, PartialOrd, Copy)]
pub enum NodeType {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Debug, Clone, Copy)]
pub struct TTEntry {
    pub hash: u64,
    pub value: TTData,
}

impl TTEntry {
    pub fn new(hash: u64, value: TTData) -> Self {
        Self { hash, value }
    }
}

impl Default for TTEntry {
    fn default() -> Self {
        Self {
            hash: 0,
            value: TTData::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TTData {
    pub depth: u8,
    pub score: i16,
    pub m: ChessMove,
    pub node_type: NodeType,
}

impl Default for TTData {
    fn default() -> Self {
        Self {
            depth: 0,
            score: 0,
            m: ChessMove::default(),
            node_type: NodeType::Exact,
        }
    }
}

#[derive(Debug)]
pub struct TT {
    table: Box<[Mutex<Option<TTEntry>>]>,
    mask: usize,
}

impl TT {
    pub fn new(size: usize) -> Self {
        if size.count_ones() != 1 {
            panic!("You cannot create a TT with a non-binary number.");
        }
        let mut table = Vec::with_capacity(size);
        for _ in 0..size {
            table.push(Mutex::new(None));
        }
        return Self {
            table: table.into_boxed_slice(),
            mask: size - 1,
        };
    }

    pub fn get(&self, hash: u64) -> Option<TTData> {
        let entry = unsafe { self.table.get_unchecked((hash as usize) & self.mask) };
        let entry = entry.lock().expect("lock to not be poisoned inside tt");
        if let Some(e) = entry.as_ref() {
            if e.hash == hash {
                return Some(e.value);
            }
        }
        return None;
    }

    pub fn set(
        &self,
        hash: u64,
        depth: u8,
        score: i16,
        m: ChessMove,
        original_alpha: i16,
        beta: i16,
    ) {
        let entry = unsafe { self.table.get_unchecked((hash as usize) & self.mask) };
        let mut entry = entry.lock().expect("lock to not be poisoned inside tt");
        if let Some(e) = entry.as_mut() {
            if e.value.depth > depth {
                return;
            }
            let node_type = if score <= original_alpha {
                NodeType::UpperBound
            } else if score >= beta {
                NodeType::LowerBound
            } else {
                NodeType::Exact
            };
            e.hash = hash;
            e.value = TTData {
                depth,
                score,
                m,
                node_type,
            };
            return;
        }
        *entry = Some(TTEntry::new(
            hash,
            TTData {
                depth,
                score,
                m,
                node_type: NodeType::Exact,
            },
        ));
    }
}
