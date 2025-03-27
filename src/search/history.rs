use super::alpha_beta::MAX_PLY;
use std::cell::UnsafeCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct MoveHistory {
    moves: Rc<UnsafeCell<Vec<u64>>>,
}

impl MoveHistory {
    pub fn new(len: usize) -> Self {
        Self {
            moves: Rc::new(UnsafeCell::new(Vec::with_capacity(len + MAX_PLY as usize))),
        }
    }

    pub fn from_vec(positions: &[u64]) -> Self {
        let history = Self::new(positions.len());
        for &m in positions.iter() {
            history.push(m);
        }
        history
    }

    // Unsafe methods that allow interior mutation without runtime checks.
    pub fn push(&self, b_hash: u64) {
        unsafe {
            (*self.moves.get()).push(b_hash);
        };
    }

    pub fn pop(&self) -> Option<u64> {
        unsafe { (*self.moves.get()).pop() }
    }

    pub fn seen_times(&self, hash: u64) -> u8 {
        unsafe { (*self.moves.get()).iter().filter(|&&h| h == hash).count() as u8 }
    }

    pub fn len(&self) -> usize {
        unsafe { (*self.moves.get()).len() }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_move_history() {
        let mut history = MoveHistory::new(10);
        history.push(1);
        history.push(2);
        history.push(3);
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_move_history_pop() {
        let mut history = MoveHistory::new(10);
        history.push(1);
        history.push(2);
        history.push(3);
        assert_eq!(history.pop(), Some(3));
        assert_eq!(history.pop(), Some(2));
        assert_eq!(history.pop(), Some(1));
        assert_eq!(history.pop(), None);
    }

    #[test]
    fn test_move_history_seen_times() {
        let mut history = MoveHistory::new(10);
        history.push(1);
        history.push(2);
        history.push(3);
        history.push(1);
        history.push(2);
        history.push(3);
        assert_eq!(history.seen_times(1), 2);
        assert_eq!(history.seen_times(2), 2);
        assert_eq!(history.seen_times(3), 2);
    }
}
