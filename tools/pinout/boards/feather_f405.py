"""Adafruit Feather STM32F405 Express pinout board data and the
feather-stm32f405/ (flagship connected-device app) project overlay.

Two variants from one board drawing:
  board   -- base board: generic header names, debug + DFU hookup
             only (board page).
  project -- flagship app: every physically-wired header pin's real
             device/function, with the implemented-vs-present-only
             distinction below (system_requirements.md section 3.1,
             replacing its ASCII diagram).

Pin data source: docs/src/system_requirements.md section 3
("CRITICAL - DO NOT MODIFY") is the authority for every MCU
pin<->silk-mark<->device assignment used below; section 3.2's
Peripheral Pin Map is the same data in table form and is unchanged
by this module.  Physical layout (silk order, pad count) is
cross-checked against the real board: a 600 dpi render of the
Adafruit product guide p.5 (~/downloads/datasheets/adafruit/
adafruit-stm32f405-feather-express.pdf), which shows the bottom
header as 15 pads-- Rst, 3.3V, Gnd, A0-A5, SCK, MO, MI, RX, TX,
B0-- not 16.

Finding: section 3.1's ASCII diagram lists the bottom (left-side)
header as a 16-pin header with two consecutive "3.3V" rows (Reset,
3.3V, 3.3V, GND, ...).  The real board has one 3.3V pad, not two; the
row immediately after it is Gnd, matching every downstream row once
the duplicate is dropped-- position-by-position, the ASCII's own
Mark column (A0...A5, SCK, MO, MI, RX, TX) and MCU-pin column line up
exactly with the real 15-pad silk order once the phantom second
3.3V row is removed.  Section 3.1 also omits BOOT0 (B0) entirely,
though it is a real, always-present STM32 pin and the board's own
DFU instructions (board page, product guide p.34) depend on it.
This module draws the real 15+12 = 27-pin header (not 16+12 = 28):
one 3.3V pad on the bottom row, and B0 included as its 15th pad.
See the module docstring open item in ../feather.py for the parallel
correction to that template's own (independently wrong, in the
other direction) pad count.

Implemented-vs-present status (module map:
boards/feather-stm32f405/AGENTS.md; 'What It Does':
docs/src/boards/feather-stm32f405.md) drives the project variant's
per-pin highlight: W5500 Ethernet, the SEN66 I2C sensor, the
on-board LED, and the BOOT0/RESET DFU fallback are firmware-verified
(amber ring, bold amber label, "*" suffix, feather.py's
PinRow.impl); the SSD1681 e-ink display, 23LC1024 SRAM, CAN1
transceiver, and the E-ink display's own on-board microSD are
physically wired per the SRS but not yet driven by any firmware
module (muted label, no ring).
The base variant carries none of this-- generic silk names only, no
device wiring-- per the board-page/project-doc split this repo
already uses for NUCLEO-N657X0-Q (tools/pinout/boards/n657x0.py).
"""

from .. import feather as t
from .. import svg

# Real silk order, both header rows (see module docstring: the
# bottom row is 15 pads, not the 16 that tools/pinout/feather.py's
# own docstring claimed before this pass corrected it).
TOP_NAMES = ['Bat', 'En', 'USB', '13', '12', '11', '10', '9', '6',
             '5', 'SCL', 'SDA']
BOTTOM_NAMES = ['Rst', '3.3V', 'Gnd', 'A0', 'A1', 'A2', 'A3', 'A4',
                'A5', 'SCK', 'MO', 'MI', 'RX', 'TX', 'B0']

# name -> "MCU-pin short-function", system_requirements.md section 3.
TOP_FUNC = {
    '13': 'PC1 LED',
    '12': 'PC2 IRQ',
    '11': 'PC3 RST',
    '10': 'PB9 TX',
    '9': 'PB8 RX',
    '6': 'PC6 CS',
    '5': 'PC7 RST',
    'SCL': 'PB6 SCL',
    'SDA': 'PB7 SDA',
}
BOTTOM_FUNC = {
    'A0': 'PA4 BUSY',
    'A1': 'PA5 SCK',
    'A2': 'PA6 MISO',
    'A3': 'PA7 MOSI',
    'A4': 'PC4 CS',
    'A5': 'PC5 D/C',
    'SCK': 'PB13 SCK',
    'MO': 'PB14 MISO',
    'MI': 'PB15 MOSI',
    'RX': 'PB11 CS',
    'TX': 'PB10 CS',
    'B0': 'BOOT0->3V3+RST',
}

# Firmware-verified names per row (project variant only); everything
# else in *_FUNC above is physically wired but not yet driven.
TOP_IMPL = {'13', '12', '11', '6', 'SCL', 'SDA'}
BOTTOM_IMPL = {'SCK', 'MO', 'MI', 'B0'}


def _board():
    return t.Board(
        name='Feather STM32F405',
        ref_design='Adafruit 4382',
        mcu_lines=('STM32F405RG', 'Cortex-M4F'),
        x=280, y=220, w=900, h=405,
    )


