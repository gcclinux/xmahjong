# Graph Report - xmahjong  (2026-07-25)

## Corpus Check
- 31 files · ~1,434,323 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 509 nodes · 1250 edges · 22 communities (19 shown, 3 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 54 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `7684464a`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- turtle_layout
- Renderer
- GameState
- storage.rs
- timer.rs
- Board
- input.rs
- AudioManager
- package.ps1
- tile_screen_rect
- package_macos.sh
- extract_tiles.py
- logic_properties.rs
- package.sh
- build.rs
- bump_version.sh script
- run.sh script
- Level Design
- partial_board_strategy

## God Nodes (most connected - your core abstractions)
1. `Renderer` - 59 edges
2. `turtle_layout()` - 53 edges
3. `GameState` - 35 edges
4. `Board` - 32 edges
5. `handler()` - 24 edges
6. `make_state()` - 24 edges
7. `GameTimer` - 21 edges
8. `Layout` - 19 edges
9. `AudioManager` - 18 edges
10. `storage_dir()` - 15 edges

## Surprising Connections (you probably didn't know these)
- `create_full_board()` --calls--> `turtle_layout()`  [INFERRED]
  tests/board_properties.rs → src/board.rs
- `hit_test_removed_tile_returns_none()` --calls--> `turtle_layout()`  [INFERRED]
  tests/renderer_properties.rs → src/board.rs
- `make_test_game_state()` --calls--> `turtle_layout()`  [INFERRED]
  tests/renderer_properties.rs → src/board.rs
- `create_full_board()` --references--> `Board`  [EXTRACTED]
  tests/board_properties.rs → src/board.rs
- `partial_board_strategy()` --references--> `Board`  [EXTRACTED]
  tests/board_properties.rs → src/board.rs

## Import Cycles
- 2-file cycle: `src/game_state.rs -> src/logic.rs -> src/game_state.rs`

## Communities (22 total, 3 thin omitted)

### Community 0 - "turtle_layout"
Cohesion: 0.13
Nodes (44): turtle_layout(), check_game_over(), check_game_over_board_empty_returns_won(), check_game_over_no_valid_pairs_returns_lost(), check_game_over_valid_pairs_exist_returns_none(), free_tiles_recalculated_after_match(), GameOverReason, HintResult (+36 more)

### Community 1 - "Renderer"
Cohesion: 0.08
Nodes (39): Canvas, Color, Font, Point, Rect, Sdl, Sdl2TtfContext, TilePosition (+31 more)

### Community 2 - "GameState"
Cohesion: 0.06
Nodes (41): Animation, Difficulty, GameState, GameStatus, HintState, name_entry_is_valid_checks_1_to_20_chars(), name_entry_new_creates_empty_state(), name_entry_pop_char_removes_last() (+33 more)

### Community 3 - "storage.rs"
Cohesion: 0.08
Nodes (18): PathBuf, default_difficulty_str(), dirs_fallback(), insert_only_stores_the_last_entry(), Leaderboard, LeaderboardEntry, qualifies_always_returns_true(), Default (+10 more)

### Community 4 - "timer.rs"
Cohesion: 0.13
Nodes (26): default_is_same_as_new(), elapsed_seconds_includes_current_running_segment(), elapsed_seconds_returns_whole_seconds(), elapsed_seconds_zero_when_new(), format_display_large_time(), format_display_minutes_and_seconds(), format_display_over_one_hour(), format_display_seconds_only() (+18 more)

### Community 5 - "Board"
Cohesion: 0.10
Nodes (36): blocking_relations_computed_correctly(), BlockingRelation, Board, board_free_tiles_empty_board(), board_free_tiles_only_top_layer(), board_free_tiles_updates_after_removal(), board_is_free_blocked_from_above(), board_is_free_blocked_on_both_sides() (+28 more)

### Community 6 - "input.rs"
Cohesion: 0.12
Nodes (28): Event, ctrl_m_maps_to_toggle_mute(), ctrl_n_maps_to_new_game(), ctrl_p_maps_to_pause_menu(), ctrl_q_maps_to_save_quit(), ctrl_r_maps_to_resume(), ctrl_s_maps_to_save(), escape_when_paused_maps_to_resume() (+20 more)

### Community 7 - "AudioManager"
Cohesion: 0.18
Nodes (14): Chunk, AudioManager, make_test_manager(), Default, Option, Self, test_initial_mute_state_is_false(), test_play_methods_do_not_panic_when_muted() (+6 more)

### Community 8 - "package.ps1"
Cohesion: 0.31
Nodes (17): Build-Msi(), Build-Msix(), Build-Portable(), ConvertTo-LicenseRtf(), Download-File(), Expand-Zip(), Find-SDL2SubDir(), Find-WindowsSdkTool() (+9 more)

### Community 9 - "tile_screen_rect"
Cohesion: 0.08
Nodes (25): About, Assets, Building, Controls, Data Storage, Difficulty, Features, Files (+17 more)

### Community 10 - "package_macos.sh"
Cohesion: 0.27
Nodes (8): build_app(), build_dmg(), build_release(), build_zip(), bundle_dylibs(), main(), rewrite_deps(), package_macos.sh script

### Community 11 - "extract_tiles.py"
Cohesion: 0.29
Nodes (9): extract_cell_to_tile(), find_transparent_gaps_horizontal(), find_transparent_gaps_vertical(), main(), Validate extracted tile is a proper image with content., Find horizontal bands of fully transparent pixels (row separators)., Find vertical bands of transparent pixels within a row range., Extract a cell from the sprite and fit it into a 256x256 transparent canvas. (+1 more)

### Community 12 - "logic_properties.rs"
Cohesion: 0.27
Nodes (6): arb_difficulty(), arb_saved_game(), make_state(), Strategy, String, Value

### Community 13 - "package.sh"
Cohesion: 0.50
Nodes (7): build_appimage(), build_deb(), build_rpm(), prepare_staging(), package.sh script, build_release(), main()

### Community 20 - "Level Design"
Cohesion: 0.14
Nodes (12): Difficulty, Dog Phase (Levels 11-20), Endgame Phase (Levels 51-100), How It Works, How It Works, Level Design, Penguin Phase (Levels 1-10), Persistence (+4 more)

### Community 21 - "partial_board_strategy"
Cohesion: 0.60
Nodes (5): create_full_board(), full_board_face_assignment(), partial_board_strategy(), Strategy, Value

## Knowledge Gaps
- **30 isolated node(s):** `Features`, `System Dependencies`, `Rust Toolchain`, `Building`, `Running` (+25 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **3 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Renderer` connect `Renderer` to `GameState`?**
  _High betweenness centrality (0.131) - this node is a cross-community bridge._
- **Why does `GameTimer` connect `timer.rs` to `turtle_layout`, `Renderer`, `GameState`, `logic_properties.rs`?**
  _High betweenness centrality (0.118) - this node is a cross-community bridge._
- **Why does `Board` connect `Board` to `turtle_layout`, `GameState`, `logic_properties.rs`, `partial_board_strategy`?**
  _High betweenness centrality (0.092) - this node is a cross-community bridge._
- **Are the 34 inferred relationships involving `turtle_layout()` (e.g. with `all_tiles_have_valid_positions()` and `different_seeds_produce_different_boards()`) actually correct?**
  _`turtle_layout()` has 34 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Features`, `System Dependencies`, `Rust Toolchain` to the rest of the system?**
  _30 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `turtle_layout` be split into smaller, more focused modules?**
  _Cohesion score 0.12727272727272726 - nodes in this community are weakly interconnected._
- **Should `Renderer` be split into smaller, more focused modules?**
  _Cohesion score 0.08019246190858059 - nodes in this community are weakly interconnected._