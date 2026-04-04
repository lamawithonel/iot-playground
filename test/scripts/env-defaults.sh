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

if [ -z "${BROKER_HOST_IP:-}" ]; then
	if [ "$(uname -s)" = 'Darwin' ]; then
		export BROKER_HOST_IP='127.0.0.1'
	else
		# Detect from the configured interface (default: eno1).
		# If detection fails, leave unset so scripts can handle
		# it or use their own fallback.
		_iface="${BROKER_INTERFACE:-eno1}"
		_ip=$(ip -4 addr show "$_iface" 2>/dev/null \
			| awk '/inet / { split($2, a, "/"); print a[1]; exit }') || true
		if [ -n "${_ip:-}" ]; then
			export BROKER_HOST_IP="$_ip"
		fi
		unset _iface _ip
	fi
fi
