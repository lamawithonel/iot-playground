#!/usr/bin/env bash
# Platform-aware environment variable defaults for mise.
# Sourced by mise via _.source in .mise.toml.
#
# Supported variables:
#   CONTAINER_RUNTIME  Container runtime (podman or docker).
#                      Auto-detected by scripts if unset.
#   BROKER_HOST_IP     Default broker bind IP address.
#                      macOS: 127.0.0.1 (loopback)
#                      Linux: detected from BROKER_INTERFACE
#   IOT_BOARDS         Sticky board[:project] pin (see
#                      .mise/tasks/_boards.sh); its first entry
#                      also feeds the probe-rs bridge below.

_env_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=.mise/tasks/_lib.sh
source "${_env_dir}/tasks/_lib.sh"

if [ -z "${BROKER_HOST_IP:-}" ]; then
	_ip="$(detect_broker_ip)" || true
	if [ -n "${_ip:-}" ]; then
		export BROKER_HOST_IP="$_ip"
	fi
	unset _ip
fi

# ── IOT_BOARDS pin -> probe-rs bridge ────────────────────
# probe-rs reads PROBE_RS_CHIP and PROBE_RS_SPEED natively, so
# deriving them from the pin's first board makes bare probe-rs
# commands (attach, gdb, reset, ...) target it with no flags.
# Explicit env and flags still win.  PROBE_RS_PROBE is bench state
# (a probe serial) and is never derived.  The chip comes from the
# board's Embed.toml, or from the runner line of its own
# .cargo/config.toml for loader boards without one (which also
# carries a --speed workaround worth exporting).
#
# `mise run` re-evaluates this file every invocation, so tasks
# always see a fresh pin; an interactive mise-activated shell may
# need a prompt refresh (cd, or `eval "$(mise hook-env)"`) after
# changing IOT_BOARDS.
if [ -n "${IOT_BOARDS:-}" ]; then
	_first="${IOT_BOARDS%%,*}"
	_first="${_first%%:*}"
	_bdir="${_env_dir}/../boards/${_first}"
	_chip=''
	_speed=''
	if [ -f "${_bdir}/Embed.toml" ]; then
		_chip="$(sed -n 's/^[[:space:]]*chip[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' \
			"${_bdir}/Embed.toml" | head -1)"
	elif [ -f "${_bdir}/.cargo/config.toml" ]; then
		_runner="$(sed -n 's/^[[:space:]]*runner[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' \
			"${_bdir}/.cargo/config.toml" | head -1)"
		case "$_runner" in
			*--chip\ *)
				_chip="${_runner##*--chip }"
				_chip="${_chip%% *}"
				;;
		esac
		case "$_runner" in
			*--speed\ *)
				_speed="${_runner##*--speed }"
				_speed="${_speed%% *}"
				;;
		esac
		unset _runner
	fi
	if [ -n "$_chip" ] && [ -z "${PROBE_RS_CHIP:-}" ]; then
		export PROBE_RS_CHIP="$_chip"
	fi
	if [ -n "$_speed" ] && [ -z "${PROBE_RS_SPEED:-}" ]; then
		export PROBE_RS_SPEED="$_speed"
	fi
	unset _first _bdir _chip _speed
fi

unset _env_dir
