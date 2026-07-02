//! Renderer module.
//!
//! Handles SDL2 window creation, tile rendering with depth effects,
//! UI overlays, animations, and layout scaling.

use std::time::Instant;

use sdl2::image::LoadTexture;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::ttf::Sdl2TtfContext;
use sdl2::video::{Window, WindowContext};

use crate::board::TilePosition;
use crate::game_state::{Animation, GameState};

/// Number of distinct tile face images.
const TILE_FACE_COUNT: usize = 50;

/// Default window width in pixels.
const DEFAULT_WIDTH: u32 = 1920;

/// Default window height in pixels.
const DEFAULT_HEIGHT: u32 = 1080;

/// Minimum window width in pixels.
const MIN_WIDTH: u32 = 1920;

/// Minimum window height in pixels.
const MIN_HEIGHT: u32 = 1080;

/// Asset base path (relative to executable or Snap package).
const ASSETS_PATH: &str = "assets";

/// Pixel offset per layer for depth effect (shifts left and up for higher layers).
const LAYER_OFFSET_PX: i32 = -3;

/// Shadow offset in pixels (drawn below/right of the tile).
const SHADOW_OFFSET_PX: i32 = 2;

/// Duration of tile removal animation in milliseconds.
#[allow(dead_code)]
const REMOVAL_DURATION_MS: u32 = 300;

/// Duration of tile mismatch flash animation in milliseconds.
#[allow(dead_code)]
const MISMATCH_DURATION_MS: u32 = 500;

/// Duration of shuffle animation in milliseconds.
#[allow(dead_code)]
const SHUFFLE_DURATION_MS: u32 = 500;

/// Duration of hint pulse cycle in milliseconds.
const HINT_PULSE_CYCLE_MS: u32 = 1000;

/// The natural width of the Turtle layout in grid units.
/// Max col (26) + tile width (2) = 28.
pub const LAYOUT_GRID_WIDTH: f32 = 28.0;

/// The natural height of the Turtle layout in grid units.
/// Max row (12) + tile height (2) = 14.
pub const LAYOUT_GRID_HEIGHT: f32 = 14.0;

/// Layout scaling metrics computed for a given window size.
///
/// These describe how to map tile grid coordinates to screen pixel coordinates,
/// maintaining the layout's aspect ratio and centering within the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutMetrics {
    /// Horizontal offset (in pixels) from the left edge of the window to the layout area.
    pub offset_x: f32,
    /// Vertical offset (in pixels) from the top edge of the window to the layout area.
    pub offset_y: f32,
    /// Width of one grid unit in pixels.
    pub tile_width: f32,
    /// Height of one grid unit in pixels.
    pub tile_height: f32,
    /// Total width of the layout area in pixels.
    pub layout_w: f32,
    /// Total height of the layout area in pixels.
    pub layout_h: f32,
}

/// Computes the layout rectangle that fits within the window while maintaining aspect ratio.
///
/// The layout is scaled to be as large as possible without exceeding the window bounds,
/// then centered. Margins are filled with the background.
///
/// # Arguments
/// * `window_width` - Current window width in pixels
/// * `window_height` - Current window height in pixels
///
/// # Returns
/// A `LayoutMetrics` struct containing offset, scale, and dimension information.
pub fn compute_layout_rect(window_width: u32, window_height: u32) -> LayoutMetrics {
    let aspect_ratio = LAYOUT_GRID_WIDTH / LAYOUT_GRID_HEIGHT;
    let window_aspect = window_width as f32 / window_height as f32;

    let (layout_w, layout_h) = if window_aspect > aspect_ratio {
        // Window is wider than layout — height-constrained
        let h = window_height as f32;
        let w = h * aspect_ratio;
        (w, h)
    } else {
        // Window is taller than layout — width-constrained
        let w = window_width as f32;
        let h = w / aspect_ratio;
        (w, h)
    };

    let offset_x = (window_width as f32 - layout_w) / 2.0;
    let offset_y = (window_height as f32 - layout_h) / 2.0;
    let tile_width = layout_w / LAYOUT_GRID_WIDTH;
    let tile_height = layout_h / LAYOUT_GRID_HEIGHT;

    LayoutMetrics {
        offset_x,
        offset_y,
        tile_width,
        tile_height,
        layout_w,
        layout_h,
    }
}

/// Computes the screen rectangle for a tile at the given position using the layout metrics.
///
/// Each tile occupies a 2×2 area in grid space. Higher layers are shifted slightly
/// up-left to create a depth/stacking effect (LAYER_OFFSET_PX per layer for both X and Y).
///
/// # Arguments
/// * `pos` - The tile's position in grid coordinates (layer, row, col)
/// * `metrics` - Precomputed layout scaling metrics
///
/// # Returns
/// An SDL2 `Rect` representing the tile's screen position and size.
pub fn tile_screen_rect(pos: &TilePosition, metrics: &LayoutMetrics) -> Rect {
    let layer_offset = pos.layer as i32 * LAYER_OFFSET_PX;

    let x = metrics.offset_x + (pos.col as f32 * metrics.tile_width) + layer_offset as f32;
    let y = metrics.offset_y + (pos.row as f32 * metrics.tile_height) + layer_offset as f32;

    // Each tile occupies 2 grid units in width and height
    let w = (2.0 * metrics.tile_width) as u32;
    let h = (2.0 * metrics.tile_height) as u32;

    Rect::new(x as i32, y as i32, w, h)
}

/// Given screen coordinates, finds which tile position was clicked.
///
/// Checks from the top layer (4) down to the bottom (0) because higher tiles
/// visually occlude lower ones. Only considers tiles that are currently present
/// on the board (not removed).
///
/// # Arguments
/// * `x` - Screen x coordinate (e.g., from mouse click)
/// * `y` - Screen y coordinate (e.g., from mouse click)
/// * `state` - The current game state (to check tile presence)
/// * `metrics` - Precomputed layout scaling metrics
///
/// # Returns
/// `Some(index)` of the topmost tile at the click position, or `None` if no tile was hit.
pub fn hit_test(x: i32, y: i32, state: &GameState, metrics: &LayoutMetrics) -> Option<usize> {
    // Iterate from highest layer to lowest for correct occlusion
    for layer in (0..=4u8).rev() {
        for (idx, pos) in state.board.layout.positions.iter().enumerate() {
            if pos.layer != layer {
                continue;
            }
            // Skip positions where the tile has been removed
            if state.board.tiles[idx].is_none() {
                continue;
            }
            let rect = tile_screen_rect(pos, metrics);
            if rect.contains_point((x, y)) {
                return Some(idx);
            }
        }
    }
    None
}

