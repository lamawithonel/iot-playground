"""Adafruit Feather form-factor template: drawing primitives shared by
every board built on the Adafruit Feather outline (2.00 x 0.90 in,
four mounting holes 0.10 in from the board edges, a 0.1 in-pitch pin
row along the top edge and another along the bottom edge, USB
connector + JST battery jack on the left edge, RST button and STEMMA
QT / NeoPixel fixtures near the MCU).

Source: Adafruit "Adafruit STM32F405 Feather Express" product guide
(local copy: ~/downloads/datasheets/adafruit/
adafruit-stm32f405-feather-express.pdf), rendered pages cached under
.cache/agents/pinout-phase2/:
  - feather-fabprint-crop-41.png -- Fabrication Print, p.41: board
    outline 2.00 x 0.90 in, mounting holes 0.10 in from the edges
    (hole-to-hole span 1.80 in = 2.00 - 2 x 0.10, confirming the
    inset), the top pin row starting 0.65 in from the left edge.
  - feather-05.png, feather-page-06.png -- silkscreen photos, p.5-6:
    top row silk order (12 pins) Bat En USB 13 12 11 10 9 6 5 SCL
    SDA; bottom row silk order (15 pads) Rst 3.3V Gnd A0 A1 A2 A3 A4
    A5 SCK MO MI RX TX B0.
  - feather-page-09.png, feather-page-10.png -- p.9-10 text/photos:
    bottom-side micro SD/SDIO slot and an unpopulated 2x5 SWD pad.

This module owns geometry math and generic widget shapes only; every
number that differs board-to-board (silk names, MCU pin/net per
name, fixture positions) is board data, supplied by the caller.

Open item from the architecture pass, now resolved: a prior reading
of this docstring claimed a 4th, unlabeled pad between 3.3V and Gnd
on the bottom row (16 pads total).  A 600 dpi re-render of p.5, done
for tools/pinout/boards/feather_f405.py's pin data, shows the row
plainly-- Rst, 3.3V, Gnd, then A0-A5, with no pad between 3.3V and
Gnd-- so the bottom row is 15 pads, not 16, and there is no
unlabeled pad to resolve.
"""

from dataclasses import dataclass, field

from .svg import GND_FILL, HL, HL_TXT, INK, MUT, PWR, ext_box

# Feather boards are blue-PCB-with-white-silk, the opposite of the
# Nucleo-144 family's white-PCB-with-blue-silk-- so this template
# owns its own PCB/silk colors rather than reusing nucleo144's.
PCB = '#1B4F72'
PCB_SILK = '#ffffff'

# Feather-standard supply/ground silk names, used to color a pin's
# pad and label by role regardless of which row it appears on.
PWR_NAMES = {'Bat', 'USB', 'En', '3.3V'}
GND_NAMES = {'Gnd'}


@dataclass
class Board:
    """One Feather board's identity and true-scale geometry.

    width_in/length_in are the board's real dimensions (its product
    guide's fabrication print)-- kept for documentation and for
    deriving pitch, the on-diagram 0.1 in pin-row step.  x/y/w/h are
    the board rect in diagram pixels.
    """

    name: str
    ref_design: str
    mcu_lines: tuple
    x: int
    y: int
    w: int
    h: int
    width_in: float = 2.00
    length_in: float = 0.90
    pitch: float = field(init=False)

    def __post_init__(self):
        # One 0.1 in pin-pitch step at this board's true scale.
        self.pitch = round(0.1 * self.w / self.width_in, 2)

    @property
    def r(self):
        return self.x + self.w

    @property
    def b(self):
        return self.y + self.h

    @property
    def cx(self):
        return self.x + self.w // 2


