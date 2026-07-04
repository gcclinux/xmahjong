# LMahjong

A Tux-themed Mahjong solitaire game for Linux, built with Rust and SDL2.

## About

LMahjong is a classic tile-matching solitaire game featuring Tux penguin-themed graphics. Clear all 144 tiles from the board by matching pairs of free tiles. The game uses the traditional Turtle layout with 5 stacked layers, and every generated board is guaranteed to be solvable.

![Initial board](assets/initial-board.png)

### Features

- Classic Turtle layout with 144 tiles across 5 layers
- Guaranteed solvable boards via reverse-deal generation
- Hint system, undo (up to 10 moves), and shuffle (up to 3 per game)
- Timer and scoring system with local leaderboard (top 10)
- Keyboard shortcuts for all actions
- Audio feedback with mute support
- Resizable window (min 1920×1080, adapts to screen resolution)
- Native Linux packages (.deb, .rpm, AppImage)

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

## Packaging (.deb, .rpm, AppImage)

A packaging script is included to create distribution packages:

```bash
# Build all three formats
./package.sh all

# Or build individually
./package.sh deb
./package.sh rpm
./package.sh appimage
```

Output goes to `target/package/`.

### Prerequisites for packaging

| Format | Tool needed | Install with |
|--------|-------------|--------------|
| .deb | dpkg-deb | Pre-installed on Debian/Ubuntu |
| .rpm | rpmbuild | `sudo dnf install rpm-build` |
| AppImage | wget | Pre-installed on most distros (downloads appimagetool automatically) |

### Installing the packages

**.deb (Ubuntu/Debian):**
```bash
sudo apt install ./target/package/lmahjong_0.1.0_amd64.deb
```
This automatically installs SDL2 runtime dependencies via apt.

**.rpm (Fedora/RHEL):**
```bash
sudo dnf install ./target/package/lmahjong-0.1.0-1.x86_64.rpm
```
This automatically installs SDL2 runtime dependencies via dnf.

**AppImage (any distro):**
```bash
chmod +x target/package/lmahjong-0.1.0-x86_64.AppImage
./target/package/lmahjong-0.1.0-x86_64.AppImage
```
AppImages bundle SDL2 libraries inside, so no system dependencies are needed.

## Assets

![Tux Penguin Sprite Sheet](sprite/tux-sprite-lmahjong.png)

## Data Storage

- **Leaderboard:** `~/.local/share/lmahjong/leaderboard.json`
- **Settings:** `~/.local/share/lmahjong/settings.json`
- **Saved game:** `~/.local/share/lmahjong/savegame.json`

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
```

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).
