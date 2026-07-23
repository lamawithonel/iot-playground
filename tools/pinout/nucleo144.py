"""Nucleo-144 form-factor template: drawing primitives shared by every
board built on the ST Nucleo-144 outline (70 x 133.34 mm, full-length
2.54 mm morpho columns down both edges, ST-LINK zone + USB centered on
the top edge, USER/RESET buttons at the bottom corners, user USB +
Ethernet on the bottom edge, MCU at center).

This module owns geometry math and generic widget shapes only.  Every
number that differs board-to-board-- pin tables, silk names, fixture
designators and positions, jumper defaults-- is board data, supplied
by the caller as one of the dataclasses below.  See
tools/pinout/boards/n657x0.py for the first board built on this
template, and tools/pinout/boards/h753zi.py for the second.

Fixture x/y positions here are NOT re-derived geometrically; each
board module supplies its own, measured from that board's manual.
Only the handful of items the NUCLEO-N657X0-Q/NUCLEO-H753ZI-Q
comparison found byte-for-byte identical across both boards (the
board rect, the morpho columns' shape, the header/jumper/LED widget
shapes) live here as parameterized functions.
"""

from dataclasses import dataclass, field

from .svg import (
    GND_FILL, HIL, HL, HL_TXT, INK, MUT, PAD, PCB, PCB_EDGE, PWR, SILK,
)

# Standard ST Nucleo morpho/Arduino supply names, shared across the
# Nucleo-144 family-- used to color a pin's row label by role
# regardless of which board or header it appears on.
PWR_NAMES = {'5V', '3V3', 'VIN', 'VDDIO', 'VDDIO4', 'VDDIO5',
             'VBAT', '5V_STLK'}
GND_NAMES = {'GND', 'AGND'}


@dataclass
class Board:
    """One Nucleo-144 board's identity and true-scale geometry.

    width_mm/length_mm are the board's real dimensions (its manual's
    mechanical drawing)-- kept for documentation and for deriving
    PITCH, the on-diagram 2.54 mm grid step.  x/y/w/h are the board
    rect in diagram pixels; g0 is the pixel y where header/morpho pin
    row 0 sits-- a layout choice with no algebraic tie to width_mm/
    length_mm, so it stays literal per-board data rather than a
    derived value.
    """

    name: str
    ref_design: str
    mcu_lines: tuple
    width_mm: float
    length_mm: float
    x: int
    y: int
    w: int
    h: int
    g0: int
    # Absolute diagram y-coordinates for a handful of template-drawn
    # fixtures whose exact pixel position is a per-board layout
    # choice, not a derived value (same reasoning as g0 above)--
    # defaults match the N6/MB1940 hand-tuned layout; a second board
    # module overrides whichever ones its own manual places
    # differently, instead of the position silently drifting.
    # ponytail: H753ZI, the second board built on this template, uses
    # every default below unchanged-- none of the six has been
    # overridden yet.  Fine to leave for the flexibility it offers,
    # but a candidate for hardcoding or deletion if a third board
    # still doesn't need it.
    label_name_y: int = 530
    label_ref_y: int = 548
    mcu_box_y: int = 580
    button_cy: int = 1078
    eth_conn_y: int = 1096
    usb_conn_y: int = 1128
    pitch: float = field(init=False)

    def __post_init__(self):
        # One 2.54 mm grid step at this board's true scale, rounded
        # to the tenth of a px the original N6 drawing was hand-tuned
        # to (2.54 mm * (566 px / 70 mm) = 20.5378... -> 20.5).
        self.pitch = round(2.54 * self.w / self.width_mm, 1)

    @property
    def r(self):
        return self.x + self.w

    @property
    def cx(self):
        return self.x + self.w // 2


@dataclass
class MorphoStrip:
    """One ST morpho column: a populated header (fitted_rows) whose
    through-hole grid continues, unpopulated, for unfitted_rows more
    rows under unfitted_designator."""

    designator: str
    x: int
    side: str  # 'left' | 'right' -- which board edge this strip sits on
    fitted_rows: int
    unfitted_rows: int
    unfitted_designator: str
    odd_names: list
    even_names: list
    pwr_pins: set
    gnd_pins: set
    hl_pins: set = field(default_factory=set)
    hil_pins: set = field(default_factory=set)


