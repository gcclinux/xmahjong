# Level Design

LMahjong has 20 levels split into two phases: **Penguin Phase** (1-10) and **Dog Phase** (11-20).

## Penguin Phase (Levels 1-10)

Levels 1-10 use only penguin tile faces (face IDs 0-49). Tile count increases each level.

| Level | Tiles | Pairs | Faces Used | Theme |
|-------|-------|-------|------------|-------|
| 1 | 36 | 18 | 9 | Penguins only |
| 2 | 48 | 24 | 12 | Penguins only |
| 3 | 60 | 30 | 15 | Penguins only |
| 4 | 72 | 36 | 18 | Penguins only |
| 5 | 84 | 42 | 21 | Penguins only |
| 6 | 96 | 48 | 24 | Penguins only |
| 7 | 108 | 54 | 27 | Penguins only |
| 8 | 120 | 60 | 30 | Penguins only |
| 9 | 132 | 66 | 33 | Penguins only |
| 10 | 144 | 72 | 36 | Penguins only (full board) |

## Dog Phase (Levels 11-20)

Levels 11-20 repeat the same tile count progression but mix in dog tile faces from `assets/dogs/`. Each level introduces one additional dog style (10 face variants per style) until 5 styles are active.

| Level | Tiles | Pairs | Dog Styles | Face Pool Size | Theme |
|-------|-------|-------|------------|----------------|-------|
| 11 | 36 | 18 | 1 (faces 50-59) | 60 | Penguins + Dogs |
| 12 | 48 | 24 | 2 (faces 50-69) | 70 | Penguins + Dogs |
| 13 | 60 | 30 | 3 (faces 50-79) | 80 | Penguins + Dogs |
| 14 | 72 | 36 | 4 (faces 50-89) | 90 | Penguins + Dogs |
| 15 | 84 | 42 | 5 (faces 50-99) | 100 | Penguins + Dogs |
| 16 | 96 | 48 | 5 (faces 50-99) | 100 | Penguins + Dogs |
| 17 | 108 | 54 | 5 (faces 50-99) | 100 | Penguins + Dogs |
| 18 | 120 | 60 | 5 (faces 50-99) | 100 | Penguins + Dogs |
| 19 | 132 | 66 | 5 (faces 50-99) | 100 | Penguins + Dogs |
| 20 | 144 | 72 | 5 (faces 50-99) | 100 | Penguins + Dogs (full board) |

## How It Works

- **Tile count** must always be a multiple of 4 (each face ID needs exactly 4 tiles to form 2 matchable pairs).
- **Faces Used** = Tiles / 4 (the number of distinct face images on the board).
- **Face Pool** is the set of face IDs the generator randomly picks from. A larger pool means more visual variety.
- The generator uses the full 144-position turtle layout for all levels. For levels with fewer tiles, it removes positions from the outside in before placing tiles, keeping the board compact and playable.

## Progression

- Completing a level shows a "NEXT LEVEL" button on the victory screen (up to level 20).
- "NEW GAME" always resets to level 1.
- At level 20 (max), only "NEW GAME" and "LEADERBOARD" buttons are shown.
- The current level is displayed on the HUD between Score and Shuffle.
- Level is persisted in save files so quitting mid-game resumes at the correct level.
