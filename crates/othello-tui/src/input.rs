//! キーイベント → [`Action`] のマッピング．`crossterm` から独立にテストできる．

use crossterm::event::KeyCode;

/// アプリケーション側に渡るアクション．
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// カーソルを上に移動 ( Play モード)．
    Up,
    /// カーソルを下に移動 ( Play モード)．
    Down,
    /// カーソルを左に移動 ( Play モード)．
    Left,
    /// カーソルを右に移動 ( Play モード)．
    Right,
    /// 着手 ( Play モード)．
    Place,
    /// パス ( Play モード)．
    Pass,
    /// 1 手進める ( Replay モード)．
    StepForward,
    /// 1 手戻る ( Replay モード)．
    StepBackward,
    /// 最初に戻る ( Replay モード)．
    JumpStart,
    /// 最後に飛ぶ ( Replay モード)．
    JumpEnd,
    /// 自動再生を切替 ( Replay モード)．
    ToggleAutoPlay,
    /// 自動再生間隔を増やす ( Observe モード)．
    IncreaseDelay,
    /// 自動再生間隔を減らす ( Observe モード)．
    DecreaseDelay,
    /// 終了．
    Quit,
}

/// Play モード用のキー → Action 変換．
#[must_use]
pub fn map_play_key(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::Down),
        KeyCode::Left | KeyCode::Char('h') => Some(Action::Left),
        KeyCode::Right | KeyCode::Char('l') => Some(Action::Right),
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Place),
        KeyCode::Char('p') => Some(Action::Pass),
        _ => None,
    }
}

/// Replay モード用のキー → Action 変換．
#[must_use]
pub fn map_replay_key(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        KeyCode::Right | KeyCode::Char('l') => Some(Action::StepForward),
        KeyCode::Left | KeyCode::Char('h') => Some(Action::StepBackward),
        KeyCode::Char('0') => Some(Action::JumpStart),
        KeyCode::Char('$') => Some(Action::JumpEnd),
        KeyCode::Char(' ') | KeyCode::Char('a') => Some(Action::ToggleAutoPlay),
        KeyCode::Char('+') | KeyCode::Char('=') => Some(Action::IncreaseDelay),
        KeyCode::Char('-') | KeyCode::Char('_') => Some(Action::DecreaseDelay),
        _ => None,
    }
}

/// Observe モード用のキー → Action 変換．
#[must_use]
pub fn map_observe_key(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        KeyCode::Char(' ') => Some(Action::StepForward),
        KeyCode::Char('a') => Some(Action::ToggleAutoPlay),
        KeyCode::Char('+') | KeyCode::Char('=') => Some(Action::IncreaseDelay),
        KeyCode::Char('-') | KeyCode::Char('_') => Some(Action::DecreaseDelay),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_arrow_mapping() {
        assert_eq!(map_play_key(KeyCode::Up), Some(Action::Up));
        assert_eq!(map_play_key(KeyCode::Char('k')), Some(Action::Up));
        assert_eq!(map_play_key(KeyCode::Char('h')), Some(Action::Left));
    }

    #[test]
    fn play_quit_mapping() {
        assert_eq!(map_play_key(KeyCode::Char('q')), Some(Action::Quit));
        assert_eq!(map_play_key(KeyCode::Esc), Some(Action::Quit));
    }

    #[test]
    fn play_place_and_pass() {
        assert_eq!(map_play_key(KeyCode::Enter), Some(Action::Place));
        assert_eq!(map_play_key(KeyCode::Char(' ')), Some(Action::Place));
        assert_eq!(map_play_key(KeyCode::Char('p')), Some(Action::Pass));
    }

    #[test]
    fn replay_navigation() {
        assert_eq!(map_replay_key(KeyCode::Right), Some(Action::StepForward));
        assert_eq!(
            map_replay_key(KeyCode::Char('l')),
            Some(Action::StepForward)
        );
        assert_eq!(map_replay_key(KeyCode::Left), Some(Action::StepBackward));
        assert_eq!(map_replay_key(KeyCode::Char('0')), Some(Action::JumpStart));
        assert_eq!(map_replay_key(KeyCode::Char('$')), Some(Action::JumpEnd));
        assert_eq!(
            map_replay_key(KeyCode::Char(' ')),
            Some(Action::ToggleAutoPlay)
        );
    }

    #[test]
    fn replay_quit_mapping() {
        assert_eq!(map_replay_key(KeyCode::Esc), Some(Action::Quit));
        assert_eq!(map_replay_key(KeyCode::Char('q')), Some(Action::Quit));
    }
}