@dataclass
class Header:
    """One pin header, in either of two physical layouts.

    columns=1 (the default) is the single-column Arduino V3-style
    strip used by every header on the N6 board (CN4/CN5/CN13/CN14):
    rows is a flat list of (mark, mcu_pin, func, note) tuples, one
    per pin.

    columns=2 is the 2-row Zio connector layout NUCLEO-H753ZI (and
    the wider STM32 Nucleo-144 family) use instead of Arduino V3
    (UM2407 Fig 4/6, Tables 15-21): rows is a list of
    ((mark, mcu_pin, func, note), (mark, mcu_pin, func, note)) pairs,
    one pair per physical connector row-- the first tuple is the
    header's inboard pin, the second its outboard pin.
    """

    designator: str
    x: int
    k0: int  # grid row of the header's first pin
    side: str  # 'left' | 'right'
    rows: list  # (mark, mcu_pin, func, note), or a pair per columns
    title: str = 'top'
    columns: int = 1


@dataclass
class Jumper:
    """One jumper/selector widget.  kind selects the shape:
    '2pin' -- a bare 2-pin header, drawn open (no cap)
    '3pin' -- a 3-pin header with a 2-position cap; default=True caps
              the upper pair, False the lower pair
    'sel3' -- a 3-position, 2-pin-per-position selector (e.g. a power
              source select); positions is an ordered ((y_offset,
              label), ...) tuple and default is the selected index
    'sel2x4' -- a 4-position, 2-pin-per-position selector (a 2x4 pin
              block, e.g. NUCLEO-H753ZI's JP2 power-source select,
              UM2407 sect. 7.4.1); same positions/default shape as
              'sel3', generalized to more rows
    """

    designator: str
    kind: str
    x: int
    y: int
    tag: str
    default: object = None
    positions: tuple = ()


@dataclass
class DebugConn:
    """An external debug connector (e.g. a MIPI20 header) drawn as a
    small body plus a two-line leader-line label into the margin."""

    x: int
    y: int
    w: int
    h: int
    designator: str
    label: tuple  # two lines


@dataclass
class SmallFixture:
    """A single labeled rect-- small onboard connectors (e.g. a
    camera header) with no internal pin detail worth drawing."""

    x: int
    y: int
    w: int
    h: int
    label: str


@dataclass
class PowerTestHeader:
    """A 14-pin (2x7), always-fitted current-measurement header."""

    x: int
    y: int
    designator: str


def draw_board_rect(doc, board):
    doc.emit(f'<rect x="{board.x}" y="{board.y}" width="{board.w}" '
             f'height="{board.h}" rx="14" fill="{PCB}" stroke="{PCB_EDGE}" '
             f'stroke-width="3"/>')


def draw_board_label(doc, board):
    doc.emit(f'<text x="{board.cx}" y="{board.label_name_y}" '
             f'font-size="15" fill="{SILK}" text-anchor="middle" '
             f'font-weight="bold">{board.name}</text>')
    doc.emit(f'<text x="{board.cx}" y="{board.label_ref_y}" '
             f'font-size="11" fill="{SILK}" text-anchor="middle">'
             f'{board.ref_design}</text>')


