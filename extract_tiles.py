#!/usr/bin/env python3
"""
Extract individual penguin tile images from the sprite sheet.

Uses the alpha-channel sprite (tux-sprite-lmahjong.png) to detect the grid
boundaries via transparent gaps between images, then extracts each image
cell, scales it to fit in a 256x256 canvas, and centers it with a
transparent (alpha=0) border.

The sprite has a title row at the top (skipped), then a grid of penguin
scene images separated by transparent gap lines.
"""

import os
import sys
from PIL import Image

SPRITE_PATH = "sprite/tux-sprite-lmahjong.png"
OUTPUT_DIR = "assets/tiles"
TILE_SIZE = 256


def find_transparent_gaps_horizontal(alpha_channel, width, height, min_gap=5):
    """Find horizontal bands of fully transparent pixels (row separators)."""
    gaps = []
    in_gap = False
    gap_start = 0

    for y in range(height):
        # Sample across the width
        samples = [alpha_channel.getpixel((x, y)) for x in range(0, width, 8)]
        is_transparent = all(s < 10 for s in samples)

        if is_transparent and not in_gap:
            gap_start = y
            in_gap = True
        elif not is_transparent and in_gap:
            if y - gap_start >= min_gap:
                gaps.append((gap_start, y - 1))
            in_gap = False

    if in_gap and height - gap_start >= min_gap:
        gaps.append((gap_start, height - 1))

    return gaps


def find_transparent_gaps_vertical(alpha_channel, width, height, y_start, y_end, min_gap=5):
    """Find vertical bands of transparent pixels within a row range."""
    gaps = []
    in_gap = False
    gap_start = 0

    for x in range(width):
        # Sample within the row range
        samples = [alpha_channel.getpixel((x, y)) for y in range(y_start, y_end, 8)]
        is_transparent = all(s < 10 for s in samples)

        if is_transparent and not in_gap:
            gap_start = x
            in_gap = True
        elif not is_transparent and in_gap:
            if x - gap_start >= min_gap:
                gaps.append((gap_start, x - 1))
            in_gap = False

    if in_gap and width - gap_start >= min_gap:
        gaps.append((gap_start, width - 1))

    return gaps


def extract_cell_to_tile(img, x_start, y_start, x_end, y_end):
    """Extract a cell from the sprite and fit it into a 256x256 transparent canvas."""
    # Crop the cell
    cell = img.crop((x_start, y_start, x_end, y_end))
    cell_w, cell_h = cell.size

    # Scale to fit within 256x256 maintaining aspect ratio
    scale = min(TILE_SIZE / cell_w, TILE_SIZE / cell_h)
    if scale < 1.0:
        new_w = int(cell_w * scale)
        new_h = int(cell_h * scale)
        cell = cell.resize((new_w, new_h), Image.LANCZOS)
    else:
        new_w, new_h = cell_w, cell_h

    # Create transparent canvas and center the image
    canvas = Image.new("RGBA", (TILE_SIZE, TILE_SIZE), (0, 0, 0, 0))
    offset_x = (TILE_SIZE - new_w) // 2
    offset_y = (TILE_SIZE - new_h) // 2

    # Ensure source is RGBA for proper pasting
    if cell.mode != "RGBA":
        cell = cell.convert("RGBA")

    canvas.paste(cell, (offset_x, offset_y), cell)
    return canvas


def validate_tile(tile_img, idx):
    """Validate extracted tile is a proper image with content."""
    alpha = tile_img.split()[3]
    total_pixels = TILE_SIZE * TILE_SIZE
    
    # Count opaque pixels
    opaque_count = 0
    for y in range(TILE_SIZE):
        for x in range(TILE_SIZE):
            if alpha.getpixel((x, y)) > 10:
                opaque_count += 1
    
    fill_ratio = opaque_count / total_pixels

    # Must have substantial content (at least 30% filled)
    if fill_ratio < 0.30:
        return False, f"Too little content ({fill_ratio*100:.1f}% fill)"

    # Must not be 100% opaque (should have transparent border)
    if fill_ratio > 0.99:
        return False, f"No transparent border ({fill_ratio*100:.1f}% fill)"

    # Check bounding box exists and is reasonably centered
    bbox = tile_img.getbbox()
    if not bbox:
        return False, "No content"

    # Content should be centered (within 20% of center)
    center_x = (bbox[0] + bbox[2]) / 2
    center_y = (bbox[1] + bbox[3]) / 2
    x_off = abs(center_x - TILE_SIZE/2) / TILE_SIZE
    y_off = abs(center_y - TILE_SIZE/2) / TILE_SIZE

    if x_off > 0.2 or y_off > 0.2:
        return False, f"Content not centered (x_off={x_off:.2f}, y_off={y_off:.2f})"

    return True, "OK"


