"""Board registry for pinout SVG generation.

Each entry is (module, variant, output path relative to the repo
root).  tools/gen_pinout_svg.py imports REGISTRY and regenerates
every entry-- registering a new board or variant here is enough to
make it part of that script's output.  module.build(variant) must
return (width_px, lines), where lines is the list of SVG element
strings to join and write.
"""

from .boards import feather_f405, h753zi, n657x0

REGISTRY = (
    (n657x0, 'board', 'docs/src/boards/nucleo-n657x0-pinout.svg'),
    (n657x0, 'ars', 'docs/src/projects/ars-toolhead-sensor/'
                     'nucleo-n657x0-ars-pinout.svg'),
    (h753zi, 'board', 'docs/src/boards/nucleo-h753zi-pinout.svg'),
    (h753zi, 'ars', 'docs/src/projects/ars-toolhead-sensor/'
                     'nucleo-h753zi-ars-pinout.svg'),
    (feather_f405, 'board', 'docs/src/boards/feather-stm32f405-pinout.svg'),
    (feather_f405, 'project', 'docs/src/boards/'
                     'feather-stm32f405-project-pinout.svg'),
)
