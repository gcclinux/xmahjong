//! LMahjong - A Tux-themed Mahjong solitaire game for Linux.
//!
//! Main entry point: initializes SDL2, creates all game components,
//! and runs the game loop at ~60 FPS.

use std::time::{Duration, Instant};

use sdl2::rect::Rect;

use lmahjong::audio::AudioManager;
use lmahjong::board::turtle_layout;
use lmahjong::game_state::{GameState, GameStatus, NameEntryState, ScoreTracker};
use lmahjong::generator::BoardGenerator;
use lmahjong::input::{GameAction, InputHandler};
use lmahjong::logic::{self, GameOverReason, HintResult, SelectionResult};
use lmahjong::renderer::{self, Renderer};
use lmahjong::storage::{Leaderboard, LeaderboardEntry, SavedGame, Settings};
use lmahjong::timer::GameTimer;

/// Target frame duration for ~60 FPS (16.67ms per frame).
const FRAME_DURATION_MS: u64 = 16;

/// Duration in seconds before auto-dismissing hints.
const HINT_DISMISS_SECS: u64 = 3;

fn main() {
    // 1. Initialize SDL2 context
    let sdl_context = sdl2::init().expect("Failed to initialize SDL2");

    // 2. Create Renderer (handles window, canvas, textures)
    let mut renderer = Renderer::new(&sdl_context).expect("Failed to create renderer");

    // Enable SDL2 text input (needed for TextInput events during name entry)
    {
        let video = sdl_context.video().expect("Failed to get video subsystem for text input");
        video.text_input().start();
    }

    // 3. Create AudioManager (handles init failure gracefully)
    let mut audio = AudioManager::new();

    // 4. Load Settings (mute state)
    let mut settings = Settings::load();
    audio.set_mute(settings.muted);

    // 5. Create InputHandler
    let input_handler = InputHandler::new();

    // 6. Generate initial board and create GameState
    let mut game_state = if SavedGame::exists() {
        match load_saved_game() {
            Some(state) => {
                SavedGame::delete();
                state
            }
            None => create_new_game_state(),
        }
    } else {
        create_new_game_state()
    };

    // Start the timer immediately for the first game
    game_state.timer.start();

    // Track whether we're showing a quit confirmation
    let mut quit_confirmation = false;

    // Track name entry state for leaderboard (active after winning with a qualifying score)
    let mut name_entry: Option<NameEntryState> = None;

    // SDL2 event pump
    let mut event_pump = sdl_context
        .event_pump()
        .expect("Failed to create SDL2 event pump");

    // 7. Main game loop
    'game_loop: loop {
        let frame_start = Instant::now();

        // --- 7a. Poll SDL2 events and process through InputHandler ---
        for event in event_pump.poll_iter() {
            // --- Handle name entry state input first ---
            if game_state.status == GameStatus::NameEntry {
                if let Some(ref mut entry) = name_entry {
                    match &event {
                        sdl2::event::Event::TextInput { text, .. } => {
                            for c in text.chars() {
                                entry.push_char(c);
                            }
                            continue;
                        }
                        sdl2::event::Event::KeyDown {
                            keycode: Some(keycode),
                            ..
                        } => {
                            match *keycode {
                                sdl2::keyboard::Keycode::Return
                                | sdl2::keyboard::Keycode::KpEnter => {
                                    // Submit the name if valid
                                    if entry.is_valid() {
                                        let date = current_date_string();
                                        let lb_entry = LeaderboardEntry {
                                            name: entry.text.clone(),
                                            score: entry.score,
                                            time_seconds: entry.time_seconds,
                                            date,
                                        };
                                        let mut leaderboard = Leaderboard::load();
                                        leaderboard.insert(lb_entry);
                                        leaderboard.save();
                                        // Transition back to Won state
                                        name_entry = None;
                                        game_state.status = GameStatus::Won;
                                    }
                                    // If not valid (empty), ignore the Enter press
                                    continue;
                                }
                                sdl2::keyboard::Keycode::Backspace => {
                                    entry.pop_char();
                                    continue;
                                }
                                sdl2::keyboard::Keycode::Escape => {
                                    // Cancel name entry, go back to victory screen
                                    name_entry = None;
                                    game_state.status = GameStatus::Won;
                                    continue;
                                }
                                _ => {
                                    continue;
                                }
                            }
                        }
                        sdl2::event::Event::Quit { .. } => {
                            break 'game_loop;
                        }
                        _ => {
                            continue;
                        }
                    }
                }
            }

            let is_paused = matches!(
                game_state.status,
                GameStatus::Paused | GameStatus::Menu
            );

            if let Some(action) = input_handler.process_event(&event, is_paused) {
                // If quit confirmation is showing, handle specially
                if quit_confirmation {
                    match action {
                        GameAction::Quit => break 'game_loop,
                        GameAction::SelectTile(x, y) => {
                            // Check if click hit YES or NO button
                            let (win_w, win_h) = renderer.window_size();
                            let dialog_w: i32 = 300;
                            let dialog_h: i32 = 180;
                            let dialog_x = (win_w as i32 - dialog_w) / 2;
                            let dialog_y = (win_h as i32 - dialog_h) / 2;

                            let btn_w: i32 = 120;
                            let btn_h: i32 = 40;
                            let btn_spacing: i32 = 20;
                            let total_btn_width = btn_w * 2 + btn_spacing;
                            let btn_start_x = dialog_x + (dialog_w - total_btn_width) / 2;
                            let btn_y = dialog_y + 110;

                            // YES button area
                            if x >= btn_start_x && x < btn_start_x + btn_w
                                && y >= btn_y && y < btn_y + btn_h
                            {
                                break 'game_loop;
                            }

                            // NO button area
                            let no_x = btn_start_x + btn_w + btn_spacing;
                            if x >= no_x && x < no_x + btn_w
                                && y >= btn_y && y < btn_y + btn_h
                            {
                                quit_confirmation = false;
                            }
                            // Click outside buttons: do nothing (dialog stays)
                            continue;
                        }
                        _ => {
                            // Escape or any key dismisses confirmation
                            quit_confirmation = false;
                            continue;
                        }
                    }
                }

                // --- 7b. Process GameAction based on current GameStatus ---
                match action {
                    GameAction::SelectTile(x, y) => {
                        if game_state.status == GameStatus::Playing {
                            let won = handle_select_tile(
                                &mut game_state,
                                &mut audio,
                                &renderer,
                                x,
                                y,
                            );
                            if won {
                                // Check if score qualifies for leaderboard
                                let score = game_state.score.calculate_score();
                                let leaderboard = Leaderboard::load();
                                if leaderboard.qualifies(score) {
                                    let time_seconds = game_state.score.elapsed_seconds;
                                    name_entry = Some(NameEntryState::new(score, time_seconds));
                                    game_state.status = GameStatus::NameEntry;
                                }
                            }
                        } else if game_state.status == GameStatus::Paused {
                            // Handle clicks on pause menu buttons
                            let (win_w, win_h) = renderer.window_size();
                            let dialog_w: u32 = 300;
                            let dialog_h: u32 = 440;
                            let dialog_x = (win_w.saturating_sub(dialog_w)) / 2;
                            let dialog_y = (win_h.saturating_sub(dialog_h)) / 2;

                            let btn_w: u32 = 220;
                            let btn_h: u32 = 40;
                            let btn_x = dialog_x as i32 + ((dialog_w - btn_w) / 2) as i32;
                            let start_y = dialog_y as i32 + 60;
                            let spacing: i32 = 50;

                            // Check which button was clicked
                            if x >= btn_x && x < btn_x + btn_w as i32 {
                                if y >= start_y && y < start_y + btn_h as i32 {
                                    // RESUME
                                    game_state.status = GameStatus::Playing;
                                    game_state.timer.resume();
                                } else if y >= start_y + spacing && y < start_y + spacing + btn_h as i32 {
                                    // NEW GAME
                                    game_state = create_new_game_state();
                                    game_state.timer.start();
                                } else if y >= start_y + spacing * 2 && y < start_y + spacing * 2 + btn_h as i32 {
                                    // UNDO
                                    game_state.status = GameStatus::Playing;
                                    game_state.timer.resume();
                                    let _ = logic::undo(&mut game_state);
                                } else if y >= start_y + spacing * 3 && y < start_y + spacing * 3 + btn_h as i32 {
                                    // HINT
                                    game_state.status = GameStatus::Playing;
                                    game_state.timer.resume();
                                    let result = logic::request_hint(&mut game_state);
                                    if let HintResult::NoMatchesAvailable = result {
                                        game_state.status = GameStatus::Lost;
                                    }
                                } else if y >= start_y + spacing * 4 && y < start_y + spacing * 4 + btn_h as i32 {
                                    // SHUFFLE
                                    game_state.status = GameStatus::Playing;
                                    game_state.timer.resume();
                                    if logic::shuffle(&mut game_state).is_ok() {
                                        audio.play_shuffle();
                                    }
                                } else if y >= start_y + spacing * 5 && y < start_y + spacing * 5 + btn_h as i32 {
                                    // SAVE + QUIT
                                    save_current_game(&game_state);
                                    break 'game_loop;
                                } else if y >= start_y + spacing * 6 && y < start_y + spacing * 6 + btn_h as i32 {
                                    // QUIT (without saving)
                                    break 'game_loop;
                                }
                            }
                        } else if game_state.status == GameStatus::Lost {
                            // Handle clicks on the "No Moves" dialog buttons
                            let (win_w, win_h) = renderer.window_size();
                            let dialog_w: u32 = 320;
                            let dialog_h: u32 = 220;
                            let dialog_x = (win_w.saturating_sub(dialog_w)) / 2;
                            let dialog_y = (win_h.saturating_sub(dialog_h)) / 2;

                            let btn_w: u32 = 200;
                            let btn_h: u32 = 44;
                            let btn_x = dialog_x as i32 + ((dialog_w - btn_w) / 2) as i32;

                            // Shuffle button: y offset 110 from dialog top
                            let shuffle_y = dialog_y as i32 + 110;
                            if x >= btn_x && x < btn_x + btn_w as i32
                                && y >= shuffle_y && y < shuffle_y + btn_h as i32
                            {
                                // Perform shuffle and return to playing
                                game_state.status = GameStatus::Playing;
                                if logic::shuffle(&mut game_state).is_ok() {
                                    audio.play_shuffle();
                                }
                            }

                            // New Game button: y offset 164 from dialog top
                            let new_game_y = dialog_y as i32 + 164;
                            if x >= btn_x && x < btn_x + btn_w as i32
                                && y >= new_game_y && y < new_game_y + btn_h as i32
                            {
                                game_state = create_new_game_state();
                                game_state.timer.start();
                            }
                        }
                    }

                    GameAction::NewGame => {
                        game_state = create_new_game_state();
                        game_state.timer.start();
                        quit_confirmation = false;
                    }

                    GameAction::Undo => {
                        if game_state.status == GameStatus::Playing {
                            let _ = logic::undo(&mut game_state);
                        }
                    }

                    GameAction::Hint => {
                        if game_state.status == GameStatus::Playing {
                            let result = logic::request_hint(&mut game_state);
                            if let HintResult::NoMatchesAvailable = result {
                                // No moves available — transition to Lost
                                game_state.status = GameStatus::Lost;
                            }
                        }
                    }

                    GameAction::Shuffle => {
                        if game_state.status == GameStatus::Playing
                            || game_state.status == GameStatus::Lost
                        {
                            if game_state.status == GameStatus::Lost {
                                game_state.status = GameStatus::Playing;
                            }
                            if logic::shuffle(&mut game_state).is_ok() {
                                audio.play_shuffle();
                            }
                        }
                    }

                    GameAction::PauseMenu => {
                        if game_state.status == GameStatus::Playing {
                            game_state.status = GameStatus::Paused;
                            game_state.timer.pause();
                        }
                    }

                    GameAction::Resume => {
                        if game_state.status == GameStatus::Paused {
                            game_state.status = GameStatus::Playing;
                            game_state.timer.resume();
                        }
                    }

                    GameAction::Save => {
                        if game_state.status == GameStatus::Playing
                            || game_state.status == GameStatus::Paused
                        {
                            save_current_game(&game_state);
                        }
                    }

                    GameAction::SaveQuit => {
                        if game_state.status == GameStatus::Playing
                            || game_state.status == GameStatus::Paused
                        {
                            save_current_game(&game_state);
                            break 'game_loop;
                        } else {
                            break 'game_loop;
                        }
                    }

                    GameAction::ToggleMute => {
                        audio.toggle_mute();
                        settings.muted = audio.is_muted();
                        settings.save();
                    }

                    GameAction::ToggleFullscreen => {
                        toggle_fullscreen(&mut renderer);
                    }

                    GameAction::Quit => {
                        if game_state.status == GameStatus::Playing {
                            // Show confirmation when actively playing
                            quit_confirmation = true;
                        } else {
                            break 'game_loop;
                        }
                    }
                }
            }
        }

        // --- 7c. Update timer ---
        if game_state.status == GameStatus::Playing {
            game_state.timer.update();
        }

        // --- 7d. Expire completed animations ---
        expire_animations(&mut game_state);

        // --- 7e. Auto-dismiss hint after 3 seconds ---
        if let Some(ref hint) = game_state.hint {
            if hint.activated_at.elapsed() >= Duration::from_secs(HINT_DISMISS_SECS) {
                game_state.hint = None;
            }
        }

        // --- 7f. Render based on GameStatus ---
        renderer.clear();

        let (win_w, win_h) = renderer.window_size();
        let metrics = renderer::compute_layout_rect(win_w, win_h);
        let layout_rect = Rect::new(
            metrics.offset_x as i32,
            metrics.offset_y as i32,
            metrics.layout_w as u32,
            metrics.layout_h as u32,
        );

        match game_state.status {
            GameStatus::Playing => {
                renderer.render_board(&game_state, layout_rect);
                renderer.render_hud(&game_state);
            }
            GameStatus::Paused => {
                renderer.render_board(&game_state, layout_rect);
                renderer.render_hud(&game_state);
                renderer.render_menu();
            }
            GameStatus::Won => {
                renderer.render_board(&game_state, layout_rect);
                let time_str = game_state.timer.format_display();
                let score = game_state.score.calculate_score();
                renderer.render_victory(&time_str, score);
            }
            GameStatus::Lost => {
                renderer.render_board(&game_state, layout_rect);
                renderer.render_no_moves();
            }
            GameStatus::Menu => {
                renderer.render_menu();
            }
            GameStatus::NameEntry => {
                renderer.render_board(&game_state, layout_rect);
                if let Some(ref entry) = name_entry {
                    renderer.render_name_entry(&entry.text, entry.score, entry.time_seconds);
                }
            }
        }

        if quit_confirmation {
            renderer.render_quit_confirmation();
        }

        // --- 7g. Present frame ---
        renderer.present();

        // --- 7h. Cap to ~60 FPS using SDL2 delay ---
        let frame_elapsed = frame_start.elapsed();
        let target_duration = Duration::from_millis(FRAME_DURATION_MS);
        if frame_elapsed < target_duration {
            std::thread::sleep(target_duration - frame_elapsed);
        }
    }
}