@dataclass
class PinRow:
    """One edge's 0.1 in-pitch pin header row.

    start_in is the inset from the board's left edge, in inches, to
    the first pin's center-- 0.65 in for this board's top row, per
    the fab print.  names is left-to-right silk text; None marks a
    pad whose name is unread/unverified (see the module docstring).
    notes maps a name to an annotation drawn beside that pin (e.g.
    B0's DFU jumper-to-3.3V note, since this board has no DFU
    button).  impl flags which annotated names are firmware-verified
    (drawn amber, bold, and starred, svg.HL/HL_TXT) rather than
    merely physically wired (drawn muted, svg.MUT)-- the same
    implemented-vs-present distinction a project overlay for a board
    built on this template uses to separate its active feature set
    from everything else the PCB exposes but does not yet drive.
    """

    side: str  # 'top' | 'bottom'
    start_in: float
    names: list
    notes: dict = field(default_factory=dict)
    impl: set = field(default_factory=set)


@dataclass
class Fixture:
    """A small onboard connector or component.

    kind='conn' draws a labeled rect (USB-C, JST battery, STEMMA QT);
    kind='button' draws a labeled filled circle (RST); kind='led'
    draws a labeled small circle in its own color (the NeoPixel).
    Unlike the Nucleo-144 template's per-widget dataclasses, Feather's
    on-board fixtures reduce to just these two shapes, so one
    dataclass with a kind dispatch stays simpler without losing the
    template/data split.
    """

    kind: str
    x: int
    y: int
    label: str
    w: int = 0
    h: int = 0
    r: int = 0
    fill: str = '#3a3a3a'
    # White silk (the default) only reads where the fixture's own
    # label sits over the dark PCB fill.  A fixture whose label
    # overhangs the board edge onto the page background needs dark
    # ink instead (see feather_f405._fixtures' JST label).
    label_fill: str = ''


def draw_board_rect(doc, board):
    doc.emit(f'<rect x="{board.x}" y="{board.y}" width="{board.w}" '
             f'height="{board.h}" rx="10" fill="{PCB}" '
             f'stroke="#0d2f47" stroke-width="3"/>')
    hole_r = board.pitch * 0.5
    inset_x = 0.10 * board.w / board.width_in
    inset_y = 0.10 * board.w / board.width_in  # same px/in scale both axes
    for hx in (board.x + inset_x, board.r - inset_x):
        for hy in (board.y + inset_y, board.b - inset_y):
            doc.emit(f'<circle cx="{hx}" cy="{hy}" r="{hole_r}" '
                     f'fill="none" stroke="#cfd8dc" stroke-width="2"/>')


def draw_board_label(doc, board):
    cy = board.y + board.h / 2
    doc.emit(f'<text x="{board.cx}" y="{cy - 6}" font-size="13" '
             f'fill="{PCB_SILK}" text-anchor="middle" '
             f'font-weight="bold">{board.name}</text>')
    doc.emit(f'<text x="{board.cx}" y="{cy + 10}" font-size="9" '
             f'fill="{PCB_SILK}" text-anchor="middle">'
             f'{board.ref_design}</text>')


