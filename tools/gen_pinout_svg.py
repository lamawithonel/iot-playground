#!/usr/bin/env python3
"""Generate every registered board pinout SVG.

Run from anywhere: `python3 tools/gen_pinout_svg.py`.  The checked-in
SVGs under docs/src/ are generated artifacts-- edit the board/template
modules under tools/pinout/ and regenerate, never the SVGs by hand.

The drawing logic lives in tools/pinout/: nucleo144.py is the
Nucleo-144 form-factor template (generic drawing primitives shared by
every board built on that outline); tools/pinout/boards/*.py hold each
board's data-- pin tables, silk names, fixture positions-- plus any
per-project overlay (e.g. n657x0.py's ARS toolhead-sensor variant).
tools/pinout/__init__.py registers which (board, variant) pairs get
written to which path; add a board or variant there to include it in
this script's output.
"""

import os
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, REPO)

from tools.pinout import REGISTRY  # noqa: E402 (path setup must run first)

for module, variant, rel_path in REGISTRY:
    w, lines = module.build(variant)
    path = os.path.join(REPO, rel_path)
    with open(path, 'w') as f:
        f.write('\n'.join(lines) + '\n')
    print('wrote', path, f'({w}px wide, {len(lines)} elements)')
