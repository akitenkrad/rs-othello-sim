//! [`SumTree`]: Prioritized Experience Replay の比例サンプリング用データ構造．
//!
//! 完全二分木で，葉が priority を持ち，内部ノードは子の和を持つ．
//! `total()` 全体和は O(1)，`get(value)` の累積探索は O(log N)．

/// Sum-tree．`capacity` 件の leaf を持つ完全二分木．
///
/// 内部表現は `Vec<f32>` で長さ `2 * capacity - 1`．先頭の `capacity - 1` 個が
/// internal node ( 0..capacity-1)，残り `capacity` 個が leaf ( capacity-1..2*capacity-1)．
/// leaf index `leaf_id` ( 0..capacity) と内部表現 index の関係は
/// `tree_index = leaf_id + capacity - 1`．
#[derive(Debug, Clone)]
pub struct SumTree {
    capacity: usize,
    nodes: Vec<f32>,
    cursor: usize,
    size: usize,
}

impl SumTree {
    /// `capacity` 葉の sum-tree を作る．`capacity` は 1 以上．
    ///
    /// # Panics
    ///
    /// `capacity == 0` の場合 panic．
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

    /// 葉の容量．
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 現在格納されている要素数．
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.size
    }

    /// 空かどうか．
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// 総和 ( 全葉の priority の合計)．
    #[inline]
    #[must_use]
    pub fn total(&self) -> f32 {
        self.nodes[0]
    }

    /// リング位置に従って 1 件追加し，書き込んだ leaf index を返す．
    pub fn add(&mut self, priority: f32) -> usize {
        let leaf_id = self.cursor;
        self.update(leaf_id, priority);
        self.cursor = (self.cursor + 1) % self.capacity;
        if self.size < self.capacity {
            self.size += 1;
        }
        leaf_id
    }

    /// `leaf_id` 番目の leaf の priority を更新する．
    ///
    /// # Panics
    ///
    /// `leaf_id >= capacity` で panic．
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

    /// 累積和が `value` を超える ( もしくは等しくなる) 最初の leaf を返す．
    ///
    /// 戻り値は `(leaf_id, priority)`．`value` は `[0, total())` の範囲を想定．
    /// 範囲外の `value` でも安全に最後の leaf を返す．
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

    /// すべての leaf を 0 にリセットする．
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
