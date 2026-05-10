//! Conversion from `GameRecord` to a sequence of `Transition`s.
//!
//! Converts a [`GameRecord`] produced by self-play (or restored from
//! JSON / GGF / WTHOR) into a list of [`Transition`]s for training. Each
//! transition's `value` is the terminal result from the player's own
//! viewpoint (+1 for the winner, -1 for the loser, 0 for a draw). No
//! policy target is attached (`None`).

use ndarray::Array1;
use othello_core::{Color, GameState, Move};
use othello_io::GameRecord;

use crate::action_space::Action;
use crate::observation::{Observation, ObservationType, make_observation};
use crate::replay_buffer::transition::Transition;

use super::ReplayError;

/// Converts a `GameRecord` into a sequence of `Transition`s from a single
/// viewpoint.
///
/// Generates one transition per move played by the side specified by
/// `view`. Each transition's `value` is set to the final result from the
/// player's viewpoint (+1 win, 0 draw, -1 loss).
///
/// Invalid records (rule-violating moves) are rejected via core-layer
/// errors.
pub fn transitions_from_record(
    record: &GameRecord,
    view: Color,
) -> Result<Vec<Transition>, ReplayError> {
    let size = record.metadata.board_size;
    let mut state = GameState::standard(size)
        .map_err(|e| ReplayError::InvalidRecord(format!("invalid board size: {e}")))?;
    let mut history: Vec<Move> = Vec::new();
    let mut transitions: Vec<Transition> = Vec::new();
    let game_id = record.metadata.id.clone();

    for entry in &record.moves {
        if state.is_terminal() {
            break;
        }
        if state.side_to_move == view {
            let obs_arr = match make_observation(&state, view, ObservationType::Planes, &history) {
                Observation::Planes(a) => a,
                _ => unreachable!("Planes was requested"),
            };
            let mask = legal_mask_for(&state, view, size);
            let action = Action::from_move(entry.r#move, size).0;
            transitions.push(Transition {
                observation: obs_arr,
                action,
                policy: None,
                value: 0.0, // 終局後にまとめて埋める
                legal_mask: mask,
                side: view,
                move_number: state.move_number,
                game_id: game_id.clone(),
            });
        }
        // 着手側と棋譜側が一致しているか軽く確認．
        if entry.side != state.side_to_move {
            return Err(ReplayError::InvalidRecord(format!(
                "side mismatch at move {}: record={:?}, state={:?}",
                entry.n, entry.side, state.side_to_move
            )));
        }
        state.apply_move(entry.r#move).map_err(|e| {
            ReplayError::InvalidRecord(format!("apply_move failed at move {}: {e}", entry.n))
        })?;
        history.push(entry.r#move);
    }

    // value を埋める ( 自分視点の終局結果)
    let value = view_value(&state, view, record);
    for t in &mut transitions {
        t.value = value;
    }
    Ok(transitions)
}

/// Returns transitions from both Black and White viewpoints (the typical
/// case for self-play training).
///
/// The result is the Black-viewpoint and White-viewpoint vectors
/// concatenated in that order; the move order is preserved.
pub fn transitions_from_record_both_sides(
    record: &GameRecord,
) -> Result<Vec<Transition>, ReplayError> {
    let mut out = transitions_from_record(record, Color::Black)?;
    out.extend(transitions_from_record(record, Color::White)?);
    Ok(out)
}

fn legal_mask_for(state: &GameState, view: Color, size: othello_core::BoardSize) -> Array1<bool> {
    let len = Action::space_size(size) as usize;
    let mut mask = Array1::from_elem((len,), false);
    if state.side_to_move != view {
        return mask;
    }
    let legal = state.legal_moves();
    if legal.is_empty() {
        mask[len - 1] = true;
    } else {
        for mv in legal {
            let a = Action::from_move(mv, size);
            mask[a.0 as usize] = true;
        }
    }
    mask
}

/// Computes the terminal reward from the `view` perspective. Prefers the
/// record's final result; falls back to the on-board stone counts when no
/// result is recorded.
fn view_value(state: &GameState, view: Color, record: &GameRecord) -> f32 {
    if let Some(result) = record.metadata.result.as_ref() {
        return match result.winner {
            Some(c) if c == view => 1.0,
            Some(_) => -1.0,
            None => 0.0,
        };
    }
    if state.is_terminal() {
        let own = state.board.count(view);
        let opp = state.board.count(view.opponent());
        match own.cmp(&opp) {
            std::cmp::Ordering::Greater => 1.0,
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Equal => 0.0,
        }
    } else {
        // 進行中の棋譜なら value は 0 として扱う ( 学習では終局棋譜のみ使う想定)
        0.0
    }
}