def _rows(project):
    if project:
        top_notes, top_impl = dict(TOP_FUNC), TOP_IMPL
        bottom_notes = dict(BOTTOM_FUNC)
    else:
        # Base board: generic silk names only, no peripheral wiring--
        # BOOT0 is the one documented exception (doctrine: the base
        # diagram must still show the DFU entry point).
        top_notes, top_impl = {}, set()
        bottom_notes = {'B0': BOTTOM_FUNC['B0']}
    bottom_impl = BOTTOM_IMPL if project else set()
    return [
        t.PinRow(side='top', start_in=0.65, names=TOP_NAMES,
                 notes=top_notes, impl=top_impl),
        # 0.30 in centers the 15-pad row inside the 0.10-1.90 in
        # mounting-hole span (1.80 in usable, 1.40 in needed)-- not a
        # fab-print-measured offset like the top row's 0.65 in (see
        # module docstring: no fab-print crop covers this row).
        t.PinRow(side='bottom', start_in=0.30, names=BOTTOM_NAMES,
                 notes=bottom_notes, impl=bottom_impl),
    ]


def _fixtures(board):
    # x values overlap the board's left edge rather than floating off
    # it entirely: draw_fixture's label sits below the fixture rect,
    # and needs the dark PCB fill (not the page background) behind
    # it for the template's white PCB_SILK label text to read.
    return [
        t.Fixture(kind='conn', x=board.x - 6, y=board.y + 50,
                   w=34, h=54, label='USB-C', fill='#8a8a8a'),
        t.Fixture(kind='conn', x=board.x - 4, y=board.y + 140,
                   w=20, h=16, label='JST 2-pin (BAT)', fill='#111111'),
        t.Fixture(kind='button', x=board.x + 76, y=board.y + 66,
                   r=11, label='RST', fill='#3a3a3a'),
    ]


def _swatch(doc, x, y, fill, stroke, label, fill_text=svg.INK):
    doc.emit(f'<rect x="{x}" y="{y}" width="12" height="12" '
             f'fill="{fill}" stroke="{stroke}" stroke-width="0.6"/>')
    doc.emit(f'<text x="{x + 20}" y="{y + 10}" font-size="12.5" '
             f'fill="{fill_text}">{label}</text>')


