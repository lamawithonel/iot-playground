#!/usr/bin/env python3
"""RAM-boot loader for the NUCLEO-N657X0-Q: ``flash.py <elf>``.

Load path of record while ``probe-rs run``/``download`` wedge this
board's onboard STLINK-V3EC (upstream bulk-transport bug): a real
system reset, a bulk load through the probe-rs gdb server, SP/PC
from the vector table, then a DHCSR write to release the core.
docs/src/boards/nucleo-n657x0.md ("Bring-Up: RAM-Boot Dev Flow") is
the authority on the sequence; this script transcribes it.

The reset step is load-bearing: a failed load can leave the core
halted inside an active Secure HardFault handler, and an image
started in that state runs at execution priority -1, so no
configurable interrupt is ever taken.  Only a real reset clears the
active-exception state; the SHCSR check below verifies it did.

Post-reset gotcha: DBGMCU_CR reverts to 0, so firmware that WFIs
gates the debug clock and later AP accesses fail with SwdApFault.
Recover with ``probe-rs reset --connect-under-reset`` (it may report
a connect error yet still pulse NRST).

Environment:
  PROBE_RS_PROBE  probe selector (VID:PID[:SERIAL]); required on a
                  multi-probe bench, else probe-rs prompts
  GDB             gdb binary (default: gdb; arm-none-eabi-gdb works)

Usage: flash.py <elf> [--no-run]
"""

import os
import shutil
import subprocess
import sys
import time

CHIP = "STM32N657"
# --speed 100 works around the onboard STLINK-V3EC clocking bug in
# probe-rs (it still clamps to 50 kHz; see the board page).
PROBE_ARGS = ["--chip", CHIP, "--speed", "100"]
if os.environ.get("PROBE_RS_PROBE"):
    PROBE_ARGS += ["--probe", os.environ["PROBE_RS_PROBE"]]
GDB = os.environ.get("GDB", "gdb")
GDB_PORT = "1337"  # probe-rs gdb default

# Vector table base = FLASH ORIGIN in memory.x; the two files must
# move together.
VECTOR_BASE = "0x341A0000"

DHCSR = "0xE000EDF0"
SHCSR = "0xE000ED24"
AIRCR = "0xE000ED0C"
DBGKEY_RUN = "0xA05F0001"  # key | C_DEBUGEN, C_HALT=0
AIRCR_SYSRESETREQ = "0x05FA0004"  # VECTKEY | SYSRESETREQ


def probe_rs(*args, timeout=30):
    return subprocess.run(
        ["probe-rs", *args, *PROBE_ARGS],
        capture_output=True, text=True, timeout=timeout,
    )


def system_reset():
    """Real reset before load; the only thing that clears inherited
    active-exception state (see module docstring).  SYSRESETREQ via
    AIRCR needs a live AP; if the AP is dead (target WFI'd with
    DBGMCU at reset state), fall back to NRST via
    connect-under-reset, which pulses the pin even when its own
    connect step then reports failure."""
    r = probe_rs("write", "b32", AIRCR, AIRCR_SYSRESETREQ)
    if r.returncode != 0:
        probe_rs("reset", "--connect-under-reset", timeout=60)
    time.sleep(2)
    v = probe_rs("read", "b32", SHCSR, "1")
    try:
        shcsr = v.stdout.strip().split()[-1] if v.returncode == 0 else None
        active = shcsr is None or int(shcsr, 16) & 0xFFF != 0
    except (IndexError, ValueError):
        shcsr, active = repr(v.stdout.strip()), True
    if active:
        print("RESET DID NOT CLEAR ACTIVE-EXCEPTION STATE:", shcsr)
        raise SystemExit(1)
    print("reset ok; SHCSR =", shcsr)


def gdb_load(elf):
    server = subprocess.Popen(
        ["probe-rs", "gdb", *PROBE_ARGS],
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
    )
    time.sleep(3)
    if server.poll() is not None:
        print("SERVER DIED:", server.stdout.read()[-500:])
        raise SystemExit(1)
    try:
        cmds = [
            "-batch",
            "-ex", "set architecture armv8-m.main",
            "-ex", "file " + elf,
            "-ex", "target extended-remote localhost:" + GDB_PORT,
            "-ex", "load",
            "-ex", "set $sp = *(unsigned int*)" + VECTOR_BASE,
            "-ex", "set $pc = *(unsigned int*)(" + VECTOR_BASE + " + 4) & ~1",
            "-ex", "print/x $sp",
            "-ex", "print/x $pc",
            "-ex", "disconnect",
        ]
        g = subprocess.run(
            [GDB] + cmds, capture_output=True, text=True, timeout=120
        )
        print(g.stdout[-700:])
        if "Transfer rate" not in g.stdout:
            print("LOAD DID NOT COMPLETE")
            print(g.stderr[-400:])
            raise SystemExit(1)
    finally:
        subprocess.run(
            ["pkill", "-9", "-f", "extended-remote localhost:" + GDB_PORT],
            capture_output=True,
        )
        server.kill()
        time.sleep(1)


def main():
    if len(sys.argv) < 2 or sys.argv[1].startswith("-"):
        print(__doc__)
        raise SystemExit(2)
    elf = sys.argv[1]
    # Preflight gdb before the reset: the reset is destructive, and
    # dying on a missing gdb afterwards would leave the board reset
    # with nothing loaded (stock macOS ships no gdb).
    if shutil.which(GDB) is None:
        print("gdb binary not found:", GDB)
        print("set GDB=arm-none-eabi-gdb (or another gdb) and retry")
        raise SystemExit(1)
    system_reset()
    gdb_load(elf)
    if "--no-run" not in sys.argv:
        r = probe_rs("write", "b32", DHCSR, DBGKEY_RUN)
        if r.returncode != 0:
            print("RELEASE FAILED:", r.stderr[-300:])
            raise SystemExit(1)
        print("released; core running")
        print("attach RTT:  probe-rs attach --chip", CHIP, "--speed 100", elf)


if __name__ == "__main__":
    main()
