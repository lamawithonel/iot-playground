"""NUCLEO-H753ZI pinout board data and ARS DAC/ADC loopback overlay.

Two variants from one board drawing:
  board -- base board, dev-setup jumpers + full Zio/morpho pinout
  ars   -- adds the planned DAC1_OUT1 (PA4) -> ADC (PA3) loopback
           jumper and the two not-yet-pinned HIL placeholder channels

House rule: a board drawing always uses the documented board's real
aspect ratio, from its manual-- UM2407 Rev 6 Figure 6 (p.16) gives
70.00 x 133.34 mm, the same Nucleo-144 outline as NUCLEO-N657X0-Q, so
this board reuses the template's board rect unchanged (566 x 1080).

Header/morpho data: UM2407 Rev 6 Tables 18-22 (p.40-44), transcribed
from the cached full-text dump (.cache/agents/pinout-phase3/
um2407.txt) rather than the phase-3 research summary, so pin numbers
here are verbatim table values, not paraphrase.  Jumper defaults:
Table 4 (p.12).  ST-LINK naming: sect. 7.3 ("Embedded STLINK-V3E").
LEDs/buttons: sect. 7.6.1-7.6.2 (p.27).  ARS loopback role:
docs/src/boards/nucleo-h753zi.md and
docs/src/projects/ars-toolhead-sensor/hil-measurements.md, which are
both the authority on the loopback's status (planned, not wired) and
own on any disagreement here.

Zio connectors are the columns=2 layout (UM2407 calls them "ZIO
connector (CNx)"): each physical row pairs an Arduino-Uno-compatible
"inboard" pin with a Zio-extension "outboard" pin, per the Header
docstring in nucleo144.py.  CN7/CN10 (digital + QSPI/TIM/analog A6-A8)
sit on the right, mirroring CN9's ADC A0-A5 + CN8's power/SDMMC on
the left-- the same power+analog-left/digital-right split
tools/pinout/boards/n657x0.py uses, and here it is directly
pin-table-driven: CN8/CN9 carry the power rows and A0-A5, CN7/CN10
do not.

ST morpho CN11 (left)/CN12 (right): Table 22 confirms both fully
populated at 35 pin pairs (70 pins) each, with no unfitted
continuation footprint (unlike N6's CN2/CN16)-- unfitted_rows=0 and
an empty unfitted_designator reflect that; draw_morpho_strip always
emits a footer text element, but an empty string renders no visible
mark.

JP1-JP5/CN2/CN5 x/y are diagram layout choices, not measured
coordinates-- UM2407's Figure 6 mechanical drawing dimensions only
the mounting holes and header columns, not these small jumpers/
connectors (open question already flagged in the phase-3 research).
Left/right placement of JP1/JP2 vs JP3/JP4/JP5 does track Figure 4's
real layout (JP1/JP2 near CN1 on the board's right, JP3/JP4/JP5 near
the User LEDs on the left), even though the two Zio headers stacked
on each side are grouped by pin-table content, not by matching
Figure 4's photo layout.
"""

from .. import nucleo144 as t
from .. import svg

# ST morpho per-pin names, UM2407 Rev 6 Table 22 (p.43-44), odd/even
# columns transcribed in pin-number order (1,3,5..69 / 2,4,6..70).
CN11_ODD = [
    'PC10', 'PC12', '3V3_VDD', 'BOOT0', 'PF6', 'PF7', 'PA13', 'PA14',
    'PA15', 'GND', 'PB7', 'PC13', 'PC14', 'PC15', 'PH0', 'PH1',
    'VBAT', 'PC2', 'PC3', 'PD4', 'PD5', 'PD6', 'PD7', 'PE3', 'GND',
    'PF1', 'PF0', 'PD1', 'PD0', 'PG0', 'PE1', 'PG9', 'PG12', 'NC',
    'PD9',
]
CN11_EVEN = [
    'PC11', 'PD2', '5V_EXT', 'GND', 'NC', 'IOREF', 'NRST', '3V3',
    '5V', 'GND', 'GND', 'VIN', 'NC', 'PA0', 'PA1', 'PA4', 'PB0',
    'PC1', 'PC0', 'PD3', 'PG2', 'PG3', 'PE2', 'PE4', 'PE5', 'PF2',
    'PF8', 'PF9', 'PG1', 'GND', 'PE6', 'PG15', 'PG10', 'PG13', 'PG11',
]
CN12_ODD = [
    'PC9', 'PB8', 'PB9', 'VREFP', 'GND', 'PA5', 'PA6', 'PA7', 'PB6',
    'PC7', 'PA9', 'PA8', 'PB10', 'PB4', 'PB5', 'PB3', 'PA10', 'PA2',
    'PA3', 'GND', 'PD13', 'PD12', 'PD11', 'PE10', 'PE12', 'PE14',
    'PE15', 'PE13', 'PF13', 'PF12', 'PG14', 'GND', 'PD10', 'PG7',
    'PG4',
]
CN12_EVEN = [
    'PC8', 'PC6', 'PC5', '5V_USB_STLK', 'PD8', 'PA12', 'PA11', 'PB12',
    'PB11', 'GND', 'PB2', 'PB1', 'PB15', 'PB14', 'PB13', 'AGND',
    'PC4', 'PF5', 'PF4', 'PE8', 'PF10', 'PE7', 'PD14', 'PD15', 'PF14',
    'PE9', 'GND', 'PE11', 'PF3', 'PF15', 'PF11', 'PE0', 'PG8', 'PG5',
    'PG6',
]

# Supply and ground pin numbers per Table 22.  VREFP/AVDD stay plain
# (references, not supplies-- same treatment N6 gives its own VREFP/
# AREF rows); IOREF is excluded for the same reason N6's own IOREF
# pin (CN3 pin 12) is excluded-- template convention, not an
# oversight.
MORPHO_PWR = {'CN11': {5, 6, 16, 18, 24, 33}, 'CN12': {8}}
MORPHO_GND = {'CN11': {8, 19, 20, 22, 49, 60}, 'CN12': {9, 20, 32, 39, 54, 63}}

# The loopback's two morpho landing points (CN11 pin 32 = PA4, CN12
# pin 37 = PA3)-- set unconditionally, like n657x0.py's ARS_CN15;
# build() gates visibility with the `highlight` flag, not this set.
HL_CN11 = {32}
HL_CN12 = {37}

# Zio connector rows, UM2407 Rev 6 Tables 18-21 (p.40-42): each row is
# (inboard, outboard), each side (mark, mcu_pin, func, note).  D24/A0
# get a `_marked` variant in the ars variant (PA4/PA3 loopback);
# build() picks the row set per project_mode.

CN7 = [
    (('D15', 'PB8', 'I2C1_SCL', ''), ('D16', 'PC6', 'I2S2_MCK', '')),
    (('D14', 'PB9', 'I2C1_SDA', ''), ('D17', 'PB15', 'I2S2_SD', '')),
    (('VREFP', '', 'VDDA/VREFP', ''),
     ('D18', 'PB13', 'I2S2_CK (JP6 ON: RMII_TXD1)', '')),
    (('GND', '', 'Ground', ''), ('D19', 'PB12', 'I2S2_WS', '')),
    (('D13', 'PA5', 'SPI1_SCK', ''), ('D20', 'PA15', 'I2S3_WS', '')),
    (('D12', 'PA6', 'SPI1_MISO', ''), ('D21', 'PC7', 'I2S3_MCK', '')),
    (('D11', 'PB5', 'SPI1_MOSI/TIM3_CH2', ''),
     ('D22', 'PB5', 'I2S3/SPI3_SD', '')),
    (('D10', 'PD14', 'SPI1_CS/TIM4_CH3', ''),
     ('D23', 'PB3', 'I2S3/SPI3_CK', '')),
    (('D9', 'PD15', 'TIM4_CH4', ''), ('D24', 'PA4', 'SPI3_NSS', '')),
    (('D8', 'PF3', 'I/O', ''), ('D25', 'PB4', 'SPI3_MISO', '')),
]

CN8 = [
    (('NC', '', '--', ''), ('D43', 'PC8', 'SDMMC_D0', '')),
    (('IOREF', '', '3.3V ref', ''),
     ('D44', 'PC9', 'SDMMC_D1/I2S_CKIN', '')),
    (('NRST', '', 'Reset', ''), ('D45', 'PC10', 'SDMMC_D2', '')),
    (('3V3', '', '3.3V out', ''), ('D46', 'PC11', 'SDMMC_D3', '')),
    (('5V', '', '5V out', ''), ('D47', 'PC12', 'SDMMC_CK', '')),
    (('GND', '', 'Ground', ''), ('D48', 'PD2', 'SDMMC_CMD', '')),
    (('GND', '', 'Ground', ''), ('D49', 'PG2', 'I/O', '')),
    (('VIN', '', 'Power in', ''), ('D50', 'PG3', 'I/O', '')),
]