/// Visual highlight state for a tile being rendered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileHighlight {
    /// No special highlight.
    None,
    /// Gold border indicating the tile is selected.
    Selected,
    /// Pulsing glow for hint, with phase 0.0–1.0.
    HintGlow(f32),
    /// Red flash indicating a mismatched pair.
    MismatchFlash,
    /// Fade-out animation with alpha 0.0–1.0 (0.0 = fully transparent).
    Removing(f32),
}

/// Holds pre-rendered placeholder textures for tiles when real assets are missing.
/// Each placeholder is a colored rectangle with a unique color derived from its face ID.
pub struct PlaceholderTiles {
    /// Colors used for each face ID when textures are unavailable.
    colors: Vec<Color>,
}

impl PlaceholderTiles {
    fn new() -> Self {
        let colors: Vec<Color> = (0..TILE_FACE_COUNT)
            .map(|i| {
                // Generate distinct colors using HSV-like distribution
                let hue = (i as f32 / TILE_FACE_COUNT as f32) * 360.0;
                let (r, g, b) = hsv_to_rgb(hue, 0.7, 0.9);
                Color::RGB(r, g, b)
            })
            .collect();
        Self { colors }
    }

    /// Returns the placeholder color for a given face ID.
    fn color_for(&self, face_id: u8) -> Color {
        self.colors
            .get(face_id as usize)
            .copied()
            .unwrap_or(Color::RGB(128, 128, 128))
    }
}

/// UI texture set for buttons and overlays.
pub struct UiTextures {
    /// Whether real UI textures were loaded successfully.
    pub loaded: bool,
}

/// The main renderer for LMahjong.
///
/// Manages the SDL2 window, canvas, loaded textures, fonts, and placeholder assets.
///
/// IMPORTANT: Field order matters for drop safety. `tile_textures` must be declared
/// before `texture_creator` so it is dropped first (Rust drops in declaration order).
pub struct Renderer {
    /// The SDL2 hardware-accelerated canvas for drawing.
    pub canvas: Canvas<Window>,
    /// TTF context for font rendering.
    pub ttf_context: Sdl2TtfContext,
    /// Loaded tile face textures (None if the asset file was missing).
    /// SAFETY: These textures reference texture_creator and must be dropped first.
    pub tile_textures: Vec<Option<Texture<'static>>>,
    /// Texture creator bound to the window context (for creating textures at runtime).
    pub texture_creator: TextureCreator<WindowContext>,
    /// Whether the tile back texture was loaded.
    pub tile_back_loaded: bool,
    /// Whether the background texture was loaded.
    pub background_loaded: bool,
    /// UI textures state.
    pub ui_textures: UiTextures,
    /// Placeholder tile colors for when textures are missing.
    pub placeholders: PlaceholderTiles,
    /// Background color used when no background texture is available.
    pub background_color: Color,
    /// Tile back color used when no tile back texture is available.
    pub tile_back_color: Color,
}

impl Renderer {
    /// Creates a new Renderer with an SDL2 window and hardware-accelerated canvas.
    ///
    /// Initializes:
    /// - SDL2 video subsystem
    /// - Window (1024×768, resizable, minimum 800×600)
    /// - Hardware-accelerated canvas
    /// - SDL2_ttf for text rendering
    /// - Asset loading with graceful fallback for missing files
    ///
    /// # Arguments
    /// * `sdl_context` - Reference to the initialized SDL2 context
    ///
    /// # Returns
    /// * `Ok(Renderer)` on success
    /// * `Err(String)` if window creation or canvas creation fails
    pub fn new(sdl_context: &sdl2::Sdl) -> Result<Self, String> {
        // Initialize video subsystem
        let video_subsystem = sdl_context.video()?;

        // Detect screen resolution and adjust window size if needed
        let (initial_width, initial_height, min_width, min_height) = {
            let display_mode = video_subsystem.desktop_display_mode(0)
                .unwrap_or(sdl2::video::DisplayMode::new(
                    sdl2::pixels::PixelFormatEnum::Unknown, 
                    DEFAULT_WIDTH as i32, 
                    DEFAULT_HEIGHT as i32, 
                    60
                ));
            let screen_w = display_mode.w as u32;
            let screen_h = display_mode.h as u32;

            // Use desired size or screen resolution, whichever is smaller
            let w = DEFAULT_WIDTH.min(screen_w);
            let h = DEFAULT_HEIGHT.min(screen_h);
            let mw = MIN_WIDTH.min(screen_w);
            let mh = MIN_HEIGHT.min(screen_h);
            (w, h, mw, mh)
        };

        // Create window with detected size, resizable flag
        let mut window = video_subsystem
            .window("LMahjong", initial_width, initial_height)
            .resizable()
            .position_centered()
            .build()
            .map_err(|e| format!("Failed to create window: {}", e))?;

        // Set minimum window size (capped to screen resolution)
        window.set_minimum_size(min_width, min_height)
            .map_err(|e| format!("Failed to set minimum window size: {}", e))?;

        // Attempt to set window icon (Tux icon)
        Self::set_window_icon(&mut window);

        // Create hardware-accelerated canvas
        let canvas = window
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .map_err(|e| format!("Failed to create canvas: {}", e))?;

        // Initialize SDL2_ttf
        let ttf_context = sdl2::ttf::init()
            .map_err(|e| format!("Failed to initialize SDL2_ttf: {}", e))?;

        // Create texture creator
        let texture_creator = canvas.texture_creator();

        // Load assets with graceful fallback
        let tile_textures = Self::load_tile_textures(&texture_creator);
        let tile_back_loaded = Self::load_tile_back(&texture_creator);
        let background_loaded = Self::load_background(&texture_creator);
        let ui_textures = Self::load_ui_textures(&texture_creator);

        Ok(Self {
            canvas,
            texture_creator,
            ttf_context,
            tile_textures,
            tile_back_loaded,
            background_loaded,
            ui_textures,
            placeholders: PlaceholderTiles::new(),
            background_color: Color::RGB(34, 85, 34),  // Dark green felt
            tile_back_color: Color::RGB(240, 230, 200), // Ivory tile back
        })
    }

