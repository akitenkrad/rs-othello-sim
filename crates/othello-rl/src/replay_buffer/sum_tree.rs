//! [`SumTree`]: data structure for proportional sampling in Prioritized
//! Experience Replay.
//!
//! A complete binary tree whose leaves hold priorities and whose internal
//! nodes hold the sum of their children. `total()` is `O(1)`, and the
//! cumulative-sum lookup `get(value)` is `O(log N)`.

/// Sum-tree: a complete binary tree with `capacity` leaves.
///
/// The internal representation is a `Vec<f32>` of length `2 * capacity - 1`.
/// The first `capacity - 1` entries are internal nodes (`0..capacity-1`)
/// and the remaining `capacity` entries are leaves
/// (`capacity-1..2*capacity-1`). The relationship between a leaf index
/// `leaf_id` (`0..capacity`) and the internal representation index is
/// `tree_index = leaf_id + capacity - 1`.
#[derive(Debug, Clone)]
pub struct SumTree {
    capacity: usize,
    nodes: Vec<f32>,
    cursor: usize,
    size: usize,
}

impl SumTree {
    /// Creates a sum-tree with `capacity` leaves. `capacity` must be at
    /// least 1.
    ///
    /// # Panics
    ///
    /// Panics if `capacity == 0`.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "SumTree capacity must be > 0");
        Self {
            capacity,
            nodes: vec![0.0; 2 * capacity - 1],
            cursor: 0,
            size: 0,
        }
    }

    /// Returns the leaf capacity.
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of elements currently stored.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.size
    }

    /// Returns whether the tree is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Returns the total sum (sum of all leaf priorities).
    #[inline]
    #[must_use]
    pub fn total(&self) -> f32 {
        self.nodes[0]
    }

    /// Adds one element at the current ring position and returns the leaf
    /// index that was written.
    pub fn add(&mut self, priority: f32) -> usize {
        let leaf_id = self.cursor;
        self.update(leaf_id, priority);
        self.cursor = (self.cursor + 1) % self.capacity;
        if self.size < self.capacity {
            self.size += 1;
        }
        leaf_id
    }

    /// Updates the priority of the leaf at `leaf_id`.
    ///
    /// # Panics
    ///
    /// Panics if `leaf_id >= capacity`.
    pub fn update(&mut self, leaf_id: usize, priority: f32) {
        assert!(leaf_id < self.capacity, "leaf_id {leaf_id} out of range");
        let mut idx = leaf_id + self.capacity - 1;
        let change = priority - self.nodes[idx];
        self.nodes[idx] = priority;
        while idx > 0 {
            idx = (idx - 1) / 2;
            self.nodes[idx] += change;
        }
    }

    /// Returns the first leaf whose cumulative sum reaches or exceeds
    /// `value`.
    ///
    /// The return value is `(leaf_id, priority)`. `value` is expected to
    /// fall in `[0, total())`; out-of-range values safely return the last
    /// leaf.
    #[must_use]
    pub fn get(&self, mut value: f32) -> (usize, f32) {
        let mut idx = 0usize;
        // internal node の範囲は 0..capacity-1
        while idx < self.capacity - 1 {
            let left = 2 * idx + 1;
            let right = left + 1;
            if value <= self.nodes[left] {
                idx = left;
            } else {
                value -= self.nodes[left];
                idx = right;
            }
        }
        let leaf_id = idx - (self.capacity - 1);
        (leaf_id, self.nodes[idx])
    }

    /// Resets every leaf to zero.
    pub fn clear(&mut self) {
        self.nodes.fill(0.0);
        self.cursor = 0;
        self.size = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_one_works() {
        let mut t = SumTree::new(1);
        let id = t.add(1.5);
        assert_eq!(id, 0);
        assert!((t.total() - 1.5).abs() < 1e-6);
        let (lid, p) = t.get(0.0);
        assert_eq!(lid, 0);
        assert!((p - 1.5).abs() < 1e-6);
    }

    #[test]
    fn add_four_priorities_total() {
        let mut t = SumTree::new(4);
        t.add(1.0);
        t.add(2.0);
        t.add(3.0);
        t.add(4.0);
        assert!((t.total() - 10.0).abs() < 1e-6);
        assert_eq!(t.len(), 4);
    }

    #[test]
    fn get_returns_correct_leaf() {
        let mut t = SumTree::new(4);
        t.add(1.0);
        t.add(2.0);
        t.add(3.0);
        t.add(4.0);
        // Cumulative ranges: leaf 0: [0, 1), leaf 1: [1, 3), leaf 2: [3, 6), leaf 3: [6, 10).
        // Note: `get` is "first leaf whose cumulative sum is >= value", so 0 -> leaf 0,
        // 0.5 -> leaf 0, 1.5 -> leaf 1, 4 -> leaf 2, 9 -> leaf 3.
        let (id0, _) = t.get(0.5);
        assert_eq!(id0, 0);
        let (id1, _) = t.get(1.5);
        assert_eq!(id1, 1);
        let (id2, _) = t.get(4.0);
        assert_eq!(id2, 2);
        let (id3, _) = t.get(9.0);
        assert_eq!(id3, 3);
    }

    #[test]
    fn update_changes_total() {
        let mut t = SumTree::new(4);
        t.add(1.0);
        t.add(2.0);
        t.add(3.0);
        t.add(4.0);
        t.update(0, 5.0);
        assert!((t.total() - 14.0).abs() < 1e-6);
    }

    #[test]
    fn ring_overwrite() {
        let mut t = SumTree::new(2);
        t.add(1.0);
        t.add(2.0);
        // Now at full; cursor wraps around
        let id = t.add(10.0);
        assert_eq!(id, 0);
        assert!((t.total() - 12.0).abs() < 1e-6);
    }

    #[test]
    fn clear_resets() {
        let mut t = SumTree::new(4);
        t.add(1.0);
        t.add(2.0);
        t.clear();
        assert_eq!(t.len(), 0);
        assert!((t.total()).abs() < 1e-6);
    }
}