CN9 = [
    (('A0', 'PA3', 'ADC12_INP15', ''),
     ('D51', 'PD7', 'USART2_SCLK', '')),
    (('A1', 'PC0', 'ADC123_INP10', ''), ('D52', 'PD6', 'USART2_RX', '')),
    (('A2', 'PC3', 'ADC12_INP13', ''), ('D53', 'PD5', 'USART2_TX', '')),
    (('A3', 'PB1', 'ADC12_INP5', ''), ('D54', 'PD4', 'USART2_RTS', '')),
    (('A4', 'PC2/PB9', 'ADC123_INP12/I2C1_SDA', ''),
     ('D55', 'PD3', 'USART2_CTS', '')),
    (('A5', 'PF10/PB8', 'ADC3_INP6/I2C1_SCL', ''),
     ('GND', '', 'Ground', '')),
    (('D72', 'PB2', 'COMP1_INP', ''), ('D56', 'PE2', 'SAI1_A_MCLK', '')),
    (('D71', 'PE9', 'COMP2_INP', ''), ('D57', 'PE4', 'SAI1_A_FS', '')),
    (('D70', 'PF2', 'I2C2_SMBA', ''), ('D58', 'PE5', 'SAI1_A_SCK', '')),
    (('D69', 'PF1', 'I2C2_SCL', ''), ('D59', 'PE6', 'SAI1_A_SD', '')),
    (('D68', 'PF0', 'I2C2_SDA', ''), ('D60', 'PE3', 'SAI1_B_SD', '')),
    (('GND', '', 'Ground', ''), ('D61', 'PF8', 'SAI1_B_SCK', '')),
    (('D67', 'PD0', 'CAN1_RX', ''), ('D62', 'PF7', 'SAI1_B_MCLK', '')),
    (('D66', 'PD1', 'CAN1_TX', ''), ('D63', 'PF9', 'SAI1_B_FS', '')),
    (('D65', 'PG0', 'I/O', ''), ('D64', 'PG1', 'I/O', '')),
]

CN10 = [
    (('AVDD', '', 'Analog VDD', ''), ('D7', 'PG12', 'I/O', '')),
    (('AGND', '', 'Analog GND', ''), ('D6', 'PE9', 'TIM1_CH1', '')),
    (('GND', '', 'Ground', ''), ('D5', 'PE11', 'TIM1_CH2', '')),
    (('A6', 'PF4', 'ADC3_INP9', ''), ('D4', 'PE14', 'I/O', '')),
    (('A7', 'PF5', 'ADC3_INP4', ''), ('D3', 'PE13', 'TIM1_CH3', '')),
    (('A8', 'PF6', 'ADC3_INP8', ''), ('D2', 'PG14', 'I/O', '')),
    (('D26', 'PG6', 'QSPI1_NCS', ''), ('D1', 'PB6', 'LPUART1_TX', '')),
    (('D27', 'PB2', 'QSPI1_CLK', ''), ('D0', 'PB7', 'LPUART1_RX', '')),
    (('GND', '', 'Ground', ''), ('D42', 'PE8', 'TIM1_CH1N', '')),
    (('D28', 'PD13', 'QSPI1_IO', ''), ('D41', 'PE7', 'TIM1_ETR', '')),
    (('D29', 'PD12', 'QSPI1_IO', ''), ('GND', '', 'Ground', '')),
    (('D30', 'PD11', 'QSPI1_IO', ''), ('D40', 'PE10', 'TIM1_CH2N', '')),
    (('D31', 'PE2', 'QSPI1_IO (shares CN9 pin 14)', ''),
     ('D39', 'PE12', 'TIM1_CH3N', '')),
    (('GND', '', '--', ''), ('D38', 'PE6', 'TIM1_BKIN2', '')),
    (('D32', 'PA0', 'TIM2_CH1', ''), ('D37', 'PE15', 'TIM1_BKIN1', '')),
    (('D33', 'PB0', 'TIM3_CH3', ''), ('D36', 'PB10', 'TIM2_CH3', '')),
    (('D34', 'PE0', 'TIM4_ETR', ''), ('D35', 'PB11', 'TIM2_CH4', '')),
]