def draw_pin_row(doc, board, row):
    y = board.y if row.side == 'top' else board.b
    pad_r = board.pitch * 0.28
    away = -1 if row.side == 'top' else 1
    # The silk name is drawn outboard of the pad, on the page
    # background-- not inboard on the PCB, where the real board prints
    # it in white silk (module docstring)-- so it needs dark ink
    # instead of a pad-matching fill, and needs to clear the pad
    # circle entirely rather than straddle its edge.  The clearance
    # differs by direction: a top label's descender dips back toward
    # the pad; a bottom label's ascender, which reaches further at
    # this font size, does the same.
    label_dy = away * (pad_r + (3 if row.side == 'top' else 9))
    x0 = board.x + row.start_in / board.width_in * board.w
    last_note_i, tier = None, 0
    for i, name in enumerate(row.names):
        cx = x0 + i * board.pitch
        if name is None:
            fill, stroke, txt = '#5a5a5a', '#222', '?'
        elif name in PWR_NAMES:
            fill, stroke, txt = PWR, '#000', name
        elif name in GND_NAMES:
            fill, stroke, txt = GND_FILL, '#bbbbbb', name
        else:
            fill, stroke, txt = '#c9c9c9', '#000', name
        doc.emit(f'<circle cx="{cx}" cy="{y}" r="{pad_r}" fill="{fill}" '
                 f'stroke="{stroke}" stroke-width="0.8"/>')
        is_impl = name in row.impl
        if is_impl:
            doc.emit(f'<circle cx="{cx}" cy="{y}" r="{pad_r + 2.2}" '
                     f'fill="none" stroke="{HL}" stroke-width="1.4"/>')
        doc.emit(f'<text x="{cx}" y="{y + label_dy}" font-size="9.5" '
                 f'fill="{INK}" text-anchor="middle">{txt}</text>')
        note = row.notes.get(name)
        if note:
            # Stagger onto a second tier only when the previous named
            # pin also carried a note-- adjacent middle-anchored notes
            # at this 45 px pitch otherwise collide (e.g. the bottom
            # row's "PB14 MISO"/"PB15 MOSI", or B0's long
            # "BOOT0->3V3+RST" overwriting TX's "PB10 CS").  An
            # isolated note, with nothing adjacent to collide with,
            # stays on the near tier, closest to its pin.
            tier = 0 if last_note_i != i - 1 else 1 - tier
            last_note_i = i
            note_dy = label_dy + away * (11 + tier * 9)
            color = HL_TXT if is_impl else MUT
            weight = ' font-weight="bold"' if is_impl else ''
            suffix = ' *' if is_impl else ''
            doc.emit(f'<text x="{cx}" y="{y + note_dy}" font-size="7.5" '
                     f'fill="{color}"{weight} text-anchor="middle">'
                     f'{note}{suffix}</text>')


def draw_fixture(doc, f):
    if f.kind == 'conn':
        doc.emit(f'<rect x="{f.x}" y="{f.y}" width="{f.w}" height="{f.h}" '
                 f'rx="3" fill="{f.fill}"/>')
        doc.emit(f'<text x="{f.x + f.w / 2}" y="{f.y + f.h + 13}" '
                 f'font-size="9" fill="{f.label_fill or PCB_SILK}" '
                 f'text-anchor="middle">{f.label}</text>')
    elif f.kind == 'button':
        doc.emit(f'<circle cx="{f.x}" cy="{f.y}" r="{f.r}" fill="{f.fill}" '
                 f'stroke="#111"/>')
        doc.emit(f'<text x="{f.x}" y="{f.y + f.r + 13}" font-size="9" '
                 f'fill="{PCB_SILK}" text-anchor="middle">{f.label}</text>')
    elif f.kind == 'led':
        # ponytail: no board module constructs Fixture(kind='led')--
        # the Feather NeoPixel ended up in _draw_onboard_fixed's boxes
        # instead (feather_f405.py).  Dead branch; delete at next
        # touch if it still has no caller.
        doc.emit(f'<circle cx="{f.x}" cy="{f.y}" r="{f.r}" fill="{f.fill}" '
                 f'stroke="#111" stroke-width="0.8"/>')
        doc.emit(f'<text x="{f.x}" y="{f.y + f.r + 13}" font-size="9" '
                 f'fill="{PCB_SILK}" text-anchor="middle">{f.label}</text>')
    else:
        raise ValueError(f'unknown fixture kind: {f.kind!r}')


def draw_bottom_side_note(doc, x, y, w, h, lines):
    """A dashed off-board-style box for a bottom-side feature (the
    micro SD/SDIO slot, the unpopulated 2x5 SWD pad)-- reuses
    svg.ext_box's dashed style since these are, electrically, exactly
    that: a fixture drawn as a leader-lined aside rather than inline
    on the board face, because they are not visible from the top."""
    ext_box(doc, x, y, w, h, lines, dashed=True)
