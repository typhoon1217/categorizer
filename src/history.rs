use std::collections::VecDeque;
use std::path::PathBuf;

/// A single move operation that can be reversed.
#[derive(Debug, Clone)]
pub struct MoveOp {
    pub from: PathBuf,
    pub to: PathBuf,
}

/// Ring-buffer undo stack with a fixed max capacity.
pub struct UndoStack {
    entries: VecDeque<MoveOp>,
    capacity: usize,
}

impl UndoStack {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            capacity,
        }
    }

    /// Push a move operation. Drops oldest if at capacity.
    pub fn push(&mut self, op: MoveOp) {
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(op);
    }

    /// Pop the most recent operation. Returns None if empty.
    pub fn pop(&mut self) -> Option<MoveOp> {
        self.entries.pop_back()
    }

    /// True if there are no operations to undo.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of operations currently stored.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn op(from: &str, to: &str) -> MoveOp {
        MoveOp {
            from: PathBuf::from(from),
            to: PathBuf::from(to),
        }
    }

    #[test]
    fn test_empty_stack() {
        let stack = UndoStack::new(20);
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_push_and_pop() {
        let mut stack = UndoStack::new(20);
        stack.push(op("a.jpg", "cats/a.jpg"));
        stack.push(op("b.jpg", "dogs/b.jpg"));

        assert_eq!(stack.len(), 2);
        let last = stack.pop().unwrap();
        assert_eq!(last.from, PathBuf::from("b.jpg"));
        assert_eq!(last.to, PathBuf::from("dogs/b.jpg"));
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_pop_empty_returns_none() {
        let mut stack = UndoStack::new(20);
        assert!(stack.pop().is_none());
    }

    #[test]
    fn test_capacity_drops_oldest() {
        let mut stack = UndoStack::new(3);
        stack.push(op("first.jpg", "a/first.jpg"));
        stack.push(op("second.jpg", "a/second.jpg"));
        stack.push(op("third.jpg", "a/third.jpg"));
        stack.push(op("fourth.jpg", "a/fourth.jpg")); // should drop "first"

        assert_eq!(stack.len(), 3);
        // Pop in LIFO order
        let top = stack.pop().unwrap();
        assert_eq!(top.from, PathBuf::from("fourth.jpg"));
        // Pop again
        let next = stack.pop().unwrap();
        assert_eq!(next.from, PathBuf::from("third.jpg"));
        // One left should be second, not first
        let last = stack.pop().unwrap();
        assert_eq!(last.from, PathBuf::from("second.jpg"));
        assert!(stack.is_empty());
    }

    #[test]
    fn test_is_empty_after_all_popped() {
        let mut stack = UndoStack::new(20);
        stack.push(op("a.jpg", "x/a.jpg"));
        stack.pop();
        assert!(stack.is_empty());
    }
}