def _cn7_rows(project_mode):
    rows = list(CN7)
    if project_mode:
        inb, outb = rows[8]
        rows[8] = (inb, (outb[0], outb[1],
                          'SPI3_NSS (DAC1_OUT1)', 'ars'))
    return rows


def _cn9_rows(project_mode):
    rows = list(CN9)
    if project_mode:
        inb, outb = rows[0]
        rows[0] = ((inb[0], inb[1],
                    'ADC12_INP15 loopback', 'ars'), outb)
    return rows


def _board():
    return t.Board(
        name='NUCLEO-H753ZI',
        ref_design='MB1364',
        mcu_lines=('STM32', 'H753ZI'),
        width_mm=70.0,
        length_mm=133.34,
        x=310, y=70, w=566, h=1080,
        g0=210,
    )


def _morpho_strips(board, project_mode):
    cn11 = t.MorphoStrip(
        designator='CN11', x=316, side='left',
        fitted_rows=35, unfitted_rows=0, unfitted_designator='',
        odd_names=CN11_ODD, even_names=CN11_EVEN,
        pwr_pins=MORPHO_PWR['CN11'], gnd_pins=MORPHO_GND['CN11'],
        hl_pins=HL_CN11,
    )
    cn12 = t.MorphoStrip(
        designator='CN12', x=board.r - 50, side='right',
        fitted_rows=35, unfitted_rows=0, unfitted_designator='',
        odd_names=CN12_ODD, even_names=CN12_EVEN,
        pwr_pins=MORPHO_PWR['CN12'], gnd_pins=MORPHO_GND['CN12'],
        hl_pins=HL_CN12,
    )
    return [cn11, cn12]


def _headers(board, project_mode):
    # k0 offsets leave the top of both columns (y up to ~360) clear
    # of pin rows and of the marks text each header prints just
    # inside its own side (x ~ 489-560 left, ~640-710 right)-- the
    # JP1-JP5/CN2/CN5 cluster in build() lives in the center band
    # this leaves clear (x ~ 490-710, y ~ 190-330), not beside the
    # morpho strips, whose row-label text fills x ~ 354-440 (left)
    # and ~ 730-822 (right) all the way from row 0.  Bottom headers
    # start one row after the top header's last row.
    return [
        t.Header(designator='CN8', x=435, k0=9, side='left',
                 rows=CN8, columns=2),
        t.Header(designator='CN9', x=435, k0=18, side='left',
                 rows=_cn9_rows(project_mode), columns=2,
                 title='bottom'),
        t.Header(designator='CN7', x=board.r - 160, k0=9, side='right',
                 rows=_cn7_rows(project_mode), columns=2),
        t.Header(designator='CN10', x=board.r - 160, k0=20, side='right',
                 rows=CN10, columns=2, title='bottom'),
    ]


