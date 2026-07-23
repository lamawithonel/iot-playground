"""NUCLEO-N657X0-Q pinout board data and ARS-project overlay.

Two variants from one board drawing, per the docs split:
  board -- base board, debug hookup only (board page)
  ars   -- ARS project: debug + peripheral + HIL hookups (pinout.md)

Geometry mirrors the MB1940C top overlay (schematic p.19): Arduino
power+analog headers left (CN5, CN4), digital right (CN14, CN13),
morpho strips outside them (CN3, CN15), ST-LINK and USB-C top,
USER/RESET bottom corners, USB + Ethernet bottom, MCU center.
Header data: UM3417 Rev 3, Table 12.  ARS assignments mirror
docs/src/projects/ars-toolhead-sensor/pinout.md, which is the
authority-- update both together.

House rule: a board drawing always uses the documented board's real
aspect ratio, from its manual-- here 133.34 x 70 mm (UM3417,
Nucleo-144 form factor), so the board rect is 566 x 1080.

Everything in this module is N657X0Q/MB1940-specific data, plus (in
the second half of build()) the ARS toolhead-sensor project's own
one-off overlay-- HIL taps, the external audio-chain boxes, arrows,
and the ARS legend.  None of that belongs in the Nucleo-144 template
(tools/pinout/nucleo144.py): it is specific to this one project, not
a reusable form-factor fixture.
"""

from .. import nucleo144 as t
from .. import svg

# ST morpho per-pin names (UM3417 Rev 3, Table 13).  Odd pins are
# the inner column on the top view.  HSDM/HSDP follow the board's
# own silk abbreviation for OTG_HSDM_CON/OTG_HSDP_CON.
CN3_ODD = ['NC', 'PC2', 'VDDIO', 'BOOT0', 'PH0', 'PH1', 'PC4',
           'PC5', 'PC0', 'GND', 'PE4', 'PC13', 'PC3', 'PC15',
           'PC14', 'VDDIO5', 'VBAT', 'BOOT1', 'PC8']
CN3_EVEN = ['HSDM', 'HSDP', '5V', 'GND', 'VDDIO4', 'IOREF', 'NRST',
            '3V3', '5V', 'GND', 'GND', 'VIN', 'PH4', 'PF5', 'PC10',
            'PF6', 'PA2', 'PC12', 'PH2']
CN15_ODD = ['PE12', 'PH9', 'PC1', 'VREFP', 'GND', 'PE15', 'PG1',
            'PG2', 'PA3', 'PD7', 'PD12', 'PE11', 'PD5', 'PE10',
            'PE0', 'PE9', 'PD0', 'PD8', 'PD9']
CN15_EVEN = ['PE6', 'PE5', 'PD6', '5V_STLK', 'PB2', 'PB3', 'PB8',
             'PB9', 'PB14', 'GND', 'PE7', 'PE1', 'PE14', 'PE13',
             'PE2', 'AGND', 'PD10', 'PD11', 'PA1']

# Supply and ground pins per UM3417 Table 13 (VDDIO*/VBAT count as
# supplies; VREFP/IOREF are references and stay plain)
MORPHO_PWR = {'CN3': {5, 6, 10, 16, 18, 24, 31, 33}, 'CN15': {8}}
MORPHO_GND = {'CN3': {8, 19, 20, 22}, 'CN15': {9, 20, 32}}
ARS_CN15 = {3, 5, 31, 33, 38}  # PH9/PC1/PE9/PD0/PA1, Table 13
HIL_VIOLET = {'CN3': {23, 14}}  # PC13 (ch H), NRST (ch G)