    /// Attempts to set the window icon to a Tux image.
    /// Logs a warning if the icon file is not found.
    fn set_window_icon(window: &mut Window) {
        let icon_path = format!("{}/icon.png", ASSETS_PATH);
        match sdl2::surface::Surface::load_bmp(&icon_path) {
            Ok(icon_surface) => {
                window.set_icon(&icon_surface);
            }
            Err(_) => {
                // Try PNG via SDL2_image if BMP fails
                // For now, just log the warning
                eprintln!(
                    "[LMahjong] Warning: Window icon not found at '{}'. Using default icon.",
                    icon_path
                );
            }
        }
    }

    /// Attempts to load tile face textures from the assets directory.
    /// Returns a Vec of Option<Texture> - loaded textures or None for missing files.
    /// Missing textures are logged as warnings and will use placeholder colors.
    ///
    /// SAFETY: The returned textures have their lifetime erased to 'static.
    /// The caller must ensure the TextureCreator outlives these textures.
    fn load_tile_textures(texture_creator: &TextureCreator<WindowContext>) -> Vec<Option<Texture<'static>>> {
        let mut textures = Vec::with_capacity(TILE_FACE_COUNT);

        for i in 0..TILE_FACE_COUNT {
            let path = format!("{}/tiles/face_{:02}.png", ASSETS_PATH, i);
            match texture_creator.load_texture(&path) {
                Ok(texture) => {
                    // SAFETY: The texture_creator is stored in the same struct and
                    // outlives this Vec (dropped after it due to field order).
                    let texture: Texture<'static> = unsafe { std::mem::transmute(texture) };
                    textures.push(Some(texture));
                    eprintln!("[LMahjong] Loaded tile texture: {}", path);
                }
                Err(e) => {
                    textures.push(None);
                    eprintln!(
                        "[LMahjong] Warning: Tile texture not found: '{}'. Using placeholder. ({})",
                        path, e
                    );
                }
            }
        }

