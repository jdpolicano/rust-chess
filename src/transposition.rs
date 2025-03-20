use chess::{ChessMove, Piece, Square};
use std::sync::Mutex;

// 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 1111 - promotion option (4 bits)
// 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 1111 1111 0000 - destination square (8 bits)
// 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 1111 1111 0000 0000 0000 - origin square (8 bits)
// 0000 0000 0000 0000 0000 0000 0000 1111 1111 1111 1111 0000 0000 0000 0000 0000 - score (16 bits)
// 0000 0000 0000 0000 0000 1111 1111 0000 0000 0000 0000 0000 0000 0000 0000 0000 - depth (8 bits)
// 0000 0000 0000 0000 1111 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 - type (4 bits)
// Promotion: bits 0-3
const PROMOTION_MASK: u64 = 0x000000000000000F;

// Destination square: bits 4-11
const DESTINATION_MASK: u64 = 0x0000000000000FF0;

// Origin square: bits 12-19
const ORIGIN_MASK: u64 = 0x00000000000FF000;

// Score: bits 20-35
const SCORE_MASK: u64 = 0x0000000FFFF00000;

// Depth: bits 36-43
const DEPTH_MASK: u64 = 0x00000FF000000000;

// Type: bits 44-47
const TYPE_MASK: u64 = 0x0000F00000000000;

// the bit for options on the promotion

const PROMOTION_SHIFT: u8 = 0;
const DESTINATION_SHIFT: u8 = 4;
const ORIGIN_SHIFT: u8 = 12;
const SCORE_SHIFT: u8 = 20;
const DEPTH_SHIFT: u8 = 36;
const TYPE_SHIFT: u8 = 44;

#[derive(Debug, Clone, PartialEq, PartialOrd, Copy)]
pub enum NodeType {
    Exact,
    LowerBound,
    UpperBound,
}

impl NodeType {
    pub fn to_byte(&self) -> u8 {
        *self as u8
    }
}

impl From<u64> for NodeType {
    fn from(byte: u64) -> Self {
        match byte {
            0 => NodeType::Exact,
            1 => NodeType::LowerBound,
            2 => NodeType::UpperBound,
            _ => panic!("Invalid node type"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TTEntry {
    pub hash: u64,
    pub value: u64,
}

impl TTEntry {
    pub fn new(hash: u64, value: u64) -> Self {
        Self { hash, value }
    }
}

impl Default for TTEntry {
    fn default() -> Self {
        Self { hash: 0, value: 0 }
    }
}

pub struct TTData {
    pub depth: u8,
    pub score: i16,
    pub m: ChessMove,
    pub node_type: NodeType,
}

impl TTData {
    fn make_square(sq: u64) -> Square {
        assert!(sq < 64, "Square index out of bounds: {}", sq);
        unsafe { Square::new(sq as u8) }
    }

    fn make_piece(p: u64) -> Option<Piece> {
        match p {
            1 => Some(Piece::Knight),
            2 => Some(Piece::Bishop),
            3 => Some(Piece::Rook),
            4 => Some(Piece::Queen),
            _ => None,
        }
    }
}

impl From<u64> for TTData {
    fn from(packed: u64) -> Self {
        let depth = (packed & DEPTH_MASK) >> DEPTH_SHIFT;
        let score = (packed & SCORE_MASK) >> SCORE_SHIFT;
        let dest = Self::make_square((packed & DESTINATION_MASK) >> DESTINATION_SHIFT);
        let orig = Self::make_square((packed & ORIGIN_MASK) >> ORIGIN_SHIFT);
        let promotion = Self::make_piece((packed & PROMOTION_MASK) >> PROMOTION_SHIFT);
        let node_type: NodeType = ((packed & TYPE_MASK) >> TYPE_SHIFT).into();
        return Self {
            depth: depth as u8,
            score: score as i16,
            m: ChessMove::new(orig, dest, promotion),
            node_type,
        };
    }
}

#[derive(Debug)]
pub struct TT {
    table: Box<[Mutex<TTEntry>]>,
    mask: usize,
}

impl TT {
    pub fn new(size: usize) -> Self {
        if size.count_ones() != 1 {
            panic!("You cannot create a TT with a non-binary number.");
        }
        let mut table = Vec::with_capacity(size);
        for _ in 0..size {
            table.push(Mutex::new(TTEntry::default()));
        }
        return Self {
            table: table.into_boxed_slice(),
            mask: size - 1,
        };
    }

    pub fn get(&self, hash: u64) -> Option<TTData> {
        let entry = unsafe { self.table.get_unchecked((hash as usize) & self.mask) };
        let entry = entry.lock().expect("lock to not be poisoned inside tt");
        if entry.hash == hash {
            return Some(TTData::from(entry.value));
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
        let node_type = if score <= original_alpha {
            NodeType::UpperBound
        } else if score >= beta {
            NodeType::LowerBound
        } else {
            NodeType::Exact
        };
        entry.hash = hash;
        entry.value = pack_move(m, score, depth, node_type);
    }
}

// impl TableEntry {
//     pub fn new(depth: u8, score: i32, m: ChessMove, original_alpha: i32, beta: i32) -> Self {
//         let node_type = if score <= original_alpha {
//             NodeType::UpperBound
//         } else if score >= beta {
//             NodeType::LowerBound
//         } else {
//             NodeType::Exact
//         };
//         return Self {
//             depth,
//             score,
//             m,
//             node_type,
//             age: Instant::now(),
//             lock: Mutex::new(()),
//         };
//     }

//     pub fn lock(&self) -> std::sync::MutexGuard<()> {
//         return self.lock.lock().expect("lock to not be poisoned inside tt");
//     }
// }

// impl PartialEq for TableEntry {
//     fn eq(&self, other: &Self) -> bool {
//         return self.depth == other.depth
//             && self.score == other.score
//             && self.m == other.m
//             && self.node_type == other.node_type
//             && self.age == other.age;
//     }
// }

// impl PartialOrd for TableEntry {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         return Some(self.age.cmp(&other.age));
//     }
// }

// pub struct TranspositionTable {
//     table: CacheTable<TableEntry>,
// }
fn pack_move(m: ChessMove, score: i16, depth: u8, t: NodeType) -> u64 {
    let promotion = match m.get_promotion() {
        Some(p) => match p {
            Piece::Knight => 1,
            Piece::Bishop => 2,
            Piece::Rook => 3,
            Piece::Queen => 4,
            _ => 6, // for none.
        },
        _ => 6, // for none.
    };
    let dest = m.get_dest().to_index() as u64;
    let orig = m.get_source().to_index() as u64;
    let score = score as u64;
    let depth = depth as u64;

    return promotion << PROMOTION_SHIFT
        | dest << DESTINATION_SHIFT
        | orig << ORIGIN_SHIFT
        | score << SCORE_SHIFT
        | depth << DEPTH_SHIFT
        | (t.to_byte() as u64) << TYPE_SHIFT;
}
