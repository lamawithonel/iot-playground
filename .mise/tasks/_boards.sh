#!/usr/bin/env bash
# Shared board registry for mise tasks.
# Source this file; do not execute it directly.  Callers that reach
# the net-build path must source _lib.sh first (detect_broker_ip).
#
# Holds only what cargo cannot infer.  Every other board fact keeps
# its existing home: target and runner in the board's
# .cargo/config.toml, chip and probe pin in its Embed.toml, and
# projects in its Cargo.toml [features].  The two constants below
# mirror the root Cargo.toml workspace lists; update both on board
# promotion (see boards/AGENTS.md).
#
# Provides:
#   MEMBER_BOARDS           Workspace-member boards ('all' expands to these)
#   DEFAULT_BOARD           Board used when no board argument is given
#   env_boards              Board names pinned by the IOT_BOARDS env var
#   env_project             Project pinned for a board by IOT_BOARDS
#   board_dirs              List every boards/*/ crate directory
#   board_projects          Projects (cargo feature sets) of a board
#   board_default_project   Default project of a multi-project board
#   board_loader            Loader script for boards probe-rs cannot flash
#   board_chip              Chip name from the board's Embed.toml
#   board_target            Target triple (board override or root default)
#   resolve_boards          Expand and validate board args, one per line
#   board_features          Validate a project, emit its --features flag
#   build_each              Run one cargo verb across resolved boards

# shellcheck disable=SC2034  # consumed by sourcing tasks
MEMBER_BOARDS='feather-stm32f405 nucleo-h753zi'
DEFAULT_BOARD='feather-stm32f405'

# The IOT_BOARDS env var pins sticky defaults for the board tasks, e.g.
#
#   export IOT_BOARDS='nucleo-n657x0:net,feather-stm32f405'
#
# Comma-separated board[:project] entries.  With no board argument,
# the tasks target every listed board, and a listed project becomes
# that board's default project while the variable is set (the
# --project flag still overrides).  Duplicate entries: the first
# wins.  Entries use a colon, unlike the CLI grammar, because an
# env var has no tab-completion to lose.
#
# Both parsers run under noglob: the unquoted word-split would
# otherwise glob entries against the cwd (IOT_BOARDS='*' must error as
# an unknown board, not expand to the repo listing).  Restored
# unconditionally afterward-- these tasks never set noglob.

# Board names from $IOT_BOARDS, one per line, in order.
env_boards() {
	local _entry
	set -o noglob
	for _entry in $(echo "${IOT_BOARDS:-}" | tr ',' ' '); do
		echo "${_entry%%:*}"
	done
	set +o noglob
}

# Project pinned for a board by $IOT_BOARDS; empty if none.  First
# entry wins on duplicates.
env_project() {
	local _entry _proj=''
	set -o noglob
	for _entry in $(echo "${IOT_BOARDS:-}" | tr ',' ' '); do
		if [ "${_entry%%:*}" = "$1" ] && [ "$_entry" != "${_entry%%:*}" ] \
				&& [ -z "$_proj" ]; then
			_proj="${_entry#*:}"
		fi
	done
	set +o noglob
	echo "$_proj"
}