# (mark, mcu_pin, func, ars_note)
CN14 = [
    ('D15', 'PH9', 'I2C1_SCL', 'amp SCL'),
    ('D14', 'PC1', 'I2C1_SDA', 'amp SDA'),
    ('AREF', '', 'AVDD', ''),
    ('GND', '', 'Ground', ''),
    ('D13', 'PE15', 'SPI5_SCK', ''),
    ('D12', 'PG1', 'SPI5_MISO', ''),
    ('D11', 'PG2', 'SPI5_MOSI/T14', ''),
    ('D10', 'PA3', 'SPI5_CS/T16', ''),
    ('D9', 'PD7', 'TIM1_CH2', ''),
    ('D8', 'PD12', '--', ''),
]
CN13 = [
    ('D7', 'PE11', '--', ''),
    ('D6', 'PD5', 'TIM1_CH4N', ''),
    ('D5', 'PE10', 'TIM1_CH2N', ''),
    ('D4', 'PE0', '--', ''),
    ('D3', 'PE9', 'TIM1_CH1', 'audio PWM'),
    ('D2', 'PD0', 'GPIO', 'amp mute'),
    ('D1', 'PD8', 'USART3_TX', ''),
    ('D0', 'PD9', 'USART3_RX', ''),
]
CN5 = [
    ('NC', '', '5V_IN test', ''),
    ('IOREF', '', '3V3 ref', ''),
    ('RST', 'NRST', 'Reset', ''),
    ('3V3', '', '3.3V out', ''),
    ('5V', '', '5V out', ''),
    ('GND', '', 'Ground', ''),
    ('GND', '', 'Ground', ''),
    ('VIN', '', 'Power in', ''),
]
CN4 = [
    ('A0', 'PA8', 'ADC12_INP5', 'mic in'),
    ('A1', 'PA9', 'ADC12_INP10', ''),
    ('A2', 'PA10', 'ADC12_INP11', ''),
    ('A3', 'PA12', 'ADC12_INP13', ''),
    ('A4', 'PF3', 'ADC1_INP16', ''),
    ('A5', 'PG15', 'ADC12_INP7', ''),
]


def _board():
    return t.Board(
        name='NUCLEO-N657X0-Q',
        ref_design='MB1940',
        mcu_lines=('STM32', 'N657X0Q'),
        width_mm=70.0,
        length_mm=133.34,
        x=310, y=70, w=566, h=1080,
        g0=210,
    )


def _morpho_strips(board):
    # The populated 2x19 male headers (CN3/CN15) cover only the upper
    # half of two full-length through-hole columns; the lower
    # continuation is the unfitted CN2/CN16 footprint (UM3417 Rev 3,
    # Figure 15, p.31; sect. 8.2).
    cn3 = t.MorphoStrip(
        designator='CN3', x=316, side='left',
        fitted_rows=19, unfitted_rows=17, unfitted_designator='CN2',
        odd_names=CN3_ODD, even_names=CN3_EVEN,
        pwr_pins=MORPHO_PWR['CN3'], gnd_pins=MORPHO_GND['CN3'],
        hil_pins=HIL_VIOLET['CN3'],
    )
    cn15 = t.MorphoStrip(
        designator='CN15', x=board.r - 50, side='right',
        fitted_rows=19, unfitted_rows=17, unfitted_designator='CN16',
        odd_names=CN15_ODD, even_names=CN15_EVEN,
        pwr_pins=MORPHO_PWR['CN15'], gnd_pins=MORPHO_GND['CN15'],
        ars_pins=ARS_CN15,
    )
    return [cn3, cn15]


def _headers(board, ars_mode):
    cn5_rows = CN5
    if ars_mode:
        # Beginner-explicit power wiring: name the loads on the
        # supply rows instead of drawing more cross-board lines.
        cn5_rows = []
        gnd_done = False
        for mark, pin, func, note in CN5:
            if mark == '3V3':
                func = '3.3V out -> mic VCC + amp VDDIO'
            elif func == 'Ground' and not gnd_done:
                func = 'Ground -> mic + amp + PVDD common'
                gnd_done = True
            cn5_rows.append((mark, pin, func, note))

    return [
        t.Header(designator='CN14', x=board.r - 160, k0=0,
                 side='right', rows=CN14),
        t.Header(designator='CN13', x=board.r - 160, k0=11,
                 side='right', rows=CN13, title='bottom'),
        t.Header(designator='CN5', x=435, k0=4, side='left',
                 rows=cn5_rows),
        t.Header(designator='CN4', x=435, k0=13, side='left',
                 rows=CN4),
    ]


