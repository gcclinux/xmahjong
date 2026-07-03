# LMahjong

A Tux-themed Mahjong solitaire game for Linux, built with Rust and SDL2.

## About

LMahjong is a classic tile-matching solitaire game featuring Tux penguin-themed graphics. Clear all 144 tiles from the board by matching pairs of free tiles. The game uses the traditional Turtle layout with 5 stacked layers, and every generated board is guaranteed to be solvable.

### Features

- Classic Turtle layout with 144 tiles across 5 layers
- Guaranteed solvable boards via reverse-deal generation
- Hint system, undo (up to 10 moves), and shuffle (up to 3 per game)
- Timer and scoring system with local leaderboard (top 10)
- Keyboard shortcuts for all actions
- Audio feedback with mute support
- Resizable window (min 1920×1080, adapts to screen resolution)
- Ubuntu Snap package distribution

## Prerequisites

### System Dependencies

You need SDL2 development libraries installed:

**Ubuntu / Debian:**

```bash
sudo apt install libsdl2-dev libsdl2-image-dev libsdl2-mixer-dev libsdl2-ttf-dev pkg-config
```

**Fedora:**

```bash
sudo dnf install SDL2-devel SDL2_image-devel SDL2_mixer-devel SDL2_ttf-devel pkg-config
```

**Arch Linux:**

```bash
sudo pacman -S sdl2 sdl2_image sdl2_mixer sdl2_ttf pkg-config
```

### Rust Toolchain

Install Rust via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release
```

The binary is output to `target/debug/lmahjong` or `target/release/lmahjong`.

## Running

```bash
# Run directly
cargo run

# Or run the release binary
cargo run --release
```

## Running Tests

```bash
# Run all tests (unit + property-based)
cargo test

# Run only unit tests
cargo test --lib

# Run a specific property test file
cargo test --test board_properties

# Run tests with output shown
cargo test -- --nocapture
```

The project includes 19 property-based tests using `proptest` that validate correctness properties like board generation invariants, solvability, matching logic, undo/redo behavior, shuffle guarantees, and layout scaling.

## Controls

| Shortcut | Action |
|----------|--------|
| Left click | Select tile |
| Ctrl+S | Save game |
| Ctrl+Q | Save + Quit |
| Ctrl+N | New Game |
| Ctrl+R | Resume |
| Ctrl+P | Pause |
| Ctrl+M | Toggle Mute |
| Shift+S | Shuffle |
| Shift+U | Undo |
| Shift+H | Hint |
| Escape | Pause / Resume (toggle) |
| F11 | Toggle Fullscreen |

## Scoring

Score starts at 0 and increases with each pair matched:

- **Base:** +10 points per pair removed
- **Streak bonus:** +2 per pair (rewards continuous play)
- **Penalties:** −5 per hint used, −10 per shuffle used
- **Time bonus** (at game end): max(0, 500 − elapsed_seconds)

Top 10 scores are saved to a local leaderboard.

## Installing as a Snap

Build and install locally:

```bash
# Install snapcraft if needed
sudo snap install snapcraft --classic

# Build the snap
snapcraft

# Install the local snap
sudo snap install lmahjong_*.snap --dangerous
```

After installation, run with:

```bash
lmahjong
```

### Snap Interfaces

The snap requires these interfaces (auto-connected on most systems):

```bash
sudo snap connect lmahjong:x11
sudo snap connect lmahjong:opengl
sudo snap connect lmahjong:audio-playback
```

## Assets

![Tux Penguin Sprite Sheet](sprite/tux-sprite-lmahjong.png)

## Data Storage

- **Leaderboard:** `$SNAP_USER_DATA/leaderboard.json` (snap) or `~/.local/share/lmahjong/leaderboard.json`
- **Settings:** `$SNAP_USER_DATA/settings.json` or `~/.local/share/lmahjong/settings.json`
- **Saved game:** `$SNAP_USER_DATA/savegame.json` or `~/.local/share/lmahjong/savegame.json`

## Project Structure

```
src/
├── main.rs          # Game loop and SDL2 initialization
├── lib.rs           # Module exports
├── board.rs         # Tile layout, positions, free-tile detection
├── generator.rs     # Reverse-deal solvable board generation
├── logic.rs         # Selection, matching, undo, shuffle, hints
├── game_state.rs    # Central game state and types
├── timer.rs         # Elapsed time tracking with pause
├── renderer.rs      # SDL2 rendering, layout scaling, UI overlays
├── input.rs         # Event processing and keyboard shortcuts
├── audio.rs         # SDL2_mixer audio with graceful degradation
└── storage.rs       # Leaderboard and settings persistence
tests/
├── board_properties.rs      # Properties 1, 4
├── generator_properties.rs  # Properties 2, 3
├── logic_properties.rs      # Properties 5-11
├── shuffle_properties.rs    # Properties 12-14
├── timer_properties.rs      # Property 15
├── score_properties.rs      # Property 16
├── storage_properties.rs    # Properties 17, 18
└── renderer_properties.rs   # Property 19
snap/
├── snapcraft.yaml           # Snap build configuration
├── gui/
│   ├── lmahjong.desktop     # Desktop entry
│   └── icon.png             # Application icon
└── local/
    └── launcher.sh          # Interface detection wrapper
```

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).