def draw_morpho_strip(doc, board, strip, highlight):
    x = strip.x
    name = strip.designator
    rows = strip.fitted_rows
    doc.emit(f'<rect x="{x}" y="{board.g0 - 13}" width="34" '
             f'height="{rows * board.pitch}" fill="#161616" rx="3"/>')
    for i in range(rows):
        cyc = board.g0 + i * board.pitch
        for col, dx in ((0, 6.75 - 2.75), (1, 27.25 - 2.75)):
            pin_no = 2 * i + 1 + col
            hot = highlight and pin_no in strip.hl_pins
            if hot:
                fill, stroke = HL, '#000'
            elif pin_no in strip.pwr_pins:
                fill, stroke = PWR, '#000'
            elif pin_no in strip.gnd_pins:
                fill, stroke = GND_FILL, '#bbbbbb'
            else:
                fill, stroke = '#8f8f8f', ''
            extra = (f' stroke="{stroke}" stroke-width="0.5"'
                     if stroke else '')
            doc.emit(f'<rect x="{x + dx}" y="{cyc - 2.75}" width="5.5" '
                     f'height="5.5" fill="{fill}"{extra}/>')
            if hot:
                # In the body's central gap, between the two pad
                # columns: the strip gap outside is too narrow
                doc.emit(f'<text x="{x + 17}" y="{cyc + 3}" '
                         f'font-size="8" fill="{HL}" '
                         f'font-weight="bold" text-anchor="middle">'
                         f'{pin_no}</text>')
        # Per-row silk label inboard, odd/even, each name colored by
        # role; unused names light and normal weight
        spans = []
        for col, nm in ((0, strip.odd_names[i]), (1, strip.even_names[i])):
            pin_no = 2 * i + 1 + col
            if highlight and pin_no in strip.hl_pins:
                c, w = HL_TXT, ' font-weight="bold"'
            elif highlight and pin_no in strip.hil_pins:
                c, w = HIL, ' font-weight="bold"'
            elif nm in PWR_NAMES:
                c, w = PWR, ''
            elif nm in GND_NAMES:
                c, w = GND_FILL, ''
            else:
                c, w = '#9a9a94', ''
            spans.append(f'<tspan fill="{c}"{w}>{nm}</tspan>')
        row_lbl = (spans[0]
                   + '<tspan fill="#c4c4be">/</tspan>'
                   + spans[1])
        if strip.side == 'left':
            doc.emit(f'<text x="{x + 38}" y="{cyc + 3}" '
                     f'font-size="7.5">{row_lbl}</text>')
        else:
            doc.emit(f'<text x="{x - 4}" y="{cyc + 3}" '
                     f'font-size="7.5" text-anchor="end">'
                     f'{row_lbl}</text>')
    doc.emit(f'<text x="{x + 17}" y="{board.g0 - 19}" font-size="11" '
             f'fill="{SILK}" text-anchor="middle">{name}</text>')
    # The through-hole grid continues without a break; only the
    # soldered header ends
    for i in range(strip.unfitted_rows):
        cyc = board.g0 + (rows + i) * board.pitch
        for dx in (6.75, 27.25):
            doc.emit(f'<circle cx="{x + dx}" cy="{cyc}" r="2.6" '
                     f'fill="none" stroke="#9a9a9a" stroke-width="1"/>')
    foot_y = board.g0 + (rows + strip.unfitted_rows) * board.pitch
    doc.emit(f'<text x="{x + 17}" y="{foot_y}" '
             f'font-size="8.5" fill="{MUT}" text-anchor="middle">'
             f'{strip.unfitted_designator}</text>')


def draw_header(doc, board, h, highlight):
    if h.columns == 2:
        _draw_header_2col(doc, board, h, highlight)
        return
    y0c = board.g0 + h.k0 * board.pitch
    x = h.x
    doc.emit(f'<rect x="{x}" y="{y0c - 13}" width="28" '
             f'height="{len(h.rows) * board.pitch}" rx="4" fill="#181818"/>')
    if h.title == 'top':
        ty = y0c - 19
    else:
        ty = y0c + len(h.rows) * board.pitch + 7
    doc.emit(f'<text x="{x + 14}" y="{ty}" font-size="11" '
             f'fill="{SILK}" text-anchor="middle">{h.designator}</text>')
    for i, (mark, pin, func, note) in enumerate(h.rows):
        hl = highlight and bool(note)
        is_gnd = func.startswith('Ground')
        is_pwr = mark in PWR_NAMES
        cy = y0c + i * board.pitch
        if hl:
            fill, stroke = HL, '#000'
        elif is_pwr:
            fill, stroke = PWR, '#000'
        elif is_gnd:
            fill, stroke = GND_FILL, '#bbbbbb'
        else:
            fill, stroke = PAD, '#000'
        doc.emit(f'<rect x="{x + 8}" y="{cy - 6}" width="12" '
                 f'height="12" fill="{fill}" stroke="{stroke}" '
                 f'stroke-width="0.6"/>')
        if h.side == 'left':
            doc.emit(f'<text x="{x + 34}" y="{cy + 4}" font-size="11" '
                     f'fill="{SILK}">{mark}</text>')
            tx, anchor, ta, tb = board.x - 14, 'end', board.x, board.x - 10
        else:
            doc.emit(f'<text x="{x - 6}" y="{cy + 4}" font-size="11" '
                     f'fill="{SILK}" text-anchor="end">{mark}</text>')
            tx, anchor, ta, tb = board.r + 14, 'start', board.r, board.r + 10
        doc.emit(f'<line x1="{ta}" y1="{cy}" x2="{tb}" y2="{cy}" '
                 f'stroke="#b5b5b0" stroke-width="1"/>')
        core = f'{pin}  {func}' if pin else func
        if hl:
            fillc = HL_TXT
        elif is_pwr:
            fillc = PWR
        elif is_gnd:
            fillc = GND_FILL
        elif highlight:
            fillc = '#9a9a94'  # unused: lighter, normal weight
        else:
            fillc = INK
        weight = ' font-weight="bold"' if hl else ''
        suffix = f'  ({note}) *' if hl else ''
        doc.emit(f'<text x="{tx}" y="{cy + 4}" font-size="12.5" '
                 f'fill="{fillc}"{weight} text-anchor="{anchor}">'
                 f'{core}{suffix}</text>')


