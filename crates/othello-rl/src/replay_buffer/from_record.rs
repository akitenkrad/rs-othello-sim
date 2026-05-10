//! GameRecord → Transition 列への変換．
//!
//! self-play で生成した棋譜 ( JSON / GGF / WTHOR から復元した [`GameRecord`])
//! を学習用の [`Transition`] 列に変換する．value は終局結果の自分視点
//! ( 勝者 +1，敗者 -1，引き分け 0)．policy ターゲットは付与しない ( `None`)．

use ndarray::Array1;
use othello_core::{Color, GameState, Move};
use othello_io::GameRecord;

use crate::action_space::Action;
use crate::observation::{Observation, ObservationType, make_observation};
use crate::replay_buffer::transition::Transition;

use super::ReplayError;

/// `GameRecord` を `Transition` 列に変換する ( 単一視点)．
///
/// `view` で指定したプレイヤーの手番ごとに 1 件 transition を生成する．
/// 生成される transition の `value` は当該プレイヤー視点の最終結果
/// ( 勝ち +1，引き分け 0，負け -1) になる．
///
/// 不正な棋譜 ( ルール違反な move) はコア層のエラーで弾かれる．
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

/// 黒・白両視点の transition を返す ( self-play 学習で典型的)．
///
/// 戻り値は黒視点・白視点を順に concat したベクタ．移動順は変えない．
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

/// `view` 視点での終局報酬を計算する．record の最終結果を優先し，
/// 無ければ盤面石数から判定する．
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