def build(variant):
    ars_mode = variant == 'ars'
    board = _board()
    W = (board.r + 624) if ars_mode else (board.r + 314)
    H = 1270
    doc = svg.SvgDoc(W, H)

    svg.open_doc(doc)
    t.draw_board_rect(doc, board)

    for strip in _morpho_strips(board):
        t.draw_morpho_strip(doc, board, strip, ars_mode)

    t.draw_board_label(doc, board)

    for header in _headers(board, ars_mode):
        t.draw_header(doc, board, header, ars_mode)

    # ST-LINK zone + USB-C
    t.draw_stlink_zone(
        doc, board,
        'CN10 USB-C: PRIMARY DEV PORT -- SWD + RTT debug, VCP '
        'console, 5V_STLK power in',
        'flash + attach: mise run flash nucleo-n657x0 [--project '
        'net]')
    t.draw_debug_conn(doc, board, t.DebugConn(
        x=344, y=100, w=22, h=70, designator='CN1',
        label=('CN1 MIPI20: external', 'debug probe (SWD/trace)')))
    t.draw_leds(doc, board)

    # BOOT jumpers, drawn in their dev-setup states.  Pin 1 is at
    # the bottom; UM3417 defaults are JP1 [1-2] and JP2 [1-2]
    # (flash boot), so dev boot puts JP2's cap on the UPPER pair
    # (2-3) while JP1 keeps the lower default.
    #
    # x per UM3417 Rev 3 Figure 3 (p.8) / Figure 5 (p.10): JP1 sits
    # ~21mm from the left board edge, JP2 ~25mm-- both well left of
    # CN9, not flush against it.  Y stays put: the manual's ~32mm-
    # down anchor lands inside the CN5 header box (y=279-443, row
    # labels starting x=469), so pushing this cluster down to match
    # would overlap CN5's pin names.  The current y already sits at
    # the top of that gap (block bottom ~274, CN5 starts at 279), so
    # only x moves.
    t.draw_jumper(doc, t.Jumper(designator='JP1', kind='3pin', x=480,
                                 y=208, tag='1-2 dflt', default=False))
    t.draw_jumper(doc, t.Jumper(designator='JP2', kind='3pin', x=512,
                                 y=208, tag='2-3 DEV', default=True))

    # CN9 power-source selector, capped on its 5V_STLK default so
    # the board powers from the ST-LINK USB during development.
    # x per UM3417 Rev 3 Figure 3 (p.8): CN9 sits ~33mm from the left board
    # edge, measurably right of JP1/JP2 (~21/25mm)-- not flush
    # against them.  Same y-collision reasoning as JP1/JP2 above:
    # y is unchanged.
    t.draw_jumper(doc, t.Jumper(
        designator='CN9 PWR SRC', kind='sel3', x=562, y=208,
        tag='1-2 dflt', default=0,
        positions=((0, '5V_STLK'), (18, 'USB_SNK'), (36, 'VIN'))))
    doc.emit(f'<text x="296" y="218" font-size="11" fill="{svg.MUT}" '
             f'text-anchor="end">JP2 BOOT1 capped 2-3 (upper) = dev</text>')
    doc.emit(f'<text x="296" y="232" font-size="11" fill="{svg.MUT}" '
             f'text-anchor="end">boot; JP1 + CN9 stay at defaults.</text>')
    doc.emit(f'<text x="296" y="246" font-size="11" fill="{svg.MUT}" '
             f'text-anchor="end">BOOT latches at reset-- power-cycle</text>')
    doc.emit(f'<text x="296" y="260" font-size="11" fill="{svg.MUT}" '
             f'text-anchor="end">after moving a cap.</text>')

    # CN12 power-test header (UM3417 Rev 3 Figure 5, p.10; Table 7 / Table 8,
    # p.20-21; schematic pins PICN1201-14).  The original bug report called this
    # a JTAG/SWD header-- it is not.  CN12 is a 14-pin (2x7), always-fitted
    # current-measurement header: each of its 7 columns is a pin-pair (EXT_IN,
    # EVCORE, INT_IN, GND, INT_VCORE, VDDIO, VDDA1V8) bridged elsewhere on the
    # board by a 0-ohm shunt; removing that shunt and bridging an ammeter here
    # instead measures the rail's current.  It is not a jumper, so no cap is
    # drawn.  The board's only debug connector remains CN1/MIPI20, drawn above
    # with the ST-LINK zone.  x/y measured off the UM3417 p.8/p.10 photos
    # (pixel-grid method, cross-checked against JP1/JP2/CN9/CN15's
    # already-verified positions-- see .cache/agents/pinout-phase1/); nudged a
    # few px right of the ST-LINK zone's edge (x=736) and left of the LEDs label
    # (x=831) to clear both without moving either.  Only one on-diagram label
    # line fits above CN15's row-label band (which starts at y=197, same as
    # CN14/CN15's header top)-- the fuller citation moves to the right margin
    # below, clear of the morpho per-pin labels that would otherwise run under a
    # second line.
    t.draw_power_test_header(doc, t.PowerTestHeader(
        x=745, y=162, designator='CN12'))

    # JP3 ST-LINK reset jumper (UM3417 Rev 3 Table 4, p.8: silkscreen STLK_RST,
    # default OFF).  Drawn as a 2-pin, uncapped header-- OFF means no shunt
    # fitted, unlike JP1/JP2/CN9 which are always capped somewhere.  CN15's
    # per-row pin labels run text-anchor "end" out of x=822 for every one of its
    # 19 rows (see the morpho loop above), so they occupy roughly x=786-822 all
    # the way down the header-- there is no y that clears them in that band.
    # JP3 goes in the narrower gap between CN14's pad column (ends x=736) and
    # that label band instead, not at its photo x, to stay clear of both.
    t.draw_jumper(doc, t.Jumper(designator='JP3', kind='2pin', x=753,
                                 y=213, tag='OFF'))

    # CN12/JP3 citation, right margin: clear of everything above
    # (nothing else is drawn at x >= BOARD_R+14 above y=210) so the
    # full correction and default state fit without truncation.  In
    # ars mode, the debug-hookup sentence and the morpho/amp notes
    # reuse this same column further down the page, past y=210; the
    # base variant carries its debug-hookup sentence in the bottom
    # legend instead (see the `if not ars_mode:` block below).
    doc.emit(f'<text x="{board.r + 14}" y="160" font-size="11" '
             f'fill="{svg.MUT}">CN12 power test (UM3417 Table 7/8):</text>')
    doc.emit(f'<text x="{board.r + 14}" y="174" font-size="11" '
             f'fill="{svg.MUT}">14-pin, always fitted, ammeter tap</text>')
    doc.emit(f'<text x="{board.r + 14}" y="188" font-size="11" '
             f'fill="{svg.MUT}">not JTAG/SWD; CN1/MIPI20 is the debug '
             f'port</text>')
    doc.emit(f'<text x="{board.r + 14}" y="202" font-size="11" '
             f'fill="{svg.MUT}">JP3 STLK_RST: 2-pin, default OFF (open)'
             f'</text>')

    t.draw_mcu_box(doc, board)
    t.draw_small_fixture(doc, t.SmallFixture(
        x=352, y=760, w=120, h=14, label='CN6 camera'))
    t.draw_buttons(doc, board, 'B1 USER', 'B2 RESET')
    t.draw_bottom_connectors(doc, board, 'CN8 USB', 'CN11 ETH')

    doc.emit('<line x1="356" y1="1078" x2="300" y2="1078" '
             'stroke="#b5b5b0"/>')
    if ars_mode:
        doc.emit(f'<text x="296" y="1074" font-size="12.5" '
                 f'fill="{svg.ARS_TXT}" font-weight="bold" '
                 f'text-anchor="end">PC13  EXTI13  (user button) *'
                 f'</text>')
        doc.emit(f'<text x="296" y="1090" font-size="11" '
                 f'fill="{svg.MUT}" text-anchor="end">spare EXTI, no '
                 f'wiring</text>')
    else:
        doc.emit(f'<text x="296" y="1074" font-size="12.5" '
                 f'fill="{svg.INK}" text-anchor="end">PC13  EXTI13'
                 f'</text>')
    doc.emit(f'<line x1="{board.cx + 96}" y1="1123" x2="{board.r + 10}" '
             f'y2="1123" stroke="#b5b5b0"/>')
    doc.emit(f'<text x="{board.r + 14}" y="1119" font-size="12.5" '
             f'fill="{svg.INK}">on-chip ETH1 RMII, LAN8742A PHY</text>')
    doc.emit(f'<text x="{board.r + 14}" y="1135" font-size="11" '
             f'fill="{svg.MUT}">net feature: DHCP + SNTP + MQTT/TLS</text>')
    if ars_mode:
        doc.emit(f'<text x="{board.r + 14}" y="1150" font-size="11" '
                 f'fill="{svg.MUT}">CNN stage reuses the mic + Ethernet '
                 f'paths: no extra wiring</text>')

    if not ars_mode:
        doc.emit(f'<line x1="{board.r - 6}" y1="592" x2="{board.r + 10}" '
                 f'y2="770" stroke="#b5b5b0"/>')
        doc.emit(f'<text x="{board.r + 14}" y="766" font-size="11" '
                 f'fill="{svg.MUT}">ST morpho CN3/CN15: most remaining '
                 f'I/O</text>')
        doc.emit(f'<text x="{board.r + 14}" y="780" font-size="11" '
                 f'fill="{svg.MUT}">(UM3417 Tables 13-14)</text>')
        doc.emit(f'<rect x="310" y="1170" width="12" height="12" '
                 f'fill="{svg.PWR}" stroke="#000" stroke-width="0.6"/>')
        doc.emit(f'<text x="330" y="1180" font-size="12.5" '
                 f'fill="{svg.INK}">power pin (3V3, 5V, VIN, VDDIO, '
                 f'VBAT)</text>')
        doc.emit(f'<rect x="620" y="1170" width="12" height="12" '
                 f'fill="{svg.GND_FILL}" stroke="#bbbbbb" '
                 f'stroke-width="0.6"/>')
        doc.emit(f'<text x="640" y="1180" font-size="12.5" '
                 f'fill="{svg.INK}">ground pin (GND, AGND)</text>')
        doc.emit(f'<text x="310" y="1200" font-size="12.5" '
                 f'fill="{svg.INK}">Debug hookup: CN10 USB-C alone '
                 f'carries SWD, RTT, and the VCP; CN1 takes an '
                 f'external probe.</text>')
        doc.emit(f'<text x="310" y="1222" font-size="11" '
                 f'fill="{svg.MUT}">Arduino V3 header data: UM3417 '
                 f'Rev 3, Table 12.  A0-A5 route through the on-board '
                 f'3.3V-to-1.8V</text>')
        doc.emit(f'<text x="310" y="1237" font-size="11" '
                 f'fill="{svg.MUT}">adaptation amplifier; the '
                 f'1.8V-domain ADC pin is shown.  Morpho rows: '
                 f'odd/even pin names, Table 13.</text>')
        doc.emit('</svg>')
        return W, doc.out

    # ── ARS variant: morpho routing, external chain, HIL taps ──

    # The base variant's debug-hookup sentence and general morpho
    # callout are dev-setup content, not ARS-specific-- carry both
    # over verbatim (Phase 1 review finding) so the ARS diagram
    # stays a strict superset of the base page instead of trading
    # that context away for amp-routing detail.  y=610 sits in the
    # one gap this column has free of CN14/CN13's row labels (which
    # run x=BOARD_R+14 from y~210 to y~590) and the morpho notes
    # below (y>=796)-- no leader line, matching the CN12/JP3 citation
    # above, since neither of those is drawn with one either.
    doc.emit(f'<text x="{board.r + 14}" y="616" font-size="11" '
             f'fill="{svg.MUT}">Debug hookup: CN10 USB-C alone '
             f'carries SWD, RTT, and the</text>')
    doc.emit(f'<text x="{board.r + 14}" y="630" font-size="11" '
             f'fill="{svg.MUT}">VCP; CN1 takes an external probe.'
             f'</text>')

    doc.emit(f'<text x="{board.r + 14}" y="796" font-size="11" '
             f'fill="{svg.MUT}">ST morpho CN3/CN15: most remaining '
             f'I/O</text>')
    doc.emit(f'<text x="{board.r + 14}" y="810" font-size="11" '
             f'fill="{svg.MUT}">(UM3417 Tables 13-14)</text>')
    doc.emit(f'<line x1="{board.r - 6}" y1="592" x2="{board.r + 10}" '
             f'y2="830" stroke="#b5b5b0"/>')
    doc.emit(f'<text x="{board.r + 14}" y="826" font-size="11" '
             f'fill="{svg.MUT}">morpho CN15 is the routing of record '
             f'for the amp:</text>')
    doc.emit(f'<text x="{board.r + 14}" y="840" font-size="11" '
             f'fill="{svg.MUT}">pin 3 PH9 SCL, pin 5 PC1 SDA, pin 31 '
             f'PE9, pin 33 PD0;</text>')
    doc.emit(f'<text x="{board.r + 14}" y="854" font-size="11" '
             f'fill="{svg.MUT}">pin 38 PA1 = G2 fallback ADC tap'
             f'</text>')

    # External audio chain, right column, aligned to the D3 row
    d3_y = board.g0 + 15 * board.pitch  # CN13 D3 row (k = 11 + 4);
    # level with morpho pin 31 (PE9), its electrical twin
    x0 = board.r + 314
    svg.ext_box(doc, x0, d3_y - 30, 250, 60,
                ['RC low-pass', 'passive, values at gate G3'])
    svg.ext_box(doc, x0, 700, 250, 92,
                ['MAX9744 class-D amp', 'LEFTIN = JP2.2, SDA = JP2.4',
                 'MUTE_INV = JP2.8 (low = mute)',
                 'VDDIO = Nucleo 3V3, I2C 0x4B'])
    svg.ext_box(doc, x0, 832, 250, 60,
                ['EX25VT2-4 exciter', 'clamped to the H2C toolhead'])
    svg.ext_box(doc, x0, 932, 250, 76,
                ['external PVDD supply', '4.5-14 V, never the Nucleo',
                 'grounds commoned'], dashed=True)
    # Mic feeds A0, so it lives on the left, under the CN4 labels
    svg.ext_box(doc, 40, 740, 250, 76,
                ['MEMS mic breakout', 'SPH8878LR5H-1 (modified)',
                 'VCC = 3V3, AUD = analog out'])

    # D3 (PE9) -> RC -> amp -> exciter; PVDD up into the amp
    svg.arrow(doc, board.r + 202, d3_y, x0, d3_y)
    svg.arrow(doc, x0 + 125, d3_y + 30, x0 + 125, 700)
    svg.arrow(doc, x0 + 125, 792, x0 + 125, 832)
    # PVDD feeds the amp; route up the clear lane left of the chain
    doc.emit(f'<line x1="{x0 + 10}" y1="945" x2="{x0 - 15}" y2="945" '
             f'stroke="#666666" stroke-width="1.6"/>')
    doc.emit(f'<line x1="{x0 - 15}" y1="945" x2="{x0 - 15}" y2="772" '
             f'stroke="#666666" stroke-width="1.6"/>')
    svg.arrow(doc, x0 - 15, 772, x0, 772)
    # I2C + mute leave on the morpho strip, straight into the amp
    # The amp wiring leaves the populated CN15 header (pins 3/5/33),
    # not the unfitted footprint below it: drop down from the header
    # bottom between the hole columns, then run out to the amp.
    doc.emit(f'<line x1="{board.r - 33}" y1="596" x2="{board.r - 33}" '
             f'y2="746" stroke="#666666" stroke-width="1.6"/>')
    svg.arrow(doc, board.r - 33, 746, x0, 746)
    doc.emit(f'<text x="{(board.r + x0) // 2}" y="738" font-size="10.5" '
             f'fill="{svg.MUT}" text-anchor="middle">I2C1 + MUTE_N '
             f'(morpho CN15 pins 3/5/33)</text>')
    # Mic AUD up the clear left margin into the A0 row; heads only
    # on the final segment, matching the PVDD lane
    a0_y = board.g0 + 13 * board.pitch  # CN4 A0 row
    doc.emit('<line x1="165" y1="740" x2="80" y2="740" stroke="#666666" '
             'stroke-width="1.6"/>')
    doc.emit(f'<line x1="80" y1="740" x2="80" y2="{a0_y}" '
             f'stroke="#666666" stroke-width="1.6"/>')
    svg.arrow(doc, 80, a0_y, 108, a0_y)
    doc.emit(f'<text x="72" y="690" font-size="10.5" fill="{svg.MUT}" '
             f'text-anchor="middle" transform="rotate(-90 72 690)">AUD '
             f'(analog)</text>')

    # HIL taps (hil-measurements.md, gate G3).  Circled letters
    # only; the legend decodes them.  Floating per-tap labels
    # crowded the chain and read as part of it.
    svg.hil_tap(doc, x0 - 54, d3_y, 'A')
    svg.hil_tap(doc, x0 + 125, 682, 'B')
    # Digital channels on their nets: E rides the D3 feed line with
    # tap A; F sits on the D2 row; G on the CN5 RST row; H points
    # at morpho CN3 pin 23, where PC13 is probeable (Table 13)
    svg.hil_dig(doc, 1100, d3_y, 'E')
    svg.hil_dig(doc, 1100, board.g0 + 16 * board.pitch, 'F')
    svg.hil_dig(doc, 150, board.g0 + 6 * board.pitch, 'G')
    svg.hil_dig(doc, 270, board.g0 + 12 * board.pitch, 'H')
    doc.emit(f'<line x1="280" y1="{board.g0 + 12 * board.pitch - 4}" '
             f'x2="316" y2="{board.g0 + 11 * board.pitch}" '
             f'stroke="#b5b5b0"/>')
    svg.hil_tap(doc, board.r + 170, 746, 'C')
    svg.hil_tap(doc, 80, 615, 'D')

    # Acoustic loop closure: the stimulus is firmware-synthesized
    # (TIM1 PWM sweep); the loop returns acoustically through the
    # clamped toolhead into the mic.  Dashed and gray: it is the
    # plant under test, not instrument wiring.  Routed around the
    # board through the clear lane below it.
    _loop = (f'M {x0 + 250} 862 H 1465 V 1168 H 100 V 830')
    doc.emit(f'<path d="{_loop}" fill="none" stroke="{svg.MUT}" '
             f'stroke-width="1.4" stroke-dasharray="7 5"/>')
    doc.emit(f'<line x1="100" y1="830" x2="128" y2="818" '
             f'stroke="{svg.MUT}" stroke-width="1.4"/>')
    doc.emit(f'<line x1="100" y1="830" x2="112" y2="842" '
             f'stroke="{svg.MUT}" stroke-width="1.4"/>')
    doc.emit(f'<text x="780" y="1162" font-size="10.5" fill="{svg.MUT}" '
             f'text-anchor="middle">acoustic path: exciter drives the '
             f'clamped H2C toolhead; the mic captures its response '
             f'(closed loop)</text>')

    # Legend
    doc.emit(f'<rect x="{board.x}" y="1190" width="12" height="12" '
             f'fill="{svg.ARS}" stroke="#000" stroke-width="0.6"/>')
    doc.emit(f'<text x="{board.x + 20}" y="1200" font-size="12.5" '
             f'fill="{svg.INK}">* ARS-assigned signal (provisional, '
             f'gates G0-G5) -- pinout.md is the authority</text>')
    doc.emit('<rect x="900" y="1190" width="12" height="12" '
             f'fill="{svg.PWR}" stroke="#000" stroke-width="0.6"/>')
    doc.emit(f'<text x="920" y="1200" font-size="12.5" '
             f'fill="{svg.INK}">power pin</text>')
    doc.emit('<rect x="1020" y="1190" width="12" height="12" '
             f'fill="{svg.GND_FILL}" stroke="#bbbbbb" '
             f'stroke-width="0.6"/>')
    doc.emit(f'<text x="1040" y="1200" font-size="12.5" '
             f'fill="{svg.INK}">ground pin</text>')
    doc.emit(f'<text x="{board.x}" y="1222" font-size="12.5" '
             f'fill="{svg.INK}">HIL analyzer taps, gate G3 '
             f'(hil-measurements.md):</text>')
    for lx, letter, desc in (
        (640, 'A', 'PWM carrier, pre-RC'),
        (850, 'B', 'filtered line, post-RC'),
        (1070, 'C', 'I2C decode, SCL/SDA'),
        (1280, 'D', 'mic line, into A0'),
    ):
        doc.emit(f'<circle cx="{lx}" cy="1218" r="9" fill="#ffffff" '
                 f'stroke="{svg.HIL}" stroke-width="2"/>')
        doc.emit(f'<text x="{lx}" y="1222" font-size="11" '
                 f'fill="{svg.HIL}" font-weight="bold" '
                 f'text-anchor="middle">{letter}</text>')
        doc.emit(f'<text x="{lx + 16}" y="1222" font-size="12.5" '
                 f'fill="{svg.INK}">{desc}</text>')
    doc.emit('<rect x="316" y="1232" width="16" height="16" rx="3" '
             f'fill="#ffffff" stroke="{svg.HIL}" stroke-width="2"/>')
    doc.emit(f'<text x="324" y="1244" font-size="11" fill="{svg.HIL}" '
             f'font-weight="bold" text-anchor="middle">E</text>')
    doc.emit(f'<text x="340" y="1244" font-size="12.5" '
             f'fill="{svg.INK}">MSO digital channels: C = SCL + SDA '
             f'(2), E = PE9 carrier edges, F = PD0 mute, G = NRST, H = '
             f'PC13 (morpho CN3 pin 23) -- six of eight; analog '
             f'(circles) is 2 per run</text>')
    doc.emit(f'<text x="{board.x}" y="1264" font-size="11" '
             f'fill="{svg.MUT}">Arduino V3 header data: UM3417 Rev 3, '
             f'Table 12.  A0-A5 route through the on-board '
             f'3.3V-to-1.8V adaptation amplifier; the 1.8V-domain ADC '
             f'pin is shown.  Morpho rows: odd/even names, Table 13.'
             f'</text>')
    doc.emit('</svg>')
    return W, doc.out