def _draw_header_2col(doc, board, h, highlight):
    """2-row Zio connector layout (Header.columns == 2).

    Draws both pin columns' pads side by side per row, the same way
    MorphoStrip draws its odd/even columns, and both columns' full
    pin/net description on one combined external text line per row--
    the inboard column first, an inline '|' separating it from the
    outboard column, since one physical connector row is one on-
    diagram row here, not two.
    """
    y0c = board.g0 + h.k0 * board.pitch
    x = h.x
    col_dx = (8, 28)  # inboard, outboard pad x-offsets from x
    body_w = col_dx[1] + 12 + 8
    doc.emit(f'<rect x="{x}" y="{y0c - 13}" width="{body_w}" '
             f'height="{len(h.rows) * board.pitch}" rx="4" '
             f'fill="#181818"/>')
    if h.title == 'top':
        ty = y0c - 19
    else:
        ty = y0c + len(h.rows) * board.pitch + 7
    doc.emit(f'<text x="{x + body_w / 2}" y="{ty}" font-size="11" '
             f'fill="{SILK}" text-anchor="middle">{h.designator}</text>')
    for i, pair in enumerate(h.rows):
        cy = y0c + i * board.pitch
        marks, cores, hl_row = [], [], False
        for col, (mark, pin, func, note) in enumerate(pair):
            hl = highlight and bool(note)
            hl_row = hl_row or hl
            is_gnd = func.startswith('Ground')
            is_pwr = mark in PWR_NAMES
            if hl:
                fill, stroke = HL, '#000'
            elif is_pwr:
                fill, stroke = PWR, '#000'
            elif is_gnd:
                fill, stroke = GND_FILL, '#bbbbbb'
            else:
                fill, stroke = PAD, '#000'
            doc.emit(f'<rect x="{x + col_dx[col]}" y="{cy - 6}" '
                     f'width="12" height="12" fill="{fill}" '
                     f'stroke="{stroke}" stroke-width="0.6"/>')
            marks.append(mark)
            cores.append(f'{pin}  {func}' if pin else func)
        if h.side == 'left':
            doc.emit(f'<text x="{x + body_w + 6}" y="{cy + 4}" '
                     f'font-size="11" fill="{SILK}">{marks[0]}/'
                     f'{marks[1]}</text>')
            tx, anchor, ta, tb = board.x - 14, 'end', board.x, board.x - 10
        else:
            doc.emit(f'<text x="{x - 6}" y="{cy + 4}" font-size="11" '
                     f'fill="{SILK}" text-anchor="end">{marks[0]}/'
                     f'{marks[1]}</text>')
            tx, anchor, ta, tb = board.r + 14, 'start', board.r, board.r + 10
        doc.emit(f'<line x1="{ta}" y1="{cy}" x2="{tb}" y2="{cy}" '
                 f'stroke="#b5b5b0" stroke-width="1"/>')
        if hl_row:
            fillc = HL_TXT
        elif highlight:
            fillc = '#9a9a94'  # unused: lighter, normal weight
        else:
            fillc = INK
        weight = ' font-weight="bold"' if hl_row else ''
        doc.emit(f'<text x="{tx}" y="{cy + 4}" font-size="12.5" '
                 f'fill="{fillc}"{weight} text-anchor="{anchor}">'
                 f'{cores[0]}  |  {cores[1]}</text>')


