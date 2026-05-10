//! Key event to [`Action`] mapping. Independently testable from `crossterm`.

use crossterm::event::KeyCode;

/// Action passed to the application layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Move the cursor up (Play mode).
    Up,
    /// Move the cursor down (Play mode).
    Down,
    /// Move the cursor left (Play mode).
    Left,
    /// Move the cursor right (Play mode).
    Right,
    /// Place a stone (Play mode).
    Place,
    /// Pass (Play mode).
    Pass,
    /// Step one move forward (Replay mode).
    StepForward,
    /// Step one move backward (Replay mode).
    StepBackward,
    /// Jump to the beginning (Replay mode).
    JumpStart,
    /// Jump to the end (Replay mode).
    JumpEnd,
    /// Toggle auto-play (Replay mode).
    ToggleAutoPlay,
    /// Increase the auto-play interval (Observe mode).
    IncreaseDelay,
    /// Decrease the auto-play interval (Observe mode).
    DecreaseDelay,
    /// Quit.
    Quit,
}

/// Key to action mapping for Play mode.
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

/// Key to action mapping for Replay mode.
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

/// Key to action mapping for Observe mode.
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