        textures
    }

    /// Attempts to load the tile back texture.
    fn load_tile_back(_texture_creator: &TextureCreator<WindowContext>) -> bool {
        let path = format!("{}/tiles/tile_back.png", ASSETS_PATH);
        if std::path::Path::new(&path).exists() {
            eprintln!("[LMahjong] Loaded tile back texture: {}", path);
            true
        } else {
            eprintln!(
                "[LMahjong] Warning: Tile back texture not found: '{}'. Using placeholder color.",
                path
            );
            false
        }
    }

    /// Attempts to load the background texture.
    fn load_background(_texture_creator: &TextureCreator<WindowContext>) -> bool {
        let path = format!("{}/background.png", ASSETS_PATH);
        if std::path::Path::new(&path).exists() {
            eprintln!("[LMahjong] Loaded background texture: {}", path);
            true
        } else {
            eprintln!(
                "[LMahjong] Warning: Background texture not found: '{}'. Using solid color.",
                path
            );
            false
        }
    }

    /// Attempts to load UI textures (buttons, overlays).
    fn load_ui_textures(_texture_creator: &TextureCreator<WindowContext>) -> UiTextures {
        let ui_path = format!("{}/ui", ASSETS_PATH);
        if std::path::Path::new(&ui_path).exists() {
            eprintln!("[LMahjong] Loaded UI textures from: {}", ui_path);
            UiTextures { loaded: true }
        } else {
            eprintln!(
                "[LMahjong] Warning: UI textures directory not found: '{}'. Using fallback rendering.",
                ui_path
            );
            UiTextures { loaded: false }
        }
    }

    /// Returns the current window size as (width, height).
    pub fn window_size(&self) -> (u32, u32) {
        self.canvas.output_size().unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT))
    }

    /// Clears the canvas with the background color.
    pub fn clear(&mut self) {
        self.canvas.set_draw_color(self.background_color);
        self.canvas.clear();
    }

    /// Presents the rendered frame to the screen.
    pub fn present(&mut self) {
        self.canvas.present();
    }

    /// Draws a placeholder tile at the given screen rectangle.
    ///
    /// Renders the tile back (ivory rectangle with border) and the face
    /// (colored inner rectangle) using placeholder colors.
    pub fn draw_placeholder_tile(&mut self, face_id: u8, dest: Rect, selected: bool) {
        // Draw tile back (slightly larger for border effect)
        let back_color = if selected {
            Color::RGB(255, 215, 0) // Gold highlight for selection
        } else {
            self.tile_back_color
        };
        self.canvas.set_draw_color(back_color);
        self.canvas.fill_rect(dest).ok();

        // Draw tile border
        self.canvas.set_draw_color(Color::RGB(80, 80, 80));
        self.canvas.draw_rect(dest).ok();

        // Draw face color (inner area)
        let face_color = self.placeholders.color_for(face_id);
        let inner = Rect::new(
            dest.x() + 3,
            dest.y() + 3,
            dest.width().saturating_sub(6),
            dest.height().saturating_sub(6),
        );
        self.canvas.set_draw_color(face_color);
        self.canvas.fill_rect(inner).ok();

        // Draw inner border for depth
        self.canvas.set_draw_color(Color::RGB(60, 60, 60));
        self.canvas.draw_rect(inner).ok();
    }

    /// Renders all tiles on the board with depth effect (bottom-to-top ordering).
    ///
    /// Tiles are sorted by layer so that lower layers are drawn first and higher
    /// layers paint over them. Each layer is offset by a few pixels to simulate depth.
    /// Animation states (removal, mismatch, hint glow, shuffle) are detected from
    /// `state.animations` and applied as highlight effects.
    pub fn render_board(&mut self, state: &GameState, _layout_rect: Rect) {
        let now = Instant::now();

        // Determine if shuffle animation is active (dims/hides tiles during shuffle)
        let shuffle_progress = self.get_shuffle_progress(state, now);

        // Collect tile positions with their indices, sorted by layer (bottom-to-top)
        let layout = state.board.layout;
        let mut tile_entries: Vec<(usize, &TilePosition)> = layout
            .positions
            .iter()
            .enumerate()
            .filter(|(idx, _)| state.board.tiles[*idx].is_some())
            .collect();

        // Sort by layer ascending so lower layers are drawn first
        tile_entries.sort_by_key(|(_, pos)| pos.layer);

        // Use the same LayoutMetrics as hit_test for consistent positioning
        let (win_w, win_h) = self.window_size();
        let metrics = compute_layout_rect(win_w, win_h);

        for (idx, pos) in &tile_entries {
            let tile = state.board.tiles[*idx].unwrap();

            // Calculate screen rectangle using the same function as hit_test
            let dest = tile_screen_rect(pos, &metrics);

            // Determine highlight for this tile
            let highlight = self.determine_highlight(state, *idx, now);

            // If shuffle animation is active, skip rendering individual tile effects
            // and instead render with a shuffle visual
            if let Some(progress) = shuffle_progress {
                self.render_tile_with_shuffle(tile.face_id, dest, pos.layer, progress);
            } else {
                self.render_tile(tile.face_id, dest, pos.layer, highlight);
            }
        }
    }

    /// Renders a single tile with optional highlight effects and depth shadow.
    ///
    /// The tile is drawn with:
    /// - A shadow rectangle offset below/right for depth perception
    /// - The tile body (back color + face color)
    /// - An optional highlight effect (selection border, hint glow, mismatch flash, removal fade)
    fn render_tile(&mut self, face_id: u8, dest: Rect, layer: u8, highlight: TileHighlight) {
        // Handle removal fade-out: skip drawing if fully transparent
        if let TileHighlight::Removing(alpha) = highlight {
            if alpha <= 0.0 {
                return;
            }
        }

        // Draw shadow for depth effect (only for layers > 0)
        if layer > 0 {
            let shadow_rect = Rect::new(
                dest.x() + SHADOW_OFFSET_PX,
                dest.y() + SHADOW_OFFSET_PX,
                dest.width(),
                dest.height(),
            );
            self.canvas.set_draw_color(Color::RGBA(0, 0, 0, 80));
            self.canvas.fill_rect(shadow_rect).ok();
        }

        // Determine tile back color based on highlight
        let (back_color, border_color) = match highlight {
            TileHighlight::None => (self.tile_back_color, Color::RGB(80, 80, 80)),
            TileHighlight::Selected => (
                Color::RGB(255, 215, 0), // Gold
                Color::RGB(218, 165, 32), // Darker gold border
            ),
            TileHighlight::HintGlow(phase) => {
                // Pulsing glow: interpolate between normal and bright cyan
                let intensity = ((phase * std::f32::consts::PI * 2.0).sin() + 1.0) / 2.0;
                let r = (240.0 + intensity * 15.0) as u8;
                let g = (230.0 + intensity * 25.0) as u8;
                let b = (200.0 + intensity * 55.0) as u8;
                let br = (80.0 + intensity * 100.0) as u8;
                let bg = (80.0 + intensity * 180.0) as u8;
                let bb = (80.0 + intensity * 175.0) as u8;
                (Color::RGB(r, g, b), Color::RGB(br, bg, bb))
            }
            TileHighlight::MismatchFlash => (
                Color::RGB(255, 100, 100), // Red flash
                Color::RGB(200, 0, 0),     // Dark red border
            ),
            TileHighlight::Removing(alpha) => {
                // Fade out: reduce color intensity based on alpha
                let a = (alpha * 255.0) as u8;
                let back = self.tile_back_color;
                (
                    Color::RGBA(back.r, back.g, back.b, a),
                    Color::RGBA(80, 80, 80, a),
                )
            }
        };

        // Draw tile back/body
        self.canvas.set_draw_color(back_color);
        self.canvas.fill_rect(dest).ok();

        // Draw border (thicker for selected/hint/mismatch)
        self.canvas.set_draw_color(border_color);
        self.canvas.draw_rect(dest).ok();

        // For selected/hint/mismatch: draw an extra inner border for emphasis
        match highlight {
            TileHighlight::Selected | TileHighlight::HintGlow(_) | TileHighlight::MismatchFlash => {
                let inner_border = Rect::new(
                    dest.x() + 1,
                    dest.y() + 1,
                    dest.width().saturating_sub(2),
                    dest.height().saturating_sub(2),
                );
                self.canvas.draw_rect(inner_border).ok();
            }
            _ => {}
        }

        // Draw face: use loaded texture if available, otherwise placeholder color
        let inner = Rect::new(
            dest.x() + 3,
            dest.y() + 3,
            dest.width().saturating_sub(6),
            dest.height().saturating_sub(6),
        );

        if let Some(Some(texture)) = self.tile_textures.get(face_id as usize) {
            // Draw the actual tile texture
            if let TileHighlight::Removing(alpha) = highlight {
                let a = (alpha * 255.0) as u8;
                // We can't set alpha on an immutable texture ref easily,
                // so just draw it (fade effect handled by back color)
                let _ = a; // Alpha fade is visual from the back color change
            }
            self.canvas.copy(texture, None, inner).ok();
        } else {
            // Fallback to placeholder color
            let face_color = self.placeholders.color_for(face_id);
            let face_draw_color = if let TileHighlight::Removing(alpha) = highlight {
                let a = (alpha * 255.0) as u8;
                Color::RGBA(face_color.r, face_color.g, face_color.b, a)
            } else {
                face_color
            };
            self.canvas.set_draw_color(face_draw_color);
            self.canvas.fill_rect(inner).ok();
        }

        // Draw inner border
        self.canvas.set_draw_color(Color::RGB(60, 60, 60));
        self.canvas.draw_rect(inner).ok();
    }

    /// Determines the highlight state for a tile at the given position index.
    fn determine_highlight(&self, state: &GameState, pos_idx: usize, now: Instant) -> TileHighlight {
        // Check for active removal animation on this position
        for anim in &state.animations {
            match anim {
                Animation::TileRemoval {
                    positions,
                    start_time,
                    duration_ms,
                } => {
                    if positions.0 == pos_idx || positions.1 == pos_idx {
                        let elapsed_ms = now.duration_since(*start_time).as_millis() as u32;
                        let progress = (elapsed_ms as f32 / *duration_ms as f32).min(1.0);
                        return TileHighlight::Removing(1.0 - progress);
                    }
                }
                Animation::TileMismatch {
                    positions,
                    start_time,
                    duration_ms,
                } => {
                    if positions.0 == pos_idx || positions.1 == pos_idx {
                        let elapsed_ms = now.duration_since(*start_time).as_millis() as u32;
                        if elapsed_ms < *duration_ms {
                            return TileHighlight::MismatchFlash;
                        }
                    }
                }
                Animation::HintPulse {
                    positions,
                    start_time,
                } => {
                    if positions.0 == pos_idx || positions.1 == pos_idx {
                        let elapsed_ms = now.duration_since(*start_time).as_millis() as u32;
                        let phase = (elapsed_ms % HINT_PULSE_CYCLE_MS) as f32
                            / HINT_PULSE_CYCLE_MS as f32;
                        return TileHighlight::HintGlow(phase);
                    }
                }
                Animation::Shuffle { .. } => {
                    // Shuffle animation is handled at the board level, not per-tile
                }
            }
        }

        // Check hint state (non-animation based hint display)
        if let Some(ref hint) = state.hint {
            if hint.position_a == pos_idx || hint.position_b == pos_idx {
                let elapsed_ms = now.duration_since(hint.activated_at).as_millis() as u32;
                let phase = (elapsed_ms % HINT_PULSE_CYCLE_MS) as f32 / HINT_PULSE_CYCLE_MS as f32;
                return TileHighlight::HintGlow(phase);
            }
        }

        // Check selection state
        if state.selection == Some(pos_idx) {
            return TileHighlight::Selected;
        }

        TileHighlight::None
    }

    /// Returns the shuffle animation progress (0.0–1.0) if a shuffle animation is active.
    fn get_shuffle_progress(&self, state: &GameState, now: Instant) -> Option<f32> {
        for anim in &state.animations {
            if let Animation::Shuffle {
                start_time,
                duration_ms,
            } = anim
            {
                let elapsed_ms = now.duration_since(*start_time).as_millis() as u32;
                if elapsed_ms < *duration_ms {
                    return Some(elapsed_ms as f32 / *duration_ms as f32);
                }
            }
        }
        None
    }

    /// Renders a tile during shuffle animation with a visual shuffle effect.
    /// The tile briefly flashes/fades based on shuffle progress.
    fn render_tile_with_shuffle(&mut self, face_id: u8, dest: Rect, layer: u8, progress: f32) {
        // During shuffle: tiles fade out in first half, fade in with new faces in second half
        let alpha = if progress < 0.5 {
            // Fading out: 1.0 -> 0.0 over first half
            1.0 - (progress * 2.0)
        } else {
            // Fading in: 0.0 -> 1.0 over second half
            (progress - 0.5) * 2.0
        };

        // Draw shadow for depth
        if layer > 0 {
            let shadow_rect = Rect::new(
                dest.x() + SHADOW_OFFSET_PX,
                dest.y() + SHADOW_OFFSET_PX,
                dest.width(),
                dest.height(),
            );
            let shadow_alpha = (80.0 * alpha) as u8;
            self.canvas.set_draw_color(Color::RGBA(0, 0, 0, shadow_alpha));
            self.canvas.fill_rect(shadow_rect).ok();
        }

        // Draw tile with reduced alpha
        let a = (alpha * 255.0) as u8;
        let back = self.tile_back_color;
        self.canvas.set_draw_color(Color::RGBA(back.r, back.g, back.b, a));
        self.canvas.fill_rect(dest).ok();

        self.canvas.set_draw_color(Color::RGBA(80, 80, 80, a));
        self.canvas.draw_rect(dest).ok();

        let inner = Rect::new(
            dest.x() + 3,
            dest.y() + 3,
            dest.width().saturating_sub(6),
            dest.height().saturating_sub(6),
        );

        if let Some(Some(texture)) = self.tile_textures.get(face_id as usize) {
            // Draw texture with alpha modulation for shuffle fade
            // Note: SDL2 texture alpha mod would require mutable access to texture
            self.canvas.copy(texture, None, inner).ok();
        } else {
            let face_color = self.placeholders.color_for(face_id);
            self.canvas.set_draw_color(Color::RGBA(face_color.r, face_color.g, face_color.b, a));
            self.canvas.fill_rect(inner).ok();
        }

        self.canvas.set_draw_color(Color::RGBA(60, 60, 60, a));
        self.canvas.draw_rect(inner).ok();
    }

    /// Computes tile dimensions and offset based on the layout and available screen area.
    ///
    /// Returns (tile_width, tile_height, x_offset, y_offset) where offsets position
    /// the layout within the given layout_rect.
    #[allow(dead_code)]
    fn compute_tile_geometry(
        &self,
        layout: &crate::board::Layout,
        layout_rect: Rect,
    ) -> (u32, u32, i32, i32) {
        // Find the extent of the layout in grid units
        let max_col = layout
            .positions
            .iter()
            .map(|p| p.col as u32)
            .max()
            .unwrap_or(0)
            + 2; // +2 because each tile occupies 2 grid units
        let max_row = layout
            .positions
            .iter()
            .map(|p| p.row as u32)
            .max()
            .unwrap_or(0)
            + 2;

        // Each tile occupies 2x2 grid cells, so the tile is half the cell width
        // Calculate tile size to fit within layout_rect
        let available_w = layout_rect.width();
        let available_h = layout_rect.height();

        // Tile width = available width / (max_col / 2) since tiles are 2 grid units wide
        // But we position tiles at col * (tile_w / 2), so:
        // total_width = max_col * (tile_w / 2) + tile_w
        // Solve: tile_w = available_w / (max_col/2 + 1)
        let tile_w = (available_w * 2) / (max_col + 2);
        let tile_h = (available_h * 2) / (max_row + 2);

        // Use the smaller dimension to maintain aspect ratio (tiles are roughly 4:5)
        let tile_w = tile_w.min(tile_h * 4 / 5);
        let tile_h = tile_h.min(tile_w * 5 / 4);

        // Center the layout within layout_rect
        let total_w = max_col * (tile_w / 2) + tile_w;
        let total_h = max_row * (tile_h / 2) + tile_h;
        let x_offset = layout_rect.x() + (available_w as i32 - total_w as i32) / 2;
        let y_offset = layout_rect.y() + (available_h as i32 - total_h as i32) / 2;

        (tile_w, tile_h, x_offset, y_offset)
    }

    /// Loads a font from the assets directory at the given point size.
    /// Returns None if the font file is not found.
    pub fn load_font(&self, point_size: u16) -> Option<sdl2::ttf::Font<'_, 'static>> {
        let font_path = format!("{}/fonts/default.ttf", ASSETS_PATH);
        match self.ttf_context.load_font(&font_path, point_size) {
            Ok(font) => Some(font),
            Err(e) => {
                eprintln!(
                    "[LMahjong] Warning: Could not load font '{}': {}. Text rendering disabled.",
                    font_path, e
                );
                None
            }
        }
    }

    // ─── UI Overlay Rendering ────────────────────────────────────────────────────

    /// Renders a semi-transparent dark overlay covering the entire window.
    /// Used as backdrop for menus, dialogs, and notifications.
    fn draw_overlay_backdrop(&mut self) {
        let (w, h) = self.window_size();
        self.canvas.set_draw_color(Color::RGBA(0, 0, 0, 180));
        self.canvas.fill_rect(Rect::new(0, 0, w, h)).ok();
    }

    /// Draws a centered dialog box with the given dimensions.
    /// Returns the Rect of the dialog for positioning child elements.
    fn draw_dialog_box(&mut self, width: u32, height: u32) -> Rect {
        let (win_w, win_h) = self.window_size();
        let x = (win_w.saturating_sub(width)) / 2;
        let y = (win_h.saturating_sub(height)) / 2;
        let dialog = Rect::new(x as i32, y as i32, width, height);

        // Dialog background
        self.canvas.set_draw_color(Color::RGB(45, 45, 60));
        self.canvas.fill_rect(dialog).ok();

        // Dialog border
        self.canvas.set_draw_color(Color::RGB(100, 140, 180));
        self.canvas.draw_rect(dialog).ok();

        // Inner border for depth
        let inner = Rect::new(
            dialog.x() + 2,
            dialog.y() + 2,
            dialog.width().saturating_sub(4),
            dialog.height().saturating_sub(4),
        );
        self.canvas.set_draw_color(Color::RGB(70, 90, 120));
        self.canvas.draw_rect(inner).ok();

        dialog
    }

    /// Draws a placeholder button (colored rectangle) at the given position.
    /// The color distinguishes button types. Returns the button Rect for hit-testing.
    #[allow(dead_code)]
    fn draw_button(&mut self, x: i32, y: i32, width: u32, height: u32, color: Color) -> Rect {
        let btn = Rect::new(x, y, width, height);

        // Button fill
        self.canvas.set_draw_color(color);
        self.canvas.fill_rect(btn).ok();

        // Button border (lighter for 3D effect)
        self.canvas.set_draw_color(Color::RGB(
            color.r.saturating_add(40),
            color.g.saturating_add(40),
            color.b.saturating_add(40),
        ));
        self.canvas.draw_rect(btn).ok();

        // Text area placeholder (slightly lighter inner rectangle to represent label)
        let text_area = Rect::new(
            x + 8,
            y + 4,
            width.saturating_sub(16),
            height.saturating_sub(8),
        );
        self.canvas.set_draw_color(Color::RGBA(255, 255, 255, 60));
        self.canvas.fill_rect(text_area).ok();

        btn
    }

    /// Draws a placeholder text label area at the given position.
    /// Without a loaded font, this renders a subtle rectangle where text would appear.
    fn draw_text_placeholder(&mut self, x: i32, y: i32, width: u32, height: u32, color: Color) {
        let area = Rect::new(x, y, width, height);
        self.canvas.set_draw_color(color);
        self.canvas.fill_rect(area).ok();
    }

    /// Draws text using a simple built-in bitmap font (5×7 pixel characters).
    /// Each character is scaled by `scale` factor. Color is specified by `color`.
    /// This works without any TTF font file.
    fn draw_bitmap_text(&mut self, text: &str, x: i32, y: i32, scale: u32, color: Color) {
        self.canvas.set_draw_color(color);
        let mut cursor_x = x;
        for ch in text.chars() {
            if let Some(glyph) = bitmap_glyph(ch) {
                for (row_idx, &row_bits) in glyph.iter().enumerate() {
                    for col in 0..5u32 {
                        if row_bits & (1 << (4 - col)) != 0 {
                            let px = cursor_x + (col * scale) as i32;
                            let py = y + (row_idx as u32 * scale) as i32;
                            self.canvas.fill_rect(Rect::new(px, py, scale, scale)).ok();
                        }
                    }
                }
            }
            cursor_x += (6 * scale) as i32; // 5px char + 1px spacing
        }
    }

    /// Draws a labeled button with readable bitmap text.
    fn draw_labeled_button(&mut self, x: i32, y: i32, width: u32, height: u32, color: Color, label: &str) -> Rect {
        let btn = Rect::new(x, y, width, height);

        // Button fill
        self.canvas.set_draw_color(color);
        self.canvas.fill_rect(btn).ok();

        // Button border (lighter for 3D effect)
        self.canvas.set_draw_color(Color::RGB(
            color.r.saturating_add(40),
            color.g.saturating_add(40),
            color.b.saturating_add(40),
        ));
        self.canvas.draw_rect(btn).ok();

        // Draw the label text centered within the button
        let text_scale = 2u32;
        let text_w = label.len() as i32 * 6 * text_scale as i32;
        let text_h = 7 * text_scale as i32;
        let tx = x + (width as i32 - text_w) / 2;
        let ty = y + (height as i32 - text_h) / 2;
        self.draw_bitmap_text(label, tx, ty, text_scale, Color::RGB(255, 255, 255));

        btn
    }

    /// Renders the HUD overlay: timer display (MM:SS), score, and shuffles remaining.
    ///
    /// The HUD is drawn as a top bar with three sections:
    /// - Left: Timer display
    /// - Center: Score
    /// - Right: Shuffle count remaining
    ///
    /// Without fonts, these are rendered as colored rectangles with
    /// distinguishable colors to indicate each element's purpose.
    pub fn render_hud(&mut self, state: &GameState) {
        let (win_w, _win_h) = self.window_size();

        // HUD background bar at the top
        let hud_height: u32 = 40;
        let hud_rect = Rect::new(0, 0, win_w, hud_height);
        self.canvas.set_draw_color(Color::RGBA(20, 20, 30, 200));
        self.canvas.fill_rect(hud_rect).ok();

        // Bottom border of HUD
        self.canvas.set_draw_color(Color::RGB(80, 120, 160));
        self.canvas.draw_line(
            sdl2::rect::Point::new(0, hud_height as i32),
            sdl2::rect::Point::new(win_w as i32, hud_height as i32),
        ).ok();

        // Timer display (left side)
        let timer_text = state.timer.format_display();
        self.draw_bitmap_text(&timer_text, 16, 12, 2, Color::RGB(100, 220, 100));

        // Score display (center)
        let score = state.score.calculate_score();
        let score_text = format!("SCORE {}", score);
        let score_w = score_text.len() as i32 * 12; // 6px * scale 2
        let score_x = (win_w as i32 - score_w) / 2;
        self.draw_bitmap_text(&score_text, score_x, 12, 2, Color::RGB(255, 215, 0));

        // Shuffles remaining (right side)
        let shuffle_text = format!("SHUFFLE {}", state.shuffles_remaining);
        let shuffle_w = shuffle_text.len() as i32 * 12;
        let shuffle_x = win_w as i32 - shuffle_w - 16;
        self.draw_bitmap_text(&shuffle_text, shuffle_x, 12, 2, Color::RGB(100, 150, 255));
    }

    /// Renders the pause menu overlay with game options.
    ///
    /// Menu items (rendered as placeholder buttons):
    /// - New Game (green)
    /// - Undo (blue)
    /// - Hint (cyan)
    /// - Shuffle (purple)
    /// - Mute toggle (orange)
    /// - Quit (red)
    pub fn render_menu(&mut self) {
        self.draw_overlay_backdrop();

        let dialog = self.draw_dialog_box(300, 380);

        // Title
        self.draw_bitmap_text(
            "MENU",
            dialog.x() + 120,
            dialog.y() + 18,
            3,
            Color::RGB(200, 220, 255),
        );

        let btn_w: u32 = 220;
        let btn_h: u32 = 40;
        let btn_x = dialog.x() + ((dialog.width() - btn_w) / 2) as i32;
        let start_y = dialog.y() + 60;
        let spacing: i32 = 50;

        // New Game button (green)
        self.draw_labeled_button(btn_x, start_y, btn_w, btn_h, Color::RGB(50, 140, 70), "NEW GAME");

        // Undo button (blue)
        self.draw_labeled_button(btn_x, start_y + spacing, btn_w, btn_h, Color::RGB(50, 100, 180), "UNDO");

        // Hint button (cyan)
        self.draw_labeled_button(btn_x, start_y + spacing * 2, btn_w, btn_h, Color::RGB(50, 160, 170), "HINT");

        // Shuffle button (purple)
        self.draw_labeled_button(btn_x, start_y + spacing * 3, btn_w, btn_h, Color::RGB(120, 60, 160), "SHUFFLE");

        // Mute toggle button (orange)
        self.draw_labeled_button(btn_x, start_y + spacing * 4, btn_w, btn_h, Color::RGB(200, 130, 50), "MUTE");

        // Quit button (red)
        self.draw_labeled_button(btn_x, start_y + spacing * 5, btn_w, btn_h, Color::RGB(180, 50, 50), "QUIT");
    }

    /// Renders the victory overlay showing final time and score.
    ///
    /// Displays:
    /// - Victory message
    /// - Final time (MM:SS format)
    /// - Final score
    /// - New Game button
    /// - Leaderboard button
    ///
    /// # Arguments
    /// * `time` - Formatted time string (e.g., "05:23")
    /// * `score` - Final score value
    pub fn render_victory(&mut self, time: &str, score: u32) {
        self.draw_overlay_backdrop();

        let dialog = self.draw_dialog_box(350, 300);

        // "VICTORY!" title
        self.draw_bitmap_text(
            "VICTORY!",
            dialog.x() + 110,
            dialog.y() + 24,
            3,
            Color::RGB(255, 215, 0),
        );

        // Time display
        let time_label = format!("TIME  {}", time);
        self.draw_bitmap_text(
            &time_label,
            dialog.x() + 100,
            dialog.y() + 75,
            2,
            Color::RGB(100, 220, 100),
        );

        // Score display
        let score_label = format!("SCORE {}", score);
        self.draw_bitmap_text(
            &score_label,
            dialog.x() + 100,
            dialog.y() + 110,
            2,
            Color::RGB(255, 200, 50),
        );

        let btn_w: u32 = 220;
        let btn_h: u32 = 44;
        let btn_x = dialog.x() + ((dialog.width() - btn_w) / 2) as i32;

        // New Game button (green)
        self.draw_labeled_button(btn_x, dialog.y() + 160, btn_w, btn_h, Color::RGB(50, 140, 70), "NEW GAME");

        // Leaderboard button (blue)
        self.draw_labeled_button(btn_x, dialog.y() + 220, btn_w, btn_h, Color::RGB(50, 100, 180), "LEADERBOARD");
    }

    /// Renders the no-moves notification with options to shuffle or start a new game.
    ///
    /// Displayed when no valid pairs remain but tiles are still on the board.
    /// Options:
    /// - Shuffle (if shuffles remaining > 0)
    /// - New Game
    pub fn render_no_moves(&mut self) {
        self.draw_overlay_backdrop();

        let dialog = self.draw_dialog_box(320, 220);

        // "NO MOVES!" title
        self.draw_bitmap_text(
            "NO MOVES!",
            dialog.x() + 85,
            dialog.y() + 22,
            3,
            Color::RGB(255, 120, 80),
        );

        // Explanatory text
        self.draw_bitmap_text(
            "NO VALID PAIRS REMAIN",
            dialog.x() + 40,
            dialog.y() + 68,
            2,
            Color::RGB(180, 180, 200),
        );

        let btn_w: u32 = 200;
        let btn_h: u32 = 44;
        let btn_x = dialog.x() + ((dialog.width() - btn_w) / 2) as i32;

        // Shuffle button (purple)
        self.draw_labeled_button(btn_x, dialog.y() + 110, btn_w, btn_h, Color::RGB(120, 60, 160), "SHUFFLE");

        // New Game button (green)
        self.draw_labeled_button(btn_x, dialog.y() + 164, btn_w, btn_h, Color::RGB(50, 140, 70), "NEW GAME");
    }

    /// Renders the name entry overlay for the leaderboard.
    ///
    /// Displays:
    /// - "High Score!" title
    /// - Score and time info
    /// - Text input field showing the current name
    /// - Instructions (Enter to submit, Esc to cancel)
    ///
    /// # Arguments
    /// * `name` - The current text in the name buffer
    /// * `score` - The qualifying score
    /// * `time_seconds` - The game completion time in seconds
    pub fn render_name_entry(&mut self, name: &str, score: u32, time_seconds: u32) {
        self.draw_overlay_backdrop();

        let dialog = self.draw_dialog_box(400, 280);

        // "High Score!" title placeholder (gold)
        self.draw_text_placeholder(
            dialog.x() + 120,
            dialog.y() + 16,
            160,
            32,
            Color::RGBA(255, 215, 0, 220), // Gold for "HIGH SCORE!"
        );

        // Score display placeholder
        let _score_text = format!("Score: {}", score);
        self.draw_text_placeholder(
            dialog.x() + 130,
            dialog.y() + 60,
            140,
            22,
            Color::RGBA(255, 200, 50, 180), // Gold for score
        );

        // Time display placeholder
        let minutes = time_seconds / 60;
        let seconds = time_seconds % 60;
        let _time_text = format!("Time: {:02}:{:02}", minutes, seconds);
        self.draw_text_placeholder(
            dialog.x() + 130,
            dialog.y() + 88,
            140,
            22,
            Color::RGBA(100, 200, 100, 180), // Green for time
        );

        // "Enter your name:" label
        self.draw_text_placeholder(
            dialog.x() + 40,
            dialog.y() + 124,
            160,
            20,
            Color::RGBA(200, 200, 220, 180), // Light gray label
        );

        // Text input field background
        let input_x = dialog.x() + 40;
        let input_y = dialog.y() + 150;
        let input_w: u32 = 320;
        let input_h: u32 = 36;
        let input_rect = Rect::new(input_x, input_y, input_w, input_h);

        // Input field background (dark)
        self.canvas.set_draw_color(Color::RGB(25, 25, 35));
        self.canvas.fill_rect(input_rect).ok();

        // Input field border (lighter when focused)
        self.canvas.set_draw_color(Color::RGB(100, 160, 220));
        self.canvas.draw_rect(input_rect).ok();

        // Render the typed name text as a colored bar proportional to text length
        if !name.is_empty() {
            let char_count = name.chars().count() as u32;
            // Each character is represented as ~14px wide, capped to input width
            let text_w = (char_count * 14).min(input_w - 8);
            let text_rect = Rect::new(input_x + 4, input_y + 6, text_w, input_h - 12);
            self.canvas.set_draw_color(Color::RGBA(220, 220, 240, 220));
            self.canvas.fill_rect(text_rect).ok();
        }

        // Blinking cursor (simple solid bar after text)
        let cursor_x = input_x + 4 + (name.chars().count() as i32 * 14).min((input_w as i32) - 12);
        let cursor_rect = Rect::new(cursor_x, input_y + 6, 2, input_h - 12);
        self.canvas.set_draw_color(Color::RGBA(200, 220, 255, 200));
        self.canvas.fill_rect(cursor_rect).ok();

        // Instructions: "Press Enter to submit, Esc to skip"
        self.draw_text_placeholder(
            dialog.x() + 60,
            dialog.y() + 200,
            280,
            18,
            Color::RGBA(140, 140, 160, 150), // Dim gray for instructions
        );

        // Character count indicator
        let char_count = name.chars().count() as u32;
        let count_color = if char_count == 0 {
            Color::RGBA(200, 80, 80, 180) // Red if empty
        } else if char_count > 20 {
            Color::RGBA(200, 80, 80, 180) // Red if over limit
        } else {
            Color::RGBA(100, 200, 100, 180) // Green if valid
        };
        self.draw_text_placeholder(
            dialog.x() + 320,
            dialog.y() + 230,
            60,
            16,
            count_color,
        );
    }

    /// Renders the quit confirmation dialog.
    ///
    /// Asks the player to confirm quitting the current game.
    /// Options:
    /// - Yes / Confirm (red)
    /// - No / Cancel (gray)
    pub fn render_quit_confirmation(&mut self) {
        self.draw_overlay_backdrop();

        let dialog = self.draw_dialog_box(300, 180);

        // "QUIT GAME?" title
        self.draw_bitmap_text(
            "QUIT GAME?",
            dialog.x() + 90,
            dialog.y() + 24,
            3,
            Color::RGB(255, 160, 160),
        );

        // "PROGRESS WILL BE LOST"
        self.draw_bitmap_text(
            "PROGRESS WILL BE LOST",
            dialog.x() + 30,
            dialog.y() + 64,
            2,
            Color::RGB(180, 180, 200),
        );

        let btn_w: u32 = 120;
        let btn_h: u32 = 40;
        let btn_spacing: i32 = 20;

        // Center the two buttons horizontally
        let total_btn_width = (btn_w * 2) as i32 + btn_spacing;
        let btn_start_x = dialog.x() + (dialog.width() as i32 - total_btn_width) / 2;
        let btn_y = dialog.y() + 110;

        // Yes button (red) with label
        self.draw_labeled_button(btn_start_x, btn_y, btn_w, btn_h, Color::RGB(180, 50, 50), "YES");

        // No button (gray) with label
        self.draw_labeled_button(
            btn_start_x + btn_w as i32 + btn_spacing,
            btn_y,
            btn_w,
            btn_h,
            Color::RGB(100, 100, 110),
            "NO",
        );
    }
}

