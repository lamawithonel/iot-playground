#!/usr/bin/env bash
# NUCLEO-N657X0-Q smoke suite: RAM-boot the net firmware, then
# assert the DHCP -> SNTP -> MQTT publish/PUBACK chain over RTT.
# Invoked by the `test:smoke` router (.mise/tasks/test/smoke).
#
# Requires the board's dev-boot jumper setting and loader flow (see
# docs/src/boards/nucleo-n657x0.md); on a multi-probe bench, pin
# the probe with PROBE_RS_PROBE.  The broker endpoint bakes in at
# build time from BROKER_HOST/BROKER_PORT, defaulted here from the
# same bench detection the broker tasks use, so the firmware
# publishes to the local test broker.
set -o errexit
set -o nounset
set -o pipefail

_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_repo_root="$(cd "${_script_dir}/../.." && pwd)"
cd "$_repo_root"

# shellcheck source=.mise/tasks/_lib.sh
source "${_repo_root}/.mise/tasks/_lib.sh"

# Check for connected debug probe
_probe_output="$(probe-rs list 2>&1)"
if echo "$_probe_output" | grep -q 'No probes were found'; then
	echo '⏭  No debug probe connected — skipping N6 smoke suite.' >&2
	exit 0
fi

# DHCP + SNTP + a handful of 3 s publish intervals fit well inside
# 60 s; tier wrappers may extend the window via SMOKE_TEST_DURATION.
_duration="${SMOKE_TEST_DURATION:-60}"
_chip='STM32N657'
_board='nucleo-n657x0'
_elf="boards/${_board}/target/thumbv8m.main-none-eabihf/debug/${_board}"

# Only verify the local test-broker container when BROKER_HOST was
# defaulted here; an explicit BROKER_HOST may name a remote broker
# this host cannot inspect.
_check_broker=false
if [ -z "${BROKER_HOST:-}" ]; then
	_check_broker=true
fi
BROKER_HOST="${BROKER_HOST:-$(detect_broker_ip || echo '127.0.0.1')}"
BROKER_PORT="${BROKER_PORT:-8883}"
export BROKER_HOST BROKER_PORT
echo "note: net build uses BROKER_HOST=${BROKER_HOST} BROKER_PORT=${BROKER_PORT}" >&2

# Fail fast without a broker: otherwise the suite burns a build and
# a full capture window, then fails on missing publish markers--
# indistinguishable from a firmware defect.
if [ "$_check_broker" = true ] && ! mise run broker:status >/dev/null 2>&1; then
	echo "❌ FAIL: no MQTT broker at ${BROKER_HOST}:${BROKER_PORT}." >&2
	echo '   Start it with: mise run broker:start' >&2
	exit 1
fi

echo '🔨 Building net firmware...'
( set -x; cd "boards/${_board}" && cargo build --features net )

echo '🚀 RAM-boot loading...'
"boards/${_board}/flash.py" "$_elf"

_rtt_file="$(mktemp "${TMPDIR:-/tmp}/n6-smoke-rtt.XXXXXX")"
_cleanup() {
	rm -f "$_rtt_file"
}
trap _cleanup EXIT

# --speed 100: onboard STLINK-V3EC clocking workaround (board page).
echo "📡 Capturing RTT for ${_duration}s..."
timeout "$_duration" probe-rs attach \
	--chip "$_chip" --speed 100 "$_elf" \
	> "$_rtt_file" 2>&1 || true

echo '── RTT Output ──────────────────────────────────'
cat "$_rtt_file"
echo '────────────────────────────────────────────────'

_pass=true

if [ ! -s "$_rtt_file" ]; then
	echo '❌ FAIL: No RTT output — device may not have booted.' >&2
	_pass=false
fi

if grep -qi 'panic' "$_rtt_file"; then
	echo '❌ FAIL: Panic detected in RTT output.' >&2
	_pass=false
fi

if grep -qiE 'hardfault|hard fault' "$_rtt_file"; then
	echo '❌ FAIL: HardFault detected.' >&2
	_pass=false
fi

# The publish/PUBACK pair asserts end-to-end network health: the
# firmware gates publishing on SNTP time sync, so a publish implies
# link + DHCP + UDP + DNS + SNTP, and a PUBACK proves the TLS/MQTT
# broker round trip (QoS 1).  One-shot boot markers (the SNTP line)
# are NOT asserted: the core runs before `probe-rs attach` connects,
# so early lines can rotate out of the RTT ring buffer under
# verbose DEFMT_LOG levels.
for _marker in 'Publishing #' 'acknowledged (PUBACK'; do
	if ! grep -qF "$_marker" "$_rtt_file"; then
		echo "❌ FAIL: RTT output missing marker: ${_marker}" >&2
		_pass=false
	fi
done

if [ "$_pass" = false ]; then
	echo '🔥 N6 smoke suite FAILED.' >&2
	exit 1
fi

echo '✅ N6 smoke suite PASSED.'