def build(variant):
    project_mode = variant == 'ars'
    board = _board()
    W = board.r + 340
    H = 1270
    doc = svg.SvgDoc(W, H)

    svg.open_doc(doc)
    t.draw_board_rect(doc, board)

    for strip in _morpho_strips(board, project_mode):
        t.draw_morpho_strip(doc, board, strip, project_mode)

    t.draw_board_label(doc, board)

    for header in _headers(board, project_mode):
        t.draw_header(doc, board, header, project_mode)

    # ST-LINK zone + embedded debugger/programmer (sect. 7.3: "The
    # embedded STLINK-V3E programming and debugging tool"; part name
    # is plain 'STLINK-V3E' on this board, unlike N6's own
    # 'STLINK-V3EC').
    t.draw_stlink_zone(
        doc, board,
        'CN1 Micro-USB: PRIMARY DEV PORT -- SWD + VCP debug, '
        '5V_USB_STLK power in',
        'flash + attach: mise run flash nucleo-h753zi',
        part_name='STLINK-V3E')

    # CN5 MIPI-10/STDC14 external debug connector (Table 5, p.20):
    # SWD (or JTAG) plus a VCP UART bridge on the same 10-pin
    # connector-- richer than N6's SWD-only MIPI20.
    t.draw_debug_conn(doc, board, t.DebugConn(
        x=344, y=100, w=22, h=70, designator='CN5',
        label=('CN5 MIPI-10: external', 'debug (SWD/JTAG + VCP)')))

    # CN2 DFU connector (Figure 4, p.14): named on the board layout
    # figure only-- no pin table or further description anywhere in
    # UM2407, so it is drawn label-only, the same treatment N6 gives
    # its own undocumented CN6 camera header.
    t.draw_small_fixture(doc, t.SmallFixture(
        x=344, y=175, w=95, h=14, label='CN2 DFU'))

    t.draw_leds(doc, board)

    # JP1-JP5 (Table 4, p.12): dev-setup defaults are already the
    # documented defaults, so nothing needs to move off-default for
    # a working dev bench, unlike N6's BOOT jumper.  Positioned in
    # the center band (x ~ 490-710, y ~ 190-330) that CN8/CN9/CN7/
    # CN10's k0 offsets and the morpho strips' full-height row-label
    # text leave clear-- see the module docstring and the k0 comment
    # in _headers() for why this band, not a side gap, is the only
    # collision-free spot for a jumper cluster on this board.
    t.draw_jumper(doc, t.Jumper(designator='JP1', kind='2pin', x=495,
                                 y=195, tag='OFF'))
    t.draw_jumper(doc, t.Jumper(designator='JP3', kind='2pin', x=565,
                                 y=195, tag='ON (shunt)'))
    t.draw_jumper(doc, t.Jumper(designator='JP4', kind='2pin', x=635,
                                 y=195, tag='ON (shunt)'))
    t.draw_jumper(doc, t.Jumper(
        designator='JP2', kind='sel2x4', x=495, y=250,
        tag='[1-2] dflt', default=0,
        positions=((0, 'STLK'), (16, 'VIN'), (32, 'EXT'), (48, 'CHGR'))))
    t.draw_jumper(doc, t.Jumper(designator='JP5', kind='3pin', x=635,
                                 y=250, tag='1-2 dflt', default=True))
    doc.emit(f'<text x="{board.cx}" y="340" font-size="9.5" '
             f'fill="{svg.MUT}" text-anchor="middle">Table 4 '
             f'defaults:</text>')
    doc.emit(f'<text x="{board.cx}" y="352" font-size="9.5" '
             f'fill="{svg.MUT}" text-anchor="middle">JP1 OFF, JP2 '
             f'[1-2], JP3/JP4 ON, JP5 [1-2]</text>')

    t.draw_mcu_box(doc, board)
    t.draw_buttons(doc, board, 'B1 USER', 'B2 RESET')
    t.draw_bottom_connectors(doc, board, 'CN13 USB', 'CN14 ETH')

    doc.emit(f'<rect x="{board.x}" y="1190" width="12" height="12" '
             f'fill="{svg.PWR}" stroke="#000" stroke-width="0.6"/>')
    doc.emit(f'<text x="{board.x + 20}" y="1200" font-size="12.5" '
             f'fill="{svg.INK}">power pin (3V3, 5V, VIN, VBAT, '
             f'5V_STLK/EXT variants)</text>')
    doc.emit(f'<rect x="{board.x + 320}" y="1190" width="12" '
             f'height="12" fill="{svg.GND_FILL}" stroke="#bbbbbb" '
             f'stroke-width="0.6"/>')
    doc.emit(f'<text x="{board.x + 340}" y="1200" font-size="12.5" '
             f'fill="{svg.INK}">ground pin (GND, AGND)</text>')
    doc.emit(f'<text x="{board.x}" y="1222" font-size="12.5" '
             f'fill="{svg.INK}">Debug hookup: CN1 alone carries SWD '
             f'and the VCP (USART3, sect. 7.6.5); CN5 MIPI-10 takes '
             f'an external probe.</text>')
    doc.emit(f'<text x="{board.x}" y="1244" font-size="11" '
             f'fill="{svg.MUT}">Zio header data: UM2407 Rev 6 Tables '
             f'18-21.  CN7/CN10 (right) carry the digital/QSPI/TIM '
             f'extension; CN8/CN9 (left)</text>')
    doc.emit(f'<text x="{board.x}" y="1258" font-size="11" '
             f'fill="{svg.MUT}">carry power + SDMMC and A0-A5/CAN.  '
             f'Morpho CN11/CN12: Table 22, both fully populated '
             f'(no unfitted footprint).</text>')

    if not project_mode:
        doc.emit('</svg>')
        return W, doc.out

    # ── ARS variant: planned DAC1_OUT1 -> ADC loopback overlay ──
    #
    # Strict superset of the base diagram above: same headers/morpho/
    # jumpers, plus only the loopback jumper and its two not-yet-
    # pinned HIL channels.  Both landing pins are established,
    # existing repo facts (docs/src/boards/nucleo-h753zi.md,
    # docs/src/projects/ars-toolhead-sensor/hil-measurements.md),
    # cited there, not re-derived here; this overlay only locates
    # them on the Zio/morpho grid (CN7 D24 / CN9 A0, Table 18/20;
    # also CN11 pin 32 / CN12 pin 37, Table 22) and marks the jumper
    # PLANNED, matching hil-measurements.md's own "provisional"/
    # "no pin beyond PA3/PA4 is fixed yet" language.

    d24_x = board.r - 160 + 28  # CN7 D24, outboard column
    d24_y = board.g0 + 17 * board.pitch  # CN7 k0=9, row index 8
    a0_x = 435 + 8  # CN9 A0, inboard column
    a0_y = board.g0 + 18 * board.pitch  # CN9 k0=18, row index 0

    # The only vertical channel free of every header's own "marks"
    # text (CN8/CN9 print theirs at x ~ 489-560, CN7/CN10 at
    # x ~ 640-710-- see the k0 comment in _headers()) is the ~80 px
    # gap between them, x ~ 560-640; it stays clear down to the MCU
    # box (x=528-658, y>=580).  Everything below-- label, both
    # placeholder channels, and the path's own bend point-- is kept
    # inside that gap.
    mid_y = 500
    _loop = (f'M {d24_x} {d24_y} L {board.cx + 37} {mid_y} '
             f'L {board.cx - 38} {mid_y} L {a0_x} {a0_y}')
    doc.emit(f'<path d="{_loop}" fill="none" stroke="{svg.HL_TXT}" '
             f'stroke-width="1.6" stroke-dasharray="7 5"/>')
    doc.emit(f'<circle cx="{d24_x}" cy="{d24_y}" r="4" fill="none" '
             f'stroke="{svg.HL_TXT}" stroke-width="1.6"/>')
    doc.emit(f'<circle cx="{a0_x}" cy="{a0_y}" r="4" fill="none" '
             f'stroke="{svg.HL_TXT}" stroke-width="1.6"/>')

    for y, line in ((375, 'PLANNED loopback:'),
                    (388, 'PA4 (D24, CN7)'),
                    (401, '&lt;-&gt; PA3 (A0, CN9)'),
                    (414, 'not yet wired')):
        doc.emit(f'<text x="{board.cx}" y="{y}" font-size="9.5" '
                 f'fill="{svg.HL_TXT}" font-weight="bold" '
                 f'text-anchor="middle">{line}</text>')

    # Two HIL channels the loopback plan names but does not yet pin
    # (hil-measurements.md, "Planned H753ZI Loopback Measurements"):
    # drawn as dashed, unfilled boxes-- explicitly not a pin, not a
    # position, just a named concept awaiting audio_loopback.
    for y, label in ((432, 'TIM trigger:'), (450, 'pin TBD'),
                     (468, 'capture strobe:'), (486, 'pin TBD')):
        doc.emit(f'<text x="{board.cx}" y="{y}" font-size="9" '
                 f'fill="{svg.HL_TXT}" text-anchor="middle">{label}'
                 f'</text>')
    doc.emit(f'<rect x="{board.cx - 42}" y="422" width="84" '
             f'height="38" rx="3" fill="none" stroke="{svg.HL_TXT}" '
             f'stroke-width="1.2" stroke-dasharray="4 3"/>')
    doc.emit(f'<rect x="{board.cx - 42}" y="460" width="84" '
             f'height="38" rx="3" fill="none" stroke="{svg.HL_TXT}" '
             f'stroke-width="1.2" stroke-dasharray="4 3"/>')

    doc.emit('</svg>')
    return W, doc.out