/// Handles a tile selection click at screen coordinates (x, y).
/// Returns `true` if the game was just won (so the caller can check leaderboard).
fn handle_select_tile(
    state: &mut GameState,
    audio: &mut AudioManager,
    renderer: &Renderer,
    x: i32,
    y: i32,
) -> bool {
    // Hit-test to find which tile was clicked
    let (win_w, win_h) = renderer.window_size();
    let metrics = renderer::compute_layout_rect(win_w, win_h);

    let pos = match renderer::hit_test(x, y, state, &metrics) {
        Some(idx) => idx,
        None => return false, // Click didn't hit any tile
    };

    // Process tile selection through game logic
    let result = logic::select_tile(state, pos);

    match result {
        SelectionResult::Matched(_, _) => {
            audio.play_match();

            // Check for game over after a successful match
            if let Some(reason) = logic::check_game_over(state) {
                match reason {
                    GameOverReason::Won => {
                        state.timer.pause();
                        state.score.elapsed_seconds = state.timer.elapsed_seconds();
                        state.status = GameStatus::Won;
                        audio.play_victory();
                        return true;
                    }
                    GameOverReason::Lost => {
                        state.status = GameStatus::Lost;
                    }
                }
            }
        }
        SelectionResult::Mismatched(_, _) => {
            audio.play_error();
        }
        SelectionResult::Selected
        | SelectionResult::Deselected
        | SelectionResult::Ignored => {}
    }

    false
}