def build(variant):
    project = variant == 'project'
    board = _board()
    W = board.r + (460 if project else 340)
    H = 1060 if project else 800
    doc = svg.SvgDoc(W, H)

    svg.open_doc(doc)
    t.draw_board_rect(doc, board)
    t.draw_board_label(doc, board)

    for row in _rows(project):
        t.draw_pin_row(doc, board, row)

    for f in _fixtures(board):
        t.draw_fixture(doc, f)

    if not project:
        # Base-board doctrine: generic pin names, debug + DFU hookup
        # only, no peripheral wiring (n657x0.py's board/ars split is
        # the model this repo already uses).
        doc.emit(f'<text x="{board.x}" y="{board.y - 34}" '
                  f'font-size="12.5" fill="{svg.INK}">Debug: Segger '
                  f'J-Link over SWD, unpopulated 2x5 pad on the PCB '
                  f'bottom (product guide p.9).</text>')
        t.draw_bottom_side_note(
            doc, board.x, board.b + 20, 260, 42,
            ['SWD debug (2x5 pad, unpopulated)',
             'bottom of PCB -- primary debug path'])
        doc.emit(f'<text x="{board.x}" y="{board.b + 84}" '
                  f'font-size="11" fill="{svg.MUT}">DFU entry: jumper '
                  f'B0 to 3.3V, then press RST (or power-cycle); no '
                  f'BOOT0 button on this board.</text>')
        _swatch(doc, board.x, board.b + 110, svg.PWR, '#000',
                'power pin (Bat, En, USB, 3.3V)')
        _swatch(doc, board.x + 260, board.b + 110, svg.GND_FILL,
                '#bbbbbb', 'ground pin (Gnd)')
        doc.emit(f'<text x="{board.x}" y="{board.b + 140}" '
                  f'font-size="11" fill="{svg.MUT}">Full pin data: '
                  f'system_requirements.md section 3.  This is the '
                  f'base-board diagram-- generic silk names only, no '
                  f'peripheral wiring; the project pin map (same '
                  f'header, every peripheral shown) lives in section '
                  f'3.1.</text>')
        doc.emit('</svg>')
        return W, doc.out

    # ── Project variant: every peripheral, implemented vs present ──
    # Captions sit well above the top row's own per-pin function
    # labels (drawn at board.y - 21 by draw_pin_row's note offset)--
    # 40+ px of clearance avoids the two overlapping.
    doc.emit(f'<text x="{board.x}" y="{board.y - 58}" font-size="13" '
              f'fill="{svg.INK}" font-weight="bold">feather-stm32f405'
              f'/-- Air-Quality Sensor Node: full pin map</text>')
    doc.emit(f'<text x="{board.x}" y="{board.y - 42}" font-size="11" '
              f'fill="{svg.MUT}">Amber ring + bold + "*": firmware-'
              f'verified.  Muted, no ring: physically wired, not yet '
              f'driven by any firmware module.</text>')

    # On-board fixtures with no manual-verified exact PCB position--
    # placed near the MCU as a cluster, muted (present, not driven).
    # Their pin identity (SRS section 3.2) is the verified fact here,
    # not their drawn (x, y).  fy0 clears the bottom row's own
    # per-pin function labels (drawn at board.b + 25).
    fx0, fy0 = board.cx - 170, board.b + 70
    doc.emit(f'<text x="{fx0}" y="{fy0 - 10}" font-size="10.5" '
              f'fill="{svg.MUT}">on-board, position not manual-'
              f'verified (pin identity only, SRS 3.2):</text>')
    for i, (label, note) in enumerate((
        ('NeoPixel', 'PC0'),
        ('Y1 12.000 MHz', 'PH0/PH1'),
        ('Y2 32.768 kHz', 'PC14/PC15'),
        ('25Q16 flash (2 MiB)', 'PB3/PB4/PB5/PA15'),
    )):
        fx = fx0 + i * 130
        doc.emit(f'<rect x="{fx}" y="{fy0}" width="110" height="34" '
                  f'rx="4" fill="#6b6b6b" stroke="#333" '
                  f'stroke-width="1"/>')
        doc.emit(f'<text x="{fx + 55}" y="{fy0 + 15}" font-size="9.5" '
                  f'fill="#eeeeee" text-anchor="middle">{label}'
                  f'</text>')
        doc.emit(f'<text x="{fx + 55}" y="{fy0 + 28}" font-size="8.5" '
                  f'fill="#cfcfcf" text-anchor="middle">{note}'
                  f'</text>')

    # SWD debug (verified) and on-board microSD/SDIO (present, not
    # driven)-- both bottom-side, real, unlike the fixture cluster
    # above; drawn with draw_bottom_side_note per the template's own
    # convention for bottom-of-PCB features.
    swd_y = fy0 + 60
    _swatch(doc, board.x, swd_y - 14, svg.HL, '#000',
            'firmware-verified', fill_text=svg.HL_TXT)
    t.draw_bottom_side_note(
        doc, board.x, swd_y, 260, 42,
        ['SWD debug (2x5 pad, unpopulated) *',
         'bottom of PCB -- primary debug path'])
    _swatch(doc, board.x + 280, swd_y - 14, '#6b6b6b', '#333',
            'present, not yet driven')
    t.draw_bottom_side_note(
        doc, board.x + 280, swd_y, 260, 42,
        ['On-board microSD (SDIO)',
         'PC8-PC12, PD2 -- bottom of PCB'])

    doc.emit(f'<text x="{board.x}" y="{swd_y + 62}" font-size="11" '
              f'fill="{svg.MUT}">DFU entry: jumper B0 to 3.3V, then '
              f'press RST (or power-cycle); no BOOT0 button on this '
              f'board.</text>')

    # Per-device legend: which header pins belong to which device,
    # and that device's status.  Ties the per-pin PA4/PC2/etc. tags
    # back to system_requirements.md section 3.2's Peripheral Pin
    # Map, the authority this diagram encodes.
    legend_y = swd_y + 92
    doc.emit(f'<text x="{board.x}" y="{legend_y}" font-size="12.5" '
              f'fill="{svg.INK}" font-weight="bold">Devices (SRS '
              f'section 3.2):</text>')
    rows = (
        ('W5500 Ethernet', 'top 12/11/6, bottom SCK/MO/MI', True),
        ('SEN66 sensor (I2C1)', 'top SCL/SDA', True),
        ('On-board LED', 'top 13', True),
        ('BOOT0/RESET DFU fallback', 'bottom B0 + RST button', True),
        ('SSD1681 e-ink display', 'bottom A0-A5, top 5', False),
        ('23LC1024 SRAM (SPI1)', 'bottom RX', False),
        ('E-ink microSD (SPI1)', 'bottom TX', False),
        ('CAN1 transceiver', 'top 10/9', False),
    )
    for i, (dev, where, impl) in enumerate(rows):
        ly = legend_y + 20 + i * 16
        color = svg.HL_TXT if impl else svg.MUT
        weight = ' font-weight="bold"' if impl else ''
        star = ' *' if impl else ''
        doc.emit(f'<text x="{board.x}" y="{ly}" font-size="11.5" '
                  f'fill="{color}"{weight}>{dev}{star}</text>')
        doc.emit(f'<text x="{board.x + 210}" y="{ly}" font-size="11" '
                  f'fill="{svg.MUT}">{where}</text>')
    footer_y = legend_y + 20 + len(rows) * 16 + 12
    doc.emit(f'<text x="{board.x}" y="{footer_y}" font-size="11" '
              f'fill="{svg.MUT}">Full pin data: '
              f'system_requirements.md section 3.  Module map: '
              f'boards/feather-stm32f405/AGENTS.md.</text>')

    doc.emit('</svg>')
    return W, doc.out
