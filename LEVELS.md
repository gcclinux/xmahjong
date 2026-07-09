# Level Design

xMahjong has 50 levels split into three phases: **Penguin Phase** (1-10), **Dog Phase** (11-20), and **Space Phase** (21-50).

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

## Space Phase (Levels 21-50)

Levels 21-50 mix tiles from all three themes: penguin (face IDs 0-49), dog (face IDs 50-99), and space (face IDs 100-149). The face pool grows linearly from 100 at level 21 to 200 at level 50, and the tile count cycles through the same 36-to-144 progression every 10 levels.

| Level | Tiles | Pairs | Face Pool Size | Penguin | Dog | Space | Theme |
|-------|-------|-------|----------------|---------|-----|-------|-------|
| 21 | 36 | 18 | 100 | 33 | 33 | 34 | Penguins + Dogs + Space |
| 22 | 48 | 24 | 103 | 34 | 34 | 35 | Penguins + Dogs + Space |
| 23 | 60 | 30 | 106 | 35 | 35 | 36 | Penguins + Dogs + Space |
| 24 | 72 | 36 | 110 | 36 | 36 | 38 | Penguins + Dogs + Space |
| 25 | 84 | 42 | 113 | 37 | 37 | 39 | Penguins + Dogs + Space |
| 26 | 96 | 48 | 117 | 39 | 39 | 39 | Penguins + Dogs + Space |
| 27 | 108 | 54 | 120 | 40 | 40 | 40 | Penguins + Dogs + Space |
| 28 | 120 | 60 | 124 | 41 | 41 | 42 | Penguins + Dogs + Space |
| 29 | 132 | 66 | 127 | 42 | 42 | 43 | Penguins + Dogs + Space |
| 30 | 144 | 72 | 131 | 43 | 43 | 45 | Penguins + Dogs + Space (full board) |
| 31 | 36 | 18 | 134 | 44 | 44 | 46 | Penguins + Dogs + Space |
| 32 | 48 | 24 | 137 | 45 | 45 | 47 | Penguins + Dogs + Space |
| 33 | 60 | 30 | 141 | 47 | 47 | 47 | Penguins + Dogs + Space |
| 34 | 72 | 36 | 144 | 48 | 48 | 48 | Penguins + Dogs + Space |
| 35 | 84 | 42 | 148 | 49 | 49 | 50 | Penguins + Dogs + Space |
| 36 | 96 | 48 | 151 | 50 | 50 | 51 | Penguins + Dogs + Space |
| 37 | 108 | 54 | 155 | 51 | 51 | 53 | Penguins + Dogs + Space |
| 38 | 120 | 60 | 158 | 52 | 52 | 54 | Penguins + Dogs + Space |
| 39 | 132 | 66 | 162 | 54 | 54 | 54 | Penguins + Dogs + Space |
| 40 | 144 | 72 | 165 | 55 | 55 | 55 | Penguins + Dogs + Space (full board) |
| 41 | 36 | 18 | 168 | 56 | 56 | 56 | Penguins + Dogs + Space |
| 42 | 48 | 24 | 172 | 57 | 57 | 58 | Penguins + Dogs + Space |
| 43 | 60 | 30 | 175 | 58 | 58 | 59 | Penguins + Dogs + Space |
| 44 | 72 | 36 | 179 | 59 | 59 | 61 | Penguins + Dogs + Space |
| 45 | 84 | 42 | 182 | 60 | 60 | 62 | Penguins + Dogs + Space |
| 46 | 96 | 48 | 186 | 62 | 62 | 62 | Penguins + Dogs + Space |
| 47 | 108 | 54 | 189 | 63 | 63 | 63 | Penguins + Dogs + Space |
| 48 | 120 | 60 | 193 | 64 | 64 | 65 | Penguins + Dogs + Space |
| 49 | 132 | 66 | 196 | 65 | 65 | 66 | Penguins + Dogs + Space |
| 50 | 144 | 72 | 200 | 66 | 66 | 68 | Penguins + Dogs + Space (full board) |

The face pool size grows linearly using the formula `pool_size = 100 + ((level - 21) * 100) / 29` (integer division), interpolating from 100 face IDs at level 21 to 200 at level 50. The pool is distributed as evenly as possible across the three tile sets: `floor(pool_size / 3)` IDs go to penguins, `floor(pool_size / 3)` to dogs, and the remainder to space. Face IDs are selected contiguously from the start of each set's range (penguin from 0, dog from 50, space from 100). The tile count follows the same 10-level cycle as earlier phases (36, 48, 60, 72, 84, 96, 108, 120, 132, 144), repeating three times across levels 21-30, 31-40, and 41-50. Face IDs wrap modulo 50 within each tile set, so levels with per-set counts above 50 will see repeated tile faces in the pool.

## How It Works

- **Tile count** must always be a multiple of 4 (each face ID needs exactly 4 tiles to form 2 matchable pairs).
- **Faces Used** = Tiles / 4 (the number of distinct face images on the board).
- **Face Pool** is the set of face IDs the generator randomly picks from. A larger pool means more visual variety.
- The generator uses the full 144-position turtle layout for all levels. For levels with fewer tiles, it removes positions from the outside in before placing tiles, keeping the board compact and playable.

## Progression

- Completing a level shows a "NEXT LEVEL" button on the victory screen (up to level 50).
- "NEW GAME" always resets to level 1.
- At level 50 (max), only "NEW GAME" and "LEADERBOARD" buttons are shown.
- The current level is displayed on the HUD between Score and Shuffle.
- Level is persisted in save files so quitting mid-game resumes at the correct level.