/// Creates a new GameState with a freshly generated board.
fn create_new_game_state() -> GameState {
    let layout = turtle_layout();
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let mut generator = BoardGenerator::new(seed);
    let board = generator
        .generate(layout, 5)
        .expect("Failed to generate board after 5 attempts");

    GameState {
        board,
        timer: GameTimer::new(),
        score: ScoreTracker::new(),
        status: GameStatus::Playing,
        selection: None,
        hint: None,
        undo_stack: Vec::new(),
        shuffles_remaining: 3,
        animations: Vec::new(),
    }
}

/// Removes completed animations from the game state.
fn expire_animations(state: &mut GameState) {
    let now = Instant::now();
    state.animations.retain(|anim| {
        match anim {
            lmahjong::game_state::Animation::TileRemoval {
                start_time,
                duration_ms,
                ..
            } => {
                let elapsed = now.duration_since(*start_time).as_millis() as u32;
                elapsed < *duration_ms
            }
            lmahjong::game_state::Animation::TileMismatch {
                start_time,
                duration_ms,
                ..
            } => {
                let elapsed = now.duration_since(*start_time).as_millis() as u32;
                elapsed < *duration_ms
            }
            lmahjong::game_state::Animation::HintPulse { start_time, .. } => {
                // Keep hint pulse active while hint is displayed (managed by hint auto-dismiss)
                let elapsed = now.duration_since(*start_time).as_secs();
                elapsed < HINT_DISMISS_SECS
            }
            lmahjong::game_state::Animation::Shuffle {
                start_time,
                duration_ms,
                ..
            } => {
                let elapsed = now.duration_since(*start_time).as_millis() as u32;
                elapsed < *duration_ms
            }
        }
    });
}