def draw_jumper(doc, j):
    if j.kind == '3pin':
        _draw_jumper_3pin(doc, j)
    elif j.kind == '2pin':
        _draw_jumper_2pin(doc, j)
    elif j.kind == 'sel3':
        _draw_jumper_sel3(doc, j)
    elif j.kind == 'sel2x4':
        _draw_jumper_sel2x4(doc, j)
    else:
        raise ValueError(f'unknown jumper kind: {j.kind!r}')


def _draw_jumper_3pin(doc, j):
    x = j.x
    for py in (j.y, j.y + 16, j.y + 32):
        doc.emit(f'<rect x="{x + 4}" y="{py}" width="8" height="8" '
                 f'fill="#c9c9c9" stroke="#000" stroke-width="0.5"/>')
    doc.emit(f'<text x="{x}" y="{j.y + 40}" font-size="7" fill="{SILK}" '
             f'text-anchor="end">1</text>')
    cap_y = (j.y - 3) if j.default else (j.y + 13)
    doc.emit(f'<rect x="{x + 1}" y="{cap_y}" width="14" height="30" '
             f'rx="3" fill="#3a5a8a" stroke="#111" stroke-width="1"/>')
    doc.emit(f'<text x="{x + 8}" y="{j.y + 54}" font-size="9" fill="{SILK}" '
             f'text-anchor="middle">{j.designator}</text>')
    doc.emit(f'<text x="{x + 8}" y="{j.y + 66}" font-size="7.5" '
             f'fill="{SILK}" text-anchor="middle">{j.tag}</text>')


def _draw_jumper_2pin(doc, j):
    x = j.x
    for py in (j.y, j.y + 16):
        doc.emit(f'<rect x="{x + 4}" y="{py}" width="8" height="8" '
                 f'fill="#c9c9c9" stroke="#000" stroke-width="0.5"/>')
    doc.emit(f'<text x="{x}" y="{j.y + 24}" font-size="7" fill="{SILK}" '
             f'text-anchor="end">1</text>')
    doc.emit(f'<text x="{x + 8}" y="{j.y + 38}" font-size="9" fill="{SILK}" '
             f'text-anchor="middle">{j.designator}</text>')
    doc.emit(f'<text x="{x + 8}" y="{j.y + 50}" font-size="7.5" '
             f'fill="{SILK}" text-anchor="middle">{j.tag}</text>')


def _draw_jumper_sel3(doc, j):
    x = j.x
    for r, (dy, nm) in enumerate(j.positions):
        py = j.y + dy
        for dx in (3, 19):
            doc.emit(f'<rect x="{x + dx}" y="{py}" width="8" height="8" '
                     f'fill="#c9c9c9" stroke="#000" stroke-width="0.5"/>')
        color = SILK if r == j.default else '#9a9a94'
        doc.emit(f'<text x="{x + 33}" y="{py + 7}" font-size="7.5" '
                 f'fill="{color}">{nm}</text>')
    doc.emit(f'<rect x="{x}" y="{j.y - 3}" width="30" height="14" rx="3" '
             f'fill="#3a5a8a" stroke="#111" stroke-width="1"/>')
    doc.emit(f'<text x="{x + 16}" y="{j.y + 54}" font-size="9" fill="{SILK}" '
             f'text-anchor="middle">{j.designator}</text>')
    doc.emit(f'<text x="{x + 16}" y="{j.y + 66}" font-size="7.5" '
             f'fill="{SILK}" text-anchor="middle">{j.tag}</text>')


