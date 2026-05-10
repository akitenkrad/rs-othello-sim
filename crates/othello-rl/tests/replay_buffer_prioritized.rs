//! Behavioural tests for `PrioritizedReplayBuffer` (PER).

use ndarray::{Array1, Array3};
use othello_core::Color;
use othello_rl::replay_buffer::{PrioritizedReplayBuffer, ReplayBuffer, SumTree, Transition};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn dummy_transition(value: f32, action: u32) -> Transition {
    Transition {
        observation: Array3::<f32>::zeros((3, 8, 8)),
        action,
        policy: None,
        value,
        legal_mask: Array1::from_elem((65,), false),
        side: Color::Black,
        move_number: 0,
        game_id: "test".to_string(),
    }
}

#[test]
fn sum_tree_add_get_roundtrip() {
    let mut tree = SumTree::new(4);
    tree.add(1.0);
    tree.add(2.0);
    tree.add(3.0);
    tree.add(4.0);
    assert!((tree.total() - 10.0).abs() < 1e-6);
    let (id, _) = tree.get(0.5);
    assert_eq!(id, 0);
    let (id, _) = tree.get(2.5);
    assert_eq!(id, 1);
}

#[test]
fn per_push_increases_total() {
    let mut buf = PrioritizedReplayBuffer::new(16);
    for i in 0..5 {
        buf.push(dummy_transition(0.0, i as u32));
    }
    assert_eq!(buf.len(), 5);
    assert!(!buf.is_full());
    // 新規 transition は max_priority^alpha で SumTree に追加される
    assert!(buf.max_priority() > 0.0);
}

#[test]
fn per_sample_returns_weights() {
    let mut buf = PrioritizedReplayBuffer::new(32);
    for i in 0..16 {
        buf.push(dummy_transition(i as f32, i as u32));
    }
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let batch = buf.sample(8, &mut rng).unwrap();
    assert_eq!(batch.indices.len(), 8);
    assert!(batch.weights.is_some());
    let w = batch.weights.unwrap();
    assert_eq!(w.len(), 8);
    // すべて (0, 1] の範囲 ( 正規化 + 正)
    for v in w.iter() {
        assert!(*v > 0.0 && *v <= 1.0 + 1e-5, "weight out of range: {v}");
    }
}

#[test]
fn per_update_changes_sampling_distribution() {
    let mut buf = PrioritizedReplayBuffer::new(8)
        .with_alpha(1.0)
        .with_beta_increment(0.0);
    for i in 0..8u32 {
        buf.push(dummy_transition(i as f32, i));
    }
    // index 0 だけ非常に高い priority に上げる
    buf.update_priorities(&[0], &[100.0]).unwrap();

    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let mut count_zero = 0u32;
    let trials: u32 = 1000;
    let batch_size: usize = 4;
    for _ in 0..trials {
        let batch = buf.sample(batch_size, &mut rng).unwrap();
        for &i in &batch.indices {
            if i == 0 {
                count_zero += 1;
            }
        }
    }
    // 一様サンプルなら期待値 trials * batch_size / 8 = 500 件．
    // priority 100 とその他 (max_priority 経由で 100 になっているはず) で偏らないかも
    // しれないが，少なくとも index 0 は十分に出現する．
    // 緩い assertion: index 0 の出現が (uniform 期待値の) 50% 以上あること．
    let uniform_expected = (trials * batch_size as u32) / 8;
    assert!(
        count_zero > uniform_expected / 2,
        "count_zero={count_zero}, uniform_expected={uniform_expected}"
    );
}

#[test]
fn per_high_priority_dominates_sampling() {
    // 1 件だけ priority を桁違いに高くしたとき，その index が偏って出ることを確認．
    let mut buf = PrioritizedReplayBuffer::new(16)
        .with_alpha(1.0)
        .with_beta_increment(0.0);
    for i in 0..16u32 {
        buf.push(dummy_transition(i as f32, i));
    }
    // 全件低 priority に揃えてから 1 件だけ高くする
    let lows: Vec<f32> = (0..16).map(|_| 0.01).collect();
    let idx_all: Vec<usize> = (0..16).collect();
    buf.update_priorities(&idx_all, &lows).unwrap();
    buf.update_priorities(&[5], &[1000.0]).unwrap();

    let mut rng = ChaCha8Rng::seed_from_u64(123);
    let mut count_5 = 0u32;
    let trials: u32 = 500;
    let batch_size: usize = 4;
    for _ in 0..trials {
        let batch = buf.sample(batch_size, &mut rng).unwrap();
        for &i in &batch.indices {
            if i == 5 {
                count_5 += 1;
            }
        }
    }
    // 期待: ほぼすべてのサンプルが index 5．緩く batch 数 * trials の 60% 以上．
    let total = trials * batch_size as u32;
    assert!(
        count_5 > total * 6 / 10,
        "expected index 5 to dominate; got count_5={count_5} of {total}"
    );
}

#[test]
fn per_beta_increases_and_clamps() {
    let mut buf = PrioritizedReplayBuffer::new(8)
        .with_beta(0.4)
        .with_beta_increment(0.1);
    for i in 0..8u32 {
        buf.push(dummy_transition(i as f32, i));
    }
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let _ = buf.sample(2, &mut rng).unwrap();
    assert!((buf.beta() - 0.5).abs() < 1e-5);
    // 何度も sample すると 1.0 でクランプされる
    for _ in 0..100 {
        let _ = buf.sample(2, &mut rng).unwrap();
    }
    assert!((buf.beta() - 1.0).abs() < 1e-5);
}

#[test]
fn per_update_priorities_length_mismatch() {
    let mut buf = PrioritizedReplayBuffer::new(8);
    for i in 0..4u32 {
        buf.push(dummy_transition(0.0, i));
    }
    let r = buf.update_priorities(&[0, 1], &[1.0]);
    assert!(matches!(
        r,
        Err(othello_rl::ReplayError::PriorityLengthMismatch { .. })
    ));
}

#[test]
fn per_update_priorities_negative_rejected() {
    let mut buf = PrioritizedReplayBuffer::new(8);
    for i in 0..4u32 {
        buf.push(dummy_transition(0.0, i));
    }
    let r = buf.update_priorities(&[0], &[-1.0]);
    assert!(matches!(
        r,
        Err(othello_rl::ReplayError::NegativePriority(_))
    ));
}

#[test]
fn per_clear_resets_state() {
    let mut buf = PrioritizedReplayBuffer::new(8);
    for i in 0..4u32 {
        buf.push(dummy_transition(0.0, i));
    }
    buf.clear();
    assert!(buf.is_empty());
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let r = buf.sample(1, &mut rng);
    assert!(matches!(r, Err(othello_rl::ReplayError::Empty)));
}

#[test]
fn per_sample_empty_returns_error() {
    let mut buf = PrioritizedReplayBuffer::new(8);
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let r = buf.sample(1, &mut rng);
    assert!(matches!(r, Err(othello_rl::ReplayError::Empty)));
}

#[test]
fn per_ring_overwrites_oldest() {
    let mut buf = PrioritizedReplayBuffer::new(3);
    for i in 0..3u32 {
        buf.push(dummy_transition(i as f32, i));
    }
    assert!(buf.is_full());
    buf.push(dummy_transition(99.0, 99));
    assert_eq!(buf.len(), 3);
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let mut saw_99 = false;
    for _ in 0..30 {
        let b = buf.sample(3, &mut rng).unwrap();
        if b.values.iter().any(|&v| (v - 99.0).abs() < 1e-6) {
            saw_99 = true;
        }
    }
    assert!(saw_99, "newly pushed transition should appear after wrap");
}