/// Toggles fullscreen mode on the renderer's window.
/// Saves the window size and position before entering fullscreen,
/// and restores them when returning to windowed mode.
fn toggle_fullscreen(renderer: &mut Renderer) {
    use std::cell::Cell;
    use sdl2::video::FullscreenType;

    // Thread-local storage for windowed geometry (x, y, width, height).
    // Safe because the game loop is single-threaded.
    thread_local! {
        static WINDOWED_GEOMETRY: Cell<Option<(i32, i32, u32, u32)>> = const { Cell::new(None) };
    }

    let window = renderer.canvas.window_mut();
    let current = window.fullscreen_state();

    let new_state = match current {
        FullscreenType::Off => {
            // Save current windowed geometry before going fullscreen
            let (x, y) = window.position();
            let (w, h) = window.size();
            WINDOWED_GEOMETRY.with(|geo| geo.set(Some((x, y, w, h))));
            FullscreenType::Desktop
        }
        _ => FullscreenType::Off,
    };

    if let Err(e) = window.set_fullscreen(new_state) {
        eprintln!("[LMahjong] Warning: Failed to toggle fullscreen: {}", e);
        // Graceful fallback: remain in current mode, no crash
        return;
    }

    // If we just exited fullscreen, restore previous window size and position
    if new_state == FullscreenType::Off {
        let saved = WINDOWED_GEOMETRY.with(|geo| geo.take());
        if let Some((x, y, w, h)) = saved {
            let window = renderer.canvas.window_mut();
            window.set_size(w, h).ok();
            window.set_position(
                sdl2::video::WindowPos::Positioned(x),
                sdl2::video::WindowPos::Positioned(y),
            );
        }
    }
}

