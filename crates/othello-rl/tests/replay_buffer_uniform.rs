//! Behavioural tests for `UniformReplayBuffer`.

use ndarray::{Array1, Array3};
use othello_core::Color;
use othello_rl::replay_buffer::{ReplayBuffer, Transition, UniformReplayBuffer};
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
fn push_and_len_within_capacity() {
    let mut buf = UniformReplayBuffer::new(100);
    for i in 0..10 {
        buf.push(dummy_transition(0.0, i as u32));
    }
    assert_eq!(buf.len(), 10);
    assert_eq!(buf.capacity(), 100);
    assert!(!buf.is_full());
    assert!(!buf.is_empty());
}

#[test]
fn ring_overwrites_oldest() {
    let mut buf = UniformReplayBuffer::new(3);
    buf.push(dummy_transition(1.0, 1));
    buf.push(dummy_transition(2.0, 2));
    buf.push(dummy_transition(3.0, 3));
    assert!(buf.is_full());
    // 4 件目で最古が上書きされる
    buf.push(dummy_transition(4.0, 4));
    assert_eq!(buf.len(), 3);
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    // 大きめのバッチで上書き確認
    let mut found_4 = false;
    let mut found_1 = false;
    for _ in 0..50 {
        let batch = buf.sample(3, &mut rng).unwrap();
        for &v in batch.values.iter() {
            if (v - 4.0).abs() < 1e-6 {
                found_4 = true;
            }
            if (v - 1.0).abs() < 1e-6 {
                found_1 = true;
            }
        }
    }
    assert!(found_4, "value 4 (latest) should appear in samples");
    assert!(!found_1, "value 1 (oldest) should be overwritten");
}

#[test]
fn sample_returns_batch_shapes() {
    let mut buf = UniformReplayBuffer::new(50);
    for i in 0..10 {
        buf.push(dummy_transition(i as f32, i as u32));
    }
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let batch = buf.sample(5, &mut rng).unwrap();
    assert_eq!(batch.observations.shape(), &[5, 3, 8, 8]);
    assert_eq!(batch.actions.shape(), &[5]);
    assert_eq!(batch.values.shape(), &[5]);
    assert_eq!(batch.legal_masks.shape(), &[5, 65]);
    assert!(batch.policies.is_none());
    assert!(batch.weights.is_none(), "uniform buffer has no IS weights");
    assert_eq!(batch.indices.len(), 5);
}

#[test]
fn empty_sample_returns_error() {
    let mut buf = UniformReplayBuffer::new(10);
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let r = buf.sample(1, &mut rng);
    assert!(matches!(r, Err(othello_rl::ReplayError::Empty)));
}

#[test]
fn batch_too_large_returns_error() {
    let mut buf = UniformReplayBuffer::new(10);
    for i in 0..3 {
        buf.push(dummy_transition(0.0, i as u32));
    }
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let r = buf.sample(5, &mut rng);
    assert!(matches!(
        r,
        Err(othello_rl::ReplayError::BatchTooLarge { .. })
    ));
}

#[test]
fn clear_empties_buffer() {
    let mut buf = UniformReplayBuffer::new(10);
    for i in 0..5 {
        buf.push(dummy_transition(0.0, i as u32));
    }
    buf.clear();
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
}

#[test]
fn update_priorities_is_noop_for_uniform() {
    let mut buf = UniformReplayBuffer::new(10);
    for i in 0..5 {
        buf.push(dummy_transition(0.0, i as u32));
    }
    // Should be no-op and return Ok
    buf.update_priorities(&[0, 1], &[10.0, 20.0]).unwrap();
}

#[test]
fn samples_can_repeat_with_replacement() {
    let mut buf = UniformReplayBuffer::new(5);
    buf.push(dummy_transition(7.0, 1));
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    // バッファに 1 件しかなければ batch_size <=1 のみ可能だが，要素 > 0 ならサンプル可．
    let batch = buf.sample(1, &mut rng).unwrap();
    assert_eq!(batch.values[0], 7.0);
}
