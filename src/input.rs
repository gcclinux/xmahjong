//! Input handler module.
//!
//! Processes SDL2 events and maps them to game actions,
//! including mouse clicks and keyboard shortcuts.

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::MouseButton;

/// Actions that the player can trigger via input events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameAction {
    /// Player clicked at screen coordinates (x, y) to select a tile.
    SelectTile(i32, i32),
    /// Start a new game (Ctrl+N).
    NewGame,
    /// Undo last move (Ctrl+Z).
    Undo,
    /// Request a hint (Ctrl+H).
    Hint,
    /// Shuffle remaining tiles (Ctrl+S).
    Shuffle,
    /// Toggle audio mute (Ctrl+M).
    ToggleMute,
    /// Toggle fullscreen mode (F11).
    ToggleFullscreen,
    /// Open pause/game menu (Escape).
    PauseMenu,
    /// Resume game from menu.
    Resume,
    /// Quit the game.
    Quit,
}

/// Processes SDL2 events and translates them into game actions.
pub struct InputHandler;

impl InputHandler {
    /// Creates a new input handler.
    pub fn new() -> Self {
        InputHandler
    }

    /// Processes an SDL2 event and returns the corresponding game action, if any.
    ///
    /// The `is_paused` flag determines whether Escape maps to `Resume` (when paused)
    /// or `PauseMenu` (when playing).
    pub fn process_event(&self, event: &Event, is_paused: bool) -> Option<GameAction> {
        match event {
            Event::Quit { .. } => Some(GameAction::Quit),

            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } => Some(GameAction::SelectTile(*x, *y)),

            Event::KeyDown {
                keycode: Some(keycode),
                keymod,
                ..
            } => {
                let ctrl =
                    keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD);

                match (ctrl, *keycode) {
                    (true, Keycode::N) => Some(GameAction::NewGame),
                    (true, Keycode::Z) => Some(GameAction::Undo),
                    (true, Keycode::H) => Some(GameAction::Hint),
                    (true, Keycode::S) => Some(GameAction::Shuffle),
                    (true, Keycode::M) => Some(GameAction::ToggleMute),
                    (false, Keycode::F11) => Some(GameAction::ToggleFullscreen),
                    (false, Keycode::Escape) => {
                        if is_paused {
                            Some(GameAction::Resume)
                        } else {
                            Some(GameAction::PauseMenu)
                        }
                    }
                    _ => None,
                }
            }

            _ => None,
        }
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2::event::Event;
    use sdl2::keyboard::{Keycode, Mod};
    use sdl2::mouse::MouseButton;

    fn handler() -> InputHandler {
        InputHandler::new()
    }

    #[test]
    fn quit_event_maps_to_quit() {
        let h = handler();
        let event = Event::Quit { timestamp: 0 };
        assert_eq!(h.process_event(&event, false), Some(GameAction::Quit));
    }

    #[test]
    fn left_click_maps_to_select_tile() {
        let h = handler();
        let event = Event::MouseButtonDown {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x: 150,
            y: 300,
        };
        assert_eq!(
            h.process_event(&event, false),
            Some(GameAction::SelectTile(150, 300))
        );
    }

    #[test]
    fn right_click_is_ignored() {
        let h = handler();
        let event = Event::MouseButtonDown {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Right,
            clicks: 1,
            x: 100,
            y: 200,
        };
        assert_eq!(h.process_event(&event, false), None);
    }

    #[test]
    fn ctrl_n_maps_to_new_game() {
        let h = handler();
        let event = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::N),
            scancode: None,
            keymod: Mod::LCTRLMOD,
            repeat: false,
        };
        assert_eq!(h.process_event(&event, false), Some(GameAction::NewGame));
    }

    #[test]
    fn ctrl_z_maps_to_undo() {
        let h = handler();
        let event = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::Z),
            scancode: None,
            keymod: Mod::LCTRLMOD,
            repeat: false,
        };
        assert_eq!(h.process_event(&event, false), Some(GameAction::Undo));
    }

    #[test]
    fn ctrl_h_maps_to_hint() {
        let h = handler();
        let event = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::H),
            scancode: None,
            keymod: Mod::LCTRLMOD,
            repeat: false,
        };
        assert_eq!(h.process_event(&event, false), Some(GameAction::Hint));
    }

    #[test]
    fn ctrl_s_maps_to_shuffle() {
        let h = handler();
        let event = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::S),
            scancode: None,
            keymod: Mod::LCTRLMOD,
            repeat: false,
        };
        assert_eq!(h.process_event(&event, false), Some(GameAction::Shuffle));
    }

    #[test]
    fn ctrl_m_maps_to_toggle_mute() {
        let h = handler();
        let event = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::M),
            scancode: None,
            keymod: Mod::LCTRLMOD,
            repeat: false,
        };
        assert_eq!(h.process_event(&event, false), Some(GameAction::ToggleMute));
    }

    #[test]
    fn right_ctrl_also_works() {
        let h = handler();
        let event = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::N),
            scancode: None,
            keymod: Mod::RCTRLMOD,
            repeat: false,
        };
        assert_eq!(h.process_event(&event, false), Some(GameAction::NewGame));
    }

    #[test]
    fn f11_maps_to_toggle_fullscreen() {
        let h = handler();
        let event = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::F11),
            scancode: None,
            keymod: Mod::NOMOD,
            repeat: false,
        };
        assert_eq!(
            h.process_event(&event, false),
            Some(GameAction::ToggleFullscreen)
        );
    }

    #[test]
    fn escape_when_playing_maps_to_pause_menu() {
        let h = handler();
        let event = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::Escape),
            scancode: None,
            keymod: Mod::NOMOD,
            repeat: false,
        };
        assert_eq!(
            h.process_event(&event, false),
            Some(GameAction::PauseMenu)
        );
    }

    #[test]
    fn escape_when_paused_maps_to_resume() {
        let h = handler();
        let event = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::Escape),
            scancode: None,
            keymod: Mod::NOMOD,
            repeat: false,
        };
        assert_eq!(h.process_event(&event, true), Some(GameAction::Resume));
    }

    #[test]
    fn unbound_key_returns_none() {
        let h = handler();
        let event = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::A),
            scancode: None,
            keymod: Mod::NOMOD,
            repeat: false,
        };
        assert_eq!(h.process_event(&event, false), None);
    }

    #[test]
    fn n_without_ctrl_returns_none() {
        let h = handler();
        let event = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::N),
            scancode: None,
            keymod: Mod::NOMOD,
            repeat: false,
        };
        assert_eq!(h.process_event(&event, false), None);
    }

    #[test]
    fn f11_with_ctrl_returns_none() {
        let h = handler();
        // F11 should only work without Ctrl
        let event = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::F11),
            scancode: None,
            keymod: Mod::LCTRLMOD,
            repeat: false,
        };
        assert_eq!(h.process_event(&event, false), None);
    }

    #[test]
    fn mouse_motion_is_ignored() {
        let h = handler();
        let event = Event::MouseMotion {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mousestate: sdl2::mouse::MouseState::from_sdl_state(0),
            x: 50,
            y: 50,
            xrel: 1,
            yrel: 1,
        };
        assert_eq!(h.process_event(&event, false), None);
    }
}
