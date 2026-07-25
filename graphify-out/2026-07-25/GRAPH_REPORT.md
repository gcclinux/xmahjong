# Graph Report - .  (2026-07-25)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 470 nodes · 1213 edges · 20 communities (17 shown, 3 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 54 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `3ea21ba5`
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
- `make_state()` --references--> `Board`  [EXTRACTED]
  tests/logic_properties.rs → src/board.rs
- `make_state()` --references--> `Board`  [EXTRACTED]
  tests/shuffle_properties.rs → src/board.rs

## Import Cycles
- 2-file cycle: `src/game_state.rs -> src/logic.rs -> src/game_state.rs`

## Communities (20 total, 3 thin omitted)

### Community 0 - "turtle_layout"
Cohesion: 0.07
Nodes (64): blocking_relations_computed_correctly(), BlockingRelation, board_free_tiles_empty_board(), board_free_tiles_only_top_layer(), board_free_tiles_updates_after_removal(), board_is_free_blocked_from_above(), board_is_free_blocked_on_both_sides(), board_is_free_returns_false_for_empty_position() (+56 more)

### Community 1 - "Renderer"
Cohesion: 0.10
Nodes (26): Canvas, Color, Font, Point, Rect, Sdl, Sdl2TtfContext, assets_path() (+18 more)

### Community 2 - "GameState"
Cohesion: 0.06
Nodes (42): Animation, Difficulty, GameState, GameStatus, HintState, name_entry_is_valid_checks_1_to_20_chars(), name_entry_new_creates_empty_state(), name_entry_pop_char_removes_last() (+34 more)

### Community 3 - "storage.rs"
Cohesion: 0.08
Nodes (18): PathBuf, default_difficulty_str(), dirs_fallback(), insert_only_stores_the_last_entry(), Leaderboard, LeaderboardEntry, qualifies_always_returns_true(), Default (+10 more)

### Community 4 - "timer.rs"
Cohesion: 0.13
Nodes (26): default_is_same_as_new(), elapsed_seconds_includes_current_running_segment(), elapsed_seconds_returns_whole_seconds(), elapsed_seconds_zero_when_new(), format_display_large_time(), format_display_minutes_and_seconds(), format_display_over_one_hour(), format_display_seconds_only() (+18 more)

### Community 5 - "Board"
Cohesion: 0.18
Nodes (21): Board, Layout, Option, all_tiles_have_valid_positions(), BoardGenerator, different_seeds_produce_different_boards(), each_face_id_appears_exactly_4_times(), face_ids_are_in_valid_range() (+13 more)

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
Cohesion: 0.41
Nodes (12): compute_layout_rect(), compute_thickness(), hit_test(), LayoutMetrics, tile_screen_rect(), hit_test_boundary_pixels(), hit_test_center_of_tile_returns_correct_index(), hit_test_empty_area_returns_none() (+4 more)

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

## Knowledge Gaps
- **2 isolated node(s):** `bump_version.sh script`, `run.sh script`
  These have ≤1 connection - possible missing edges or undocumented components.
- **3 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Renderer` connect `Renderer` to `tile_screen_rect`, `GameState`?**
  _High betweenness centrality (0.146) - this node is a cross-community bridge._
- **Why does `GameTimer` connect `timer.rs` to `turtle_layout`, `tile_screen_rect`, `GameState`, `logic_properties.rs`?**
  _High betweenness centrality (0.133) - this node is a cross-community bridge._
- **Why does `Board` connect `Board` to `turtle_layout`, `GameState`, `logic_properties.rs`?**
  _High betweenness centrality (0.104) - this node is a cross-community bridge._
- **Are the 34 inferred relationships involving `turtle_layout()` (e.g. with `all_tiles_have_valid_positions()` and `different_seeds_produce_different_boards()`) actually correct?**
  _`turtle_layout()` has 34 INFERRED edges - model-reasoned connections that need verification._
- **What connects `bump_version.sh script`, `run.sh script` to the rest of the system?**
  _2 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `turtle_layout` be split into smaller, more focused modules?**
  _Cohesion score 0.07315315315315316 - nodes in this community are weakly interconnected._
- **Should `Renderer` be split into smaller, more focused modules?**
  _Cohesion score 0.09507042253521127 - nodes in this community are weakly interconnected._