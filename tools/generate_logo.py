#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["pillow>=11.0"]
# ///
"""Generate the FrameWork logo: a data frame whose cells spell an FW monogram.

One set of geometry constants drives every output, so the vector and raster
masters cannot drift apart:

  - assets/icon.svg     vector master of the app icon (Icon Composer source)
  - assets/icon.png     square raster master, drawn with Pillow rather than
                        rasterized, so the build needs no SVG renderer
  - public/icon.svg     the same logo cropped to its tile, for the app's header
  - public/favicon.svg  a tab-sized mark, served as-is by Vite
  - public/favicon.ico  16/32/48 rasters of that mark, for browsers that ask

A browser tab has fewer pixels across than the monogram has cells, so the
favicon does not try to shrink the frame. It drops it and draws FW as plain
letterforms in the same palette.

Usage:
    tools/generate_logo.py [--svg FILE] [--png FILE] [--png-size N]
                           [--favicon-svg FILE] [--favicon-ico FILE] [--seed N]

After regenerating, refresh the bundled Tauri icon set from the raster master:

    npx tauri icon assets/icon.png
"""

from __future__ import annotations

import argparse
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

REPO_ROOT = Path(__file__).resolve().parent.parent

SIZE = 512

# Palette. There is exactly one green in the frame; every green you see is some
# coverage of it over a neutral grey. The cells were manila first, and warm
# paper fought the green -- against grey the same green carries much further.
# The tile stays sage: on a white one the icon has no outline left on a light
# background, only its drop shadow.
SAGE = "#9FB6A4"  # the tile the mark sits on
SAGE_EDGE = "#8AA48F"
PLATE = "#CFD5D0"  # gutters between cells, i.e. the frame itself
SHADE = "#B6BEB8"  # gutter in shadow, which is what makes the cells look raised
CELL = "#F1F4F1"  # light grey: what every cell starts as
GREEN = "#2F5540"  # a deeper cousin of the app's --accent #315c49
HEADER = "#94A097"  # muted, so the band never competes with a row of cells

# Cells are built in layers, each one over the last: grey, then a light wash of
# green at a random strength, then -- on the letters only -- a heavy coat. The
# letters inherit whatever wash landed under them, so they vary a little
# without any rule of their own.
WASH_ALPHA = (0.0, 0.20)
TEXT_ALPHA = 0.80

# Rounded tile, on Apple's icon grid: the body fills 824 of a 1024pt canvas and
# its corner radius is 22.5% of that. Measuring the shipped icons of Notes,
# Reminders and Maps puts their bodies at 0.797-0.805 of the canvas; ours was
# 0.875, which left the Cmd-Tab highlight no ring to draw in. The radius runs a
# little over Apple's 0.225 because their corner is a continuous curve and this
# is a plain arc, which reads tighter at the same nominal radius.
TILE_SIZE = SIZE * 824 / 1024
TILE_X = TILE_Y = (SIZE - TILE_SIZE) / 2
TILE_RADIUS = TILE_SIZE * 0.24
TILE_STROKE = 4
SHADOW_COLOR = "#0F1A14"
SHADOW_OPACITY = 0.18
SHADOW_DY = 6
SHADOW_BLUR = 9

# Frame geometry. The plate is centred by construction and held back from the
# tile by TABLE_INSET. A near-square plate inside a heavily rounded tile crowds
# at the diagonals long before it does along the edges, so the inset has to be
# read against the corner, not against the straight sides.
# These were tuned against a larger tile, then scaled with it so the
# composition inside the icon did not change when the icon did.
TABLE_INSET = 37
TABLE_W = TILE_SIZE - 2 * TABLE_INSET
TABLE_X = TILE_X + TABLE_INSET
TABLE_RADIUS = 31  # rounder than a frame needs, to answer the tile's curve
# The plate is also the frame's border, so its exposed perimeter has to survive
# the app icon being reduced to 32px. At 15 units it collapsed to a single
# antialiased pixel; 20 keeps a deliberate rim without taking a cell's worth of
# width away from the monogram.
OUTER_BORDER = 20
# The monogram fills the frame body. Earlier versions continued the grid with
# three scattered rows below the letters, but that made the mark bottom-heavy:
# the eye read the data tail as a second subject instead of part of the FW.
COLS, GLYPH_ROWS = 10, 6
BODY_ROWS = GLYPH_ROWS
# Tight gutters: at 32px an 8px gutter chews a fifth off every letter stroke and
# the monogram falls apart, so the grid reads by colour more than by spacing.
GAP_X, GAP_Y = 4.5, 4.5
BODY_CELL_H = 27.5
# Give the header a full cell-height face, held between half-border gutters.
# Tying these dimensions together keeps the header substantial at small sizes
# without turning it into a second, oversized row.
HEADER_H = BODY_CELL_H + OUTER_BORDER
CELL_RADIUS = 4.5
CELL_LIFT = 2  # how far each cell's shadow falls, i.e. the depth of the emboss

INNER_X = TABLE_X + OUTER_BORDER
INNER_W = TABLE_W - 2 * OUTER_BORDER
CELL_W = (INNER_W - GAP_X * (COLS - 1)) / COLS
TABLE_H = HEADER_H + BODY_ROWS * BODY_CELL_H + (BODY_ROWS - 1) * GAP_Y + OUTER_BORDER
TABLE_Y = TILE_Y + (TILE_SIZE - TABLE_H) / 2
BODY_Y = TABLE_Y + HEADER_H

CELL_SEED = 5  # picked by eye from a handful; --seed tries other colour washes

# Letter-cell patterns. The F's four-column arms give it enough width to
# balance the five-column W; the fifth column is intentional whitespace.
F_PATTERN = (
    "1111",
    "1000",
    "1111",
    "1000",
    "1000",
    "1000",
)

# The split lower strokes give the W a distinctive, steep zig-zag finish. The
# centre peak has to climb into the top half: kept to the bottom two rows, the
# upper half is just two verticals and the whole thing reads as a U.
W_PATTERN = (
    "10001",
    "10101",
    "10101",
    "10101",
    "10101",
    "01010",
)


def cell_is_letter(row: int, col: int) -> bool:
    if col < 4:
        return F_PATTERN[row][col] == "1"
    if col == 4:
        return False
    return W_PATTERN[row][col - 5] == "1"


def _mix(base: str, top: str, t: float) -> str:
    """Blend two hex colours. The faded rows sit on a known solid gutter, so
    they can be flattened here instead of relying on per-renderer alpha."""
    t = max(0.0, min(1.0, t))
    channels = (
        round(
            int(base[i : i + 2], 16)
            + (int(top[i : i + 2], 16) - int(base[i : i + 2], 16)) * t
        )
        for i in (1, 3, 5)
    )
    return "#" + "".join(f"{c:02X}" for c in channels)


def body_cells(seed: int = CELL_SEED) -> list[tuple[float, float, str, str, str]]:
    """Every drawn cell as (x, y, fill, shadow, role), seeded so runs match.

    Each cell is grey, then a wash of green at a random strength, then a
    heavy coat of the same green if it is part of a letter -- one stack, so a
    letter carries the wash that landed beneath it.
    """
    rng = random.Random(seed)

    cells = []
    for row in range(BODY_ROWS):
        for col in range(COLS):
            x = INNER_X + col * (CELL_W + GAP_X)
            y = BODY_Y + row * (BODY_CELL_H + GAP_Y)

            fill = _mix(CELL, GREEN, rng.uniform(*WASH_ALPHA))
            letter = cell_is_letter(row, col)
            if letter:
                fill = _mix(fill, GREEN, TEXT_ALPHA)
            cells.append((x, y, fill, SHADE, "letter-cell" if letter else "data-cell"))
    return cells


def make_svg(seed: int = CELL_SEED, crop: bool = False) -> str:
    """The mark. Cropped, the viewBox is the tile itself and the cast shadow is
    dropped: the padding and shadow exist to put the app icon on Apple's grid,
    and in the app's own header they would just shrink the logo and blur it."""
    cells = body_cells(seed)
    # Each cell is drawn twice: a shaded copy pushed down by CELL_LIFT, then the
    # face on top of it. That one offset is the whole emboss.
    faces = "".join(
        f'<rect class="cell-shadow" x="{x:g}" y="{y + CELL_LIFT:g}" '
        f'width="{CELL_W:g}" height="{BODY_CELL_H}" rx="{CELL_RADIUS}" fill="{shadow}"/>'
        for x, y, _, shadow, _ in cells
    ) + "".join(
        f'<rect class="{role}" x="{x:g}" y="{y:g}" '
        f'width="{CELL_W:g}" height="{BODY_CELL_H}" rx="{CELL_RADIUS}" fill="{fill}"/>'
        for x, y, fill, _, role in cells
    )

    view = (
        f"{TILE_X:g} {TILE_Y:g} {TILE_SIZE:g} {TILE_SIZE:g}"
        if crop
        else f"0 0 {SIZE} {SIZE}"
    )
    edge = TILE_SIZE if crop else SIZE
    tile_shadow = "" if crop else ' filter="url(#tile-shadow)"'

    return f'''<?xml version="1.0" encoding="UTF-8"?>
<!-- Generated by tools/generate_logo.py; edit that, not this. -->
<svg xmlns="http://www.w3.org/2000/svg" width="{edge:g}" height="{edge:g}"
     viewBox="{view}" role="img" aria-labelledby="title description">
  <title id="title">FrameWork app logo</title>
  <desc id="description">A grey data frame on a sage tile, its green cells spelling the letters FW.</desc>
  <defs>
    <filter id="tile-shadow" x="-15%" y="-15%" width="130%" height="140%">
      <feDropShadow dx="0" dy="{SHADOW_DY}" stdDeviation="{SHADOW_BLUR}" flood-color="{SHADOW_COLOR}" flood-opacity="{SHADOW_OPACITY}"/>
    </filter>
    <filter id="plate-shadow" x="-10%" y="-10%" width="120%" height="125%">
      <feDropShadow dx="0" dy="3" stdDeviation="4" flood-color="{SHADOW_COLOR}" flood-opacity="0.28"/>
    </filter>
  </defs>

  <!-- Delete this group when exporting transparent layers to Icon Composer. -->
  <g id="app-icon-tile"{tile_shadow}>
    <rect x="{TILE_X}" y="{TILE_Y}" width="{TILE_SIZE}" height="{TILE_SIZE}" rx="{TILE_RADIUS}"
          fill="{SAGE}" stroke="{SAGE_EDGE}" stroke-width="{TILE_STROKE}"/>
  </g>

  <!-- The plate is the frame: everything between the cells is gutter. -->
  <g id="frame-plate" filter="url(#plate-shadow)">
    <rect x="{TABLE_X}" y="{TABLE_Y}" width="{TABLE_W}" height="{TABLE_H:g}"
          rx="{TABLE_RADIUS}" fill="{PLATE}"/>
    <rect x="{INNER_X}" y="{TABLE_Y + OUTER_BORDER / 2:g}" width="{INNER_W}" height="{HEADER_H - OUTER_BORDER:g}"
          rx="{CELL_RADIUS}" fill="{HEADER}"/>
  </g>

  <!-- The green cells below the header band spell FW. -->
  <g id="frame-cells">
    {faces}
  </g>
</svg>
'''


# --- favicon -----------------------------------------------------------------
# A tab icon has about sixteen pixels to work with, which is fewer than the
# monogram has cells. So the favicon drops the frame entirely and draws FW as
# letterforms: a pale chip, a sage rim, and two green letters built from the
# same right angles as the grid.

FAVICON_VIEW = 64
FAVICON_RADIUS = 14
FAVICON_RIM = 2

CAP_TOP, CAP_HEIGHT = 16.0, 32.0
STROKE = 7.0
F_X, F_W = 8.0, 17.0
F_BAR_W = 13.5  # the middle arm is shorter than the top one, as in most sans faces
F_BAR_Y = 29.0
W_X, W_W = 29.0, 27.0
W_MID_Y = 27.0  # how high the centre of the W climbs back toward the cap line


def f_bars() -> list[tuple[float, float, float, float]]:
    """The three rectangles of the F, as (x, y, width, height)."""
    return [
        (F_X, CAP_TOP, STROKE, CAP_HEIGHT),
        (F_X, CAP_TOP, F_W, STROKE),
        (F_X, F_BAR_Y, F_BAR_W, STROKE),
    ]


def w_points() -> list[tuple[float, float]]:
    """Centre line of the W, stroked at STROKE width with round joins."""
    baseline = CAP_TOP + CAP_HEIGHT - STROKE / 2
    return [
        (W_X + STROKE / 2, CAP_TOP),
        (W_X + W_W * 0.33, baseline),
        (W_X + W_W * 0.5, W_MID_Y),
        (W_X + W_W * 0.67, baseline),
        (W_X + W_W - STROKE / 2, CAP_TOP),
    ]


def make_mark_svg(chip: str = CELL, ink: str = GREEN, rim_color: str = SAGE) -> str:
    """The letterform mark. The favicon takes it pale on a light chip; the app's
    own header takes it inverted, where it has to hold a spot on near-white
    paper that a solid black square used to hold."""
    bars = "".join(
        f'<rect x="{x:g}" y="{y:g}" width="{w:g}" height="{h:g}" rx="1" fill="{ink}"/>'
        for x, y, w, h in f_bars()
    )
    path = " ".join(
        f"{'M' if i == 0 else 'L'} {x:.2f} {y:.2f}"
        for i, (x, y) in enumerate(w_points())
    )
    rim = FAVICON_RIM / 2
    return f'''<?xml version="1.0" encoding="UTF-8"?>
<!-- Generated by tools/generate_logo.py; edit that, not this. -->
<svg xmlns="http://www.w3.org/2000/svg" width="{FAVICON_VIEW}" height="{FAVICON_VIEW}"
     viewBox="0 0 {FAVICON_VIEW} {FAVICON_VIEW}" role="img" aria-label="FrameWork">
  <rect width="{FAVICON_VIEW}" height="{FAVICON_VIEW}" rx="{FAVICON_RADIUS}" fill="{chip}"/>
  <rect x="{rim:g}" y="{rim:g}" width="{FAVICON_VIEW - FAVICON_RIM:g}" height="{FAVICON_VIEW - FAVICON_RIM:g}"
        rx="{FAVICON_RADIUS - rim:g}" fill="none" stroke="{rim_color}" stroke-width="{FAVICON_RIM}"/>
  {bars}
  <path d="{path}" fill="none" stroke="{ink}" stroke-width="{STROKE:g}"
        stroke-linejoin="round" stroke-linecap="butt"/>
</svg>
'''


# --- raster ------------------------------------------------------------------


def _hex_alpha(color: str, opacity: float) -> str:
    return color + f"{round(max(0.0, min(1.0, opacity)) * 255):02X}"


def make_png(size: int, seed: int = CELL_SEED, supersample: int = 4) -> Image.Image:
    """Draw the app icon with Pillow, oversampled and then downscaled."""
    scale = size * supersample / SIZE
    canvas = int(SIZE * scale)

    def box(x: float, y: float, w: float, h: float) -> list[float]:
        return [x * scale, y * scale, (x + w) * scale, (y + h) * scale]

    tile_box = box(TILE_X, TILE_Y, TILE_SIZE, TILE_SIZE)
    tile_radius = TILE_RADIUS * scale

    def shadow_layer(
        rect: list[float],
        radius: float,
        dy: float,
        blur: float,
        opacity: float,
    ) -> Image.Image:
        """A blurred silhouette on its own layer, so the blur cannot bleed inward."""
        layer = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
        ImageDraw.Draw(layer).rounded_rectangle(
            [rect[0], rect[1] + dy * scale, rect[2], rect[3] + dy * scale],
            radius=radius,
            fill=_hex_alpha(SHADOW_COLOR, opacity),
        )
        return layer.filter(ImageFilter.GaussianBlur(blur * scale))

    image = shadow_layer(tile_box, tile_radius, SHADOW_DY, SHADOW_BLUR, SHADOW_OPACITY)
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle(tile_box, radius=tile_radius, fill=SAGE)
    # An SVG stroke straddles the edge; Pillow draws inward, so grow the box by
    # half the stroke width to land on the same band of pixels.
    half = TILE_STROKE / 2 * scale
    draw.rounded_rectangle(
        [
            tile_box[0] - half,
            tile_box[1] - half,
            tile_box[2] + half,
            tile_box[3] + half,
        ],
        radius=tile_radius + half,
        outline=SAGE_EDGE,
        width=round(TILE_STROKE * scale),
    )

    plate_box = box(TABLE_X, TABLE_Y, TABLE_W, TABLE_H)
    image.alpha_composite(shadow_layer(plate_box, TABLE_RADIUS * scale, 3, 4, 0.28))
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle(plate_box, radius=TABLE_RADIUS * scale, fill=PLATE)
    draw.rounded_rectangle(
        box(INNER_X, TABLE_Y + OUTER_BORDER / 2, INNER_W, HEADER_H - OUTER_BORDER),
        radius=CELL_RADIUS * scale,
        fill=HEADER,
    )

    cells = body_cells(seed)
    for x, y, _, shadow, _ in cells:
        draw.rounded_rectangle(
            box(x, y + CELL_LIFT, CELL_W, BODY_CELL_H),
            radius=CELL_RADIUS * scale,
            fill=shadow,
        )
    for x, y, fill, _, _ in cells:
        draw.rounded_rectangle(
            box(x, y, CELL_W, BODY_CELL_H), radius=CELL_RADIUS * scale, fill=fill
        )

    return image.resize((size, size), Image.LANCZOS)


def make_favicon_png(size: int, supersample: int = 8) -> Image.Image:
    scale = size * supersample / FAVICON_VIEW
    canvas = int(FAVICON_VIEW * scale)

    image = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle(
        [0, 0, canvas - 1, canvas - 1], radius=FAVICON_RADIUS * scale, fill=CELL
    )
    draw.rounded_rectangle(
        [0, 0, canvas - 1, canvas - 1],
        radius=FAVICON_RADIUS * scale,
        outline=SAGE,
        width=round(FAVICON_RIM * scale),
    )
    for x, y, w, h in f_bars():
        draw.rounded_rectangle(
            [x * scale, y * scale, (x + w) * scale, (y + h) * scale],
            radius=1 * scale,
            fill=GREEN,
        )
    draw.line(
        [(x * scale, y * scale) for x, y in w_points()],
        fill=GREEN,
        width=round(STROKE * scale),
        joint="curve",
    )
    return image.resize((size, size), Image.LANCZOS)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--svg", type=Path, default=REPO_ROOT / "assets" / "icon.svg")
    parser.add_argument("--png", type=Path, default=REPO_ROOT / "assets" / "icon.png")
    parser.add_argument(
        "--png-size", type=int, default=1024, help="raster master edge, in pixels"
    )
    parser.add_argument(
        "--favicon-svg", type=Path, default=REPO_ROOT / "public" / "favicon.svg"
    )
    parser.add_argument(
        "--favicon-ico", type=Path, default=REPO_ROOT / "public" / "favicon.ico"
    )
    parser.add_argument(
        "--ui-svg", type=Path, default=REPO_ROOT / "public" / "icon.svg"
    )
    parser.add_argument(
        "--seed", type=int, default=CELL_SEED, help="colour-wash random seed"
    )
    args = parser.parse_args()

    for path, text in (
        (args.svg, make_svg(args.seed)),
        (args.ui_svg, make_svg(args.seed, crop=True)),
        (args.favicon_svg, make_mark_svg()),
    ):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        print(path)

    args.png.parent.mkdir(parents=True, exist_ok=True)
    make_png(args.png_size, args.seed).save(args.png)
    print(args.png)

    args.favicon_ico.parent.mkdir(parents=True, exist_ok=True)
    sizes = (16, 32, 48)
    largest = make_favicon_png(max(sizes))
    largest.save(args.favicon_ico, format="ICO", sizes=[(s, s) for s in sizes])
    print(args.favicon_ico)


if __name__ == "__main__":
    main()