# List every board crate directory (workspace member or not).
# Needs globbing, so it re-enables it locally: resolve_boards calls
# this from inside a noglob window when expanding an IOT_BOARDS pin.
# Caller state is restored either way.
board_dirs() {
	local _d _had_noglob=0
	case "$-" in
		*f*) _had_noglob=1 ;;
	esac
	set +o noglob
	for _d in boards/*/Cargo.toml; do
		basename "$(dirname "$_d")"
	done
	if [ "$_had_noglob" -eq 1 ]; then
		set -o noglob
	fi
}

# Projects a board supports, as a space-separated list.  Empty for
# single-app boards.  This is a UX convenience (friendly errors and
# a default), not the enforcement: cargo itself rejects an
# undefined feature.
board_projects() {
	case "$1" in
		nucleo-n657x0) echo 'g1-spike net' ;;
		*) echo '' ;;
	esac
}

# Default project of a multi-project board; empty otherwise.
board_default_project() {
	case "$1" in
		nucleo-n657x0) echo 'g1-spike' ;;
		*) echo '' ;;
	esac
}

# Loader script for a board that `probe-rs run` cannot flash; empty
# for boards that flash via `cargo embed`.  N6_LOAD_CMD overrides
# the tracked default (the Embed.local.toml pattern); it must be a
# single executable path, not a command line with arguments.
board_loader() {
	case "$1" in
		nucleo-n657x0) echo "${N6_LOAD_CMD:-boards/nucleo-n657x0/flash.py}" ;;
		*) echo '' ;;
	esac
}

# Chip name from the board's Embed.toml; empty if the board has
# none (loader boards).
board_chip() {
	sed -n 's/^[[:space:]]*chip[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' \
		"boards/$1/Embed.toml" 2>/dev/null | head -1
}

# Target triple: the board's own .cargo/config.toml override if it
# has one, else the workspace default from the root config.
board_target() {
	local _t=''
	if [ -f "boards/$1/.cargo/config.toml" ]; then
		_t="$(sed -n 's/^[[:space:]]*target[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' \
			"boards/$1/.cargo/config.toml" | head -1)"
	fi
	echo "${_t:-thumbv7em-none-eabihf}"
}

# Expand board arguments to one validated name per line.
#   no args  -> $IOT_BOARDS entries if pinned, else DEFAULT_BOARD
#   'all'    -> MEMBER_BOARDS (excluded boards stay opt-in by name)
# Duplicates are dropped, order preserved.
resolve_boards() {
	local _out='' _tok _b _known _pinned
	if [ $# -eq 0 ]; then
		# 'all' expands per-board, so a project suffix on it would
		# be silently dropped; reject it instead.
		case ",${IOT_BOARDS:-}," in
			*,all:*)
				echo "ERROR: 'all' takes no project in IOT_BOARDS (got IOT_BOARDS='${IOT_BOARDS:-}')." >&2
				return 1
				;;
		esac
		_pinned="$(env_boards)"
		if [ -n "$_pinned" ]; then
			# Re-enter with the pinned names so they get the same
			# validation as CLI arguments.  This unquoted expansion
			# needs its own noglob window: without it a pinned
			# 'nucleo-*' could match a stray root-level file named
			# like a board and validate silently.
			set -o noglob
			# shellcheck disable=SC2086  # one name per word
			if ! resolve_boards $_pinned; then
				set +o noglob
				echo "note: board list came from IOT_BOARDS='${IOT_BOARDS:-}'." >&2
				return 1
			fi
			set +o noglob
			return 0
		fi
		echo "$DEFAULT_BOARD"
		return 0
	fi
	for _tok in "$@"; do
		if [ "$_tok" = 'all' ]; then
			for _b in $MEMBER_BOARDS; do
				case " $_out " in
					*" $_b "*) ;;
					*) _out="${_out} ${_b}" ;;
				esac
			done
			continue
		fi
		_known=0
		for _b in $(board_dirs); do
			[ "$_b" = "$_tok" ] && _known=1
		done
		if [ "$_known" -eq 0 ]; then
			echo "ERROR: unknown board '${_tok}'." >&2
			echo "Valid: $(board_dirs | tr '\n' ' ')or 'all' (workspace members)." >&2
			return 1
		fi
		case " $_out " in
			*" $_tok "*) ;;
			*) _out="${_out} ${_tok}" ;;
		esac
	done
	for _b in $_out; do
		echo "$_b"
	done
}

# Emit the --features flag for a board and requested project.
# Empty output for single-app boards; a friendly error for an
# unsupported combination (cargo remains the real enforcer).
board_features() {
	local _board="$1" _proj="${2:-}" _supported _p _ok=0
	_supported="$(board_projects "$_board")"
	if [ -z "$_supported" ]; then
		if [ -n "$_proj" ]; then
			# Source-neutral wording: the project may come from the
			# --project flag or from an IOT_BOARDS entry.
			echo "ERROR: ${_board} is single-app and takes no project (got '${_proj}')." >&2
			return 1
		fi
		return 0
	fi
	[ -z "$_proj" ] && _proj="$(board_default_project "$_board")"
	for _p in $_supported; do
		[ "$_p" = "$_proj" ] && _ok=1
	done
	if [ "$_ok" -eq 0 ]; then
		echo "ERROR: ${_board} has no project '${_proj}'.  Supported: ${_supported}." >&2
		return 1
	fi
	echo "--features $_proj"
}

# Run one cargo verb (build or clippy) across resolved boards.
# Reads the caller task's usage_project/usage_release variables.
# `cd` into the board directory makes members and the excluded N6
# identical: each picks up its own .cargo/config.toml (target,
# runner) and lockfile, so there is no -p vs --manifest-path fork.
# `set -x` echoes the exact command a user could run by hand.
build_each() {
	local _verb="$1" _boards _b _proj _feats
	shift
	_boards="$(resolve_boards "$@")" || return 1
	if [ -n "${usage_project:-}" ] \
			&& [ "$(printf '%s\n' "$_boards" | wc -l)" -gt 1 ]; then
		echo 'ERROR: --project applies to exactly one board.' >&2
		if [ $# -eq 0 ] && [ -n "${IOT_BOARDS:-}" ]; then
			echo "note: board list came from IOT_BOARDS='${IOT_BOARDS}'." >&2
		fi
		return 1
	fi
	for _b in $_boards; do
		_proj="${usage_project:-$(env_project "$_b")}"
		if ! _feats="$(board_features "$_b" "$_proj")"; then
			# Name the source when the failing project came from the
			# env pin: the user may have exported IOT_BOARDS days ago.
			if [ -z "${usage_project:-}" ] && [ -n "$_proj" ]; then
				echo "note: project '${_proj}' came from IOT_BOARDS='${IOT_BOARDS:-}'." >&2
			fi
			return 1
		fi
		case "$_feats" in
			*net*)
				# The N6 build.rs refuses a net build without a broker
				# endpoint; default it from the bench detection in
				# _lib.sh so a net build works out of the box.
				BROKER_HOST="${BROKER_HOST:-$(detect_broker_ip || echo '127.0.0.1')}"
				BROKER_PORT="${BROKER_PORT:-8883}"
				export BROKER_HOST BROKER_PORT
				echo "note: net build uses BROKER_HOST=${BROKER_HOST} BROKER_PORT=${BROKER_PORT}" >&2
				;;
		esac
		# shellcheck disable=SC2086  # _feats intentionally splits
		case "$_verb" in
			build)
				( set -x; cd "boards/${_b}" \
					&& cargo build ${usage_release:+--release} $_feats )
				;;
			clippy)
				( set -x; cd "boards/${_b}" \
					&& cargo clippy ${usage_release:+--release} $_feats \
						-- -D warnings )
				;;
			*)
				echo "ERROR: build_each: unknown verb '${_verb}'." >&2
				return 1
				;;
		esac
	done
}