def _draw_jumper_sel2x4(doc, j):
    # Same drawing as _draw_jumper_sel3, generalized from 3 positions
    # to however many j.positions holds (4, for a 2x4 block)-- kept
    # as its own function, rather than folded into _draw_jumper_sel3,
    # so sel3's own output (N6's CN9) can never drift from this
    # change.
    x = j.x
    for r, (dy, nm) in enumerate(j.positions):
        py = j.y + dy
        for dx in (3, 19):
            doc.emit(f'<rect x="{x + dx}" y="{py}" width="8" height="8" '
                     f'fill="#c9c9c9" stroke="#000" stroke-width="0.5"/>')
        color = SILK if r == j.default else '#9a9a94'
        doc.emit(f'<text x="{x + 33}" y="{py + 7}" font-size="7.5" '
                 f'fill="{color}">{nm}</text>')
    cap_dy = j.positions[j.default][0]
    doc.emit(f'<rect x="{x}" y="{j.y + cap_dy - 3}" width="30" '
             f'height="14" rx="3" fill="#3a5a8a" stroke="#111" '
             f'stroke-width="1"/>')
    last_dy = j.positions[-1][0]
    doc.emit(f'<text x="{x + 16}" y="{j.y + last_dy + 18}" '
             f'font-size="9" fill="{SILK}" text-anchor="middle">'
             f'{j.designator}</text>')
    doc.emit(f'<text x="{x + 16}" y="{j.y + last_dy + 30}" '
             f'font-size="7.5" fill="{SILK}" text-anchor="middle">'
             f'{j.tag}</text>')


def draw_stlink_zone(doc, board, title, subtitle, part_name):
    doc.emit(f'<rect x="{board.cx - 30}" y="58" width="60" height="22" '
             f'rx="6" fill="#3a3a3a"/>')
    doc.emit(f'<rect x="{board.cx - 142}" y="95" width="285" height="90" '
             f'rx="6" fill="none" stroke="{SILK}" stroke-dasharray="5 4" '
             f'stroke-width="1.2"/>')
    # part_name is the board's ST-LINK silkscreen name, which differs
    # per board (N6/MB1940: 'STLINK-V3EC'; NUCLEO-H753ZI/MB1364:
    # plain 'STLINK-V3E', UM2407 sect. 7.3)-- every caller supplies
    # its own rather than the template guessing.
    doc.emit(f'<text x="{board.cx + 10}" y="140" font-size="12" '
             f'fill="{SILK}" text-anchor="middle">{part_name}</text>')
    doc.emit(f'<text x="{board.cx}" y="30" font-size="12.5" fill="{INK}" '
             f'font-weight="bold" text-anchor="middle">{title}</text>')
    doc.emit(f'<text x="{board.cx}" y="46" font-size="11" fill="{MUT}" '
             f'text-anchor="middle">{subtitle}</text>')


def draw_debug_conn(doc, board, f):
    doc.emit(f'<rect x="{f.x}" y="{f.y}" width="{f.w}" height="{f.h}" '
             f'rx="3" fill="#161616"/>')
    doc.emit(f'<text x="{f.x + f.w // 2}" y="{f.y + f.h + 15}" '
             f'font-size="9" fill="{SILK}" text-anchor="middle">'
             f'{f.designator}</text>')
    leader_y = f.y + f.h // 2
    doc.emit(f'<line x1="{f.x}" y1="{leader_y}" x2="{board.x - 10}" '
             f'y2="{leader_y}" stroke="#b5b5b0"/>')
    doc.emit(f'<text x="{board.x - 14}" y="{leader_y - 4}" font-size="11" '
             f'fill="{MUT}" text-anchor="end">{f.label[0]}</text>')
    doc.emit(f'<text x="{board.x - 14}" y="{leader_y + 10}" font-size="11" '
             f'fill="{MUT}" text-anchor="end">{f.label[1]}</text>')