/// Returns the current date as an ISO 8601 date string (YYYY-MM-DD).
fn current_date_string() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple date calculation (no external crate needed)
    let days = now / 86400;
    let (year, month, day) = days_to_date(days);
    format!("{:04}-{:02}-{:02}", year, month, day)
}

/// Converts days since Unix epoch to (year, month, day).
fn days_to_date(days_since_epoch: u64) -> (u32, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days_since_epoch + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32)
}

/// Saves the current game state to disk so it can be resumed later.
fn save_current_game(state: &GameState) {
    let tiles: Vec<Option<u8>> = state.board.tiles.iter().map(|t| t.map(|tile| tile.face_id)).collect();

    let undo_stack: Vec<(usize, u8, usize, u8)> = state.undo_stack.iter().map(|entry| {
        (entry.position_a, entry.tile_a.face_id, entry.position_b, entry.tile_b.face_id)
    }).collect();

    let saved = SavedGame {
        tiles,
        undo_stack,
        elapsed_ms: state.timer.elapsed_ms
            + state.timer.last_tick.map(|t| t.elapsed().as_millis() as u64).unwrap_or(0),
        hints_used: state.score.hints_used,
        shuffles_used: state.score.shuffles_used,
        shuffles_remaining: state.shuffles_remaining,
        pairs_matched: state.score.pairs_matched,
    };

    saved.save();
}