def main():
    # Load sprite
    sprite = Image.open(SPRITE_PATH)
    alpha = sprite.split()[3]
    print(f"Loaded sprite: {sprite.width}x{sprite.height} ({sprite.mode})")

    # Find horizontal gaps (row separators)
    h_gaps = find_transparent_gaps_horizontal(alpha, sprite.width, sprite.height)
    print(f"\nFound {len(h_gaps)} horizontal gaps:")
    for s, e in h_gaps:
        print(f"  y={s}-{e} (height {e-s+1})")

    # Derive image row ranges (between consecutive gaps)
    row_ranges = []
    # First row starts after the first gap (which is below the title)
    for i in range(len(h_gaps) - 1):
        row_start = h_gaps[i][1] + 1
        row_end = h_gaps[i+1][0] - 1
        if row_end - row_start > 50:  # Skip tiny strips
            row_ranges.append((row_start, row_end))

    # Last row: after last gap to bottom (if content exists)
    if h_gaps:
        last_start = h_gaps[-1][1] + 1
        if last_start < sprite.height - 50:
            row_ranges.append((last_start, sprite.height - 1))

    print(f"\nDerived {len(row_ranges)} image rows:")
    for i, (s, e) in enumerate(row_ranges):
        print(f"  Row {i}: y={s}-{e} (height {e-s+1})")

    # For each row, find vertical gaps to get column boundaries
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    tile_index = 0
    all_results = []

    for row_idx, (row_start, row_end) in enumerate(row_ranges):
        # Find vertical gaps in this row
        v_gaps = find_transparent_gaps_vertical(
            alpha, sprite.width, sprite.height, row_start, row_end
        )

        # Derive column ranges
        col_ranges = []
        # Before first gap
        if v_gaps and v_gaps[0][0] > 10:
            col_ranges.append((0, v_gaps[0][0] - 1))
        # Between consecutive gaps
        for i in range(len(v_gaps) - 1):
            col_start = v_gaps[i][1] + 1
            col_end = v_gaps[i+1][0] - 1
            if col_end - col_start > 50:
                col_ranges.append((col_start, col_end))
        # After last gap
        if v_gaps and v_gaps[-1][1] < sprite.width - 10:
            col_ranges.append((v_gaps[-1][1] + 1, sprite.width - 1))

        print(f"\n  Row {row_idx}: {len(col_ranges)} images")

        for col_idx, (col_start, col_end) in enumerate(col_ranges):
            # Extract and scale to 256x256
            tile = extract_cell_to_tile(sprite, col_start, row_start, col_end, row_end)

            # Validate
            is_valid, msg = validate_tile(tile, tile_index)

            # Save
            output_path = os.path.join(OUTPUT_DIR, f"face_{tile_index:02d}.png")
            tile.save(output_path, "PNG")

            status = "OK" if is_valid else f"WARNING: {msg}"
            print(f"    [{tile_index:2d}] {output_path} ({col_end-col_start+1}x{row_end-row_start+1} -> 256x256) - {status}")
            all_results.append((tile_index, is_valid, msg))
            tile_index += 1

    # Summary
    valid = sum(1 for _, v, _ in all_results if v)
    invalid = sum(1 for _, v, _ in all_results if not v)
    print(f"\n{'='*60}")
    print(f"Total images extracted: {tile_index}")
    print(f"  Valid: {valid}")
    print(f"  Issues: {invalid}")

    if invalid > 0:
        print("\nImages with issues:")
        for idx, v, msg in all_results:
            if not v:
                print(f"  face_{idx:02d}.png: {msg}")

    print(f"\nAll output tiles are {TILE_SIZE}x{TILE_SIZE} RGBA with transparent padding.")
    
    if tile_index != 36:
        print(f"\nNOTE: Found {tile_index} images (expected 36).")
        print("You may need to select which 36 to use for the game.")


if __name__ == "__main__":
    main()