def draw_leds(doc, board):
    # Phase 2 review asked whether this red/green/yellow triple is
    # N6-specific: checked against UM2407 sect. 8.6 (NUCLEO-H753ZI's
    # own three user LEDs, LD1 green/LD2 yellow/LD3 red)-- the same
    # three colors, so the triple stays a template literal rather
    # than becoming per-board data.
    for cy, c in ((110, '#d64545'), (130, '#3fae5a'), (150, '#e0c23e')):
        doc.emit(f'<circle cx="{board.r - 45}" cy="{cy}" r="5" '
                 f'fill="{c}"/>')
    doc.emit(f'<text x="{board.r - 45}" y="172" font-size="9" '
             f'fill="{SILK}" text-anchor="middle">LEDs</text>')


def draw_mcu_box(doc, board):
    y = board.mcu_box_y
    doc.emit(f'<rect x="{board.cx - 65}" y="{y}" width="130" height="130" '
             f'rx="6" fill="#242424" stroke="#000"/>')
    doc.emit(f'<text x="{board.cx}" y="{y + 58}" font-size="14" '
             f'fill="#eee" text-anchor="middle">{board.mcu_lines[0]}'
             f'</text>')
    doc.emit(f'<text x="{board.cx}" y="{y + 76}" font-size="14" '
             f'fill="#eee" text-anchor="middle">{board.mcu_lines[1]}'
             f'</text>')
    doc.emit(f'<circle cx="{board.cx - 54}" cy="{y + 11}" r="3" '
             f'fill="#888"/>')


def draw_small_fixture(doc, f):
    doc.emit(f'<rect x="{f.x}" y="{f.y}" width="{f.w}" height="{f.h}" '
             f'rx="3" fill="#161616"/>')
    doc.emit(f'<text x="{f.x + f.w // 2}" y="{f.y + 28}" font-size="9" '
             f'fill="{SILK}" text-anchor="middle">{f.label}</text>')


def draw_power_test_header(doc, f):
    # A fixed 2x7, 8px-pitch pad grid-- the widget shape is generic,
    # only its position and designator are board data.
    for col in range(7):
        cx = f.x + col * 8
        for cy in (f.y, f.y + 10):
            doc.emit(f'<rect x="{cx}" y="{cy}" width="6" height="6" '
                     f'fill="#c9c9c9" stroke="#000" stroke-width="0.5"/>')
    span = 6 * 8 + 6
    doc.emit(f'<text x="{f.x + span // 2}" y="{f.y + 29}" font-size="9" '
             f'fill="{SILK}" text-anchor="middle">{f.designator}</text>')


def draw_buttons(doc, board, b1_label, b2_label):
    cy = board.button_cy
    doc.emit(f'<circle cx="{board.x + 62}" cy="{cy}" r="16" '
             f'fill="#2563eb" stroke="#111"/>')
    doc.emit(f'<text x="{board.x + 62}" y="{cy + 30}" font-size="10" '
             f'fill="{SILK}" text-anchor="middle">{b1_label}</text>')
    doc.emit(f'<circle cx="{board.r - 62}" cy="{cy}" r="16" '
             f'fill="#1c1c1c" stroke="#666"/>')
    doc.emit(f'<text x="{board.r - 62}" y="{cy + 30}" font-size="10" '
             f'fill="{SILK}" text-anchor="middle">{b2_label}</text>')


def draw_bottom_connectors(doc, board, usb_label, eth_label):
    ux, uy, uw, uh = board.cx - 63, board.usb_conn_y, 56, 22
    doc.emit(f'<rect x="{ux}" y="{uy}" width="{uw}" height="{uh}" rx="5" '
             f'fill="#3a3a3a"/>')
    doc.emit(f'<text x="{ux + uw // 2}" y="{uy - 6}" font-size="9" '
             f'fill="{SILK}" text-anchor="middle">{usb_label}</text>')
    ex, ey, ew, eh = board.cx + 20, board.eth_conn_y, 76, 54
    doc.emit(f'<rect x="{ex}" y="{ey}" width="{ew}" height="{eh}" rx="4" '
             f'fill="#3a3a3a"/>')
    doc.emit(f'<rect x="{ex + 16}" y="{ey + 16}" width="{ew - 32}" '
             f'height="{eh - 24}" fill="#181818"/>')
    doc.emit(f'<text x="{ex + ew // 2}" y="{ey - 6}" font-size="9" '
             f'fill="{SILK}" text-anchor="middle">{eth_label}</text>')