/// Loads a saved game from disk and reconstructs the GameState.
/// Returns None if loading fails.
fn load_saved_game() -> Option<GameState> {
    use lmahjong::board::{Board, Tile, turtle_layout};
    use lmahjong::logic::UndoEntry;

    let saved = SavedGame::load()?;
    let layout = turtle_layout();

    // Validate tile count matches layout
    if saved.tiles.len() != layout.positions.len() {
        eprintln!("[LMahjong] Save file has wrong tile count, ignoring.");
        return None;
    }

    // Reconstruct board
    let mut board = Board::new(layout);
    for (idx, maybe_face) in saved.tiles.iter().enumerate() {
        if let Some(face_id) = maybe_face {
            board.tiles[idx] = Some(Tile {
                face_id: *face_id,
                position: layout.positions[idx],
            });
        }
    }

    // Reconstruct undo stack
    let undo_stack: Vec<UndoEntry> = saved.undo_stack.iter().map(|&(pos_a, face_a, pos_b, face_b)| {
        UndoEntry {
            tile_a: Tile { face_id: face_a, position: layout.positions[pos_a] },
            tile_b: Tile { face_id: face_b, position: layout.positions[pos_b] },
            position_a: pos_a,
            position_b: pos_b,
        }
    }).collect();

    // Reconstruct timer (paused state, will be resumed on start)
    let mut timer = GameTimer::new();
    timer.elapsed_ms = saved.elapsed_ms;
    // Timer is stopped; it will resume when game loop starts

    let state = GameState {
        board,
        timer,
        score: ScoreTracker {
            hints_used: saved.hints_used,
            shuffles_used: saved.shuffles_used,
            elapsed_seconds: 0, // Only used at game end
            pairs_matched: saved.pairs_matched,
        },
        status: GameStatus::Playing,
        selection: None,
        hint: None,
        undo_stack,
        shuffles_remaining: saved.shuffles_remaining,
        animations: Vec::new(),
    };

    Some(state)
}