/// Returns a 5×7 bitmap glyph for a given character.
/// Each element is a u8 where bits 4..0 represent pixels left-to-right.
/// Returns None for unsupported characters.
fn bitmap_glyph(ch: char) -> Option<&'static [u8; 7]> {
    match ch.to_ascii_uppercase() {
        'A' => Some(&[0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        'B' => Some(&[0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110]),
        'C' => Some(&[0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110]),
        'D' => Some(&[0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110]),
        'E' => Some(&[0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
        'F' => Some(&[0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
        'G' => Some(&[0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110]),
        'H' => Some(&[0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        'I' => Some(&[0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        'J' => Some(&[0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100]),
        'K' => Some(&[0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001]),
        'L' => Some(&[0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
        'M' => Some(&[0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001]),
        'N' => Some(&[0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001]),
        'O' => Some(&[0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        'P' => Some(&[0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000]),
        'Q' => Some(&[0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101]),
        'R' => Some(&[0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
        'S' => Some(&[0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110]),
        'T' => Some(&[0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
        'U' => Some(&[0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        'V' => Some(&[0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100]),
        'W' => Some(&[0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001]),
        'X' => Some(&[0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001]),
        'Y' => Some(&[0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
        'Z' => Some(&[0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]),
        '0' => Some(&[0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
        '1' => Some(&[0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        '2' => Some(&[0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111]),
        '3' => Some(&[0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110]),
        '4' => Some(&[0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
        '5' => Some(&[0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
        '6' => Some(&[0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
        '7' => Some(&[0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
        '8' => Some(&[0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
        '9' => Some(&[0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110]),
        ' ' => Some(&[0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000]),
        ':' => Some(&[0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000]),
        '?' => Some(&[0b01110, 0b10001, 0b00001, 0b00110, 0b00100, 0b00000, 0b00100]),
        '!' => Some(&[0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100]),
        '-' => Some(&[0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000]),
        '/' => Some(&[0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000]),
        '+' => Some(&[0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000]),
        '.' => Some(&[0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100]),
        ',' => Some(&[0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b01000]),
        '(' => Some(&[0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010]),
        ')' => Some(&[0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000]),
        _ => None,
    }
}

/// Converts HSV color values to RGB.
/// Hue in [0, 360), Saturation in [0, 1], Value in [0, 1].
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
}
