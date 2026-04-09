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

# shellcheck source=.mise/tasks/_lib.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/tasks/_lib.sh"

if [ -z "${BROKER_HOST_IP:-}" ]; then
	_ip="$(detect_broker_ip)" || true
	if [ -n "${_ip:-}" ]; then
		export BROKER_HOST_IP="$_ip"
	fi
	unset _ip
fi
