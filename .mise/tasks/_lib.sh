#!/usr/bin/env bash
# Shared library for mise tasks.
# Source this file; do not execute it directly.
#
# Provides:
#   CONTAINER_NAME            Broker container name
#   RUNTIME                   Container runtime (set by detect_runtime)
#   detect_runtime            Detect podman or docker
#   detect_broker_interface   Auto-detect network interface
#   detect_broker_ip          Detect broker IP from interface
#   detect_host_target        Detect the Rust host triple

# shellcheck disable=SC2034
CONTAINER_NAME='mosquitto-tls'

# Detect the container runtime.  Prefers podman, falls back
# to docker.  Respects CONTAINER_RUNTIME if set.  Verifies
# the selected runtime is reachable (daemon/service running).
#
# Sets: RUNTIME (caller-visible variable)
# Returns: 0 if found, 1 if not
detect_runtime() {
	if [ -n "${CONTAINER_RUNTIME:-}" ]; then
		case "$CONTAINER_RUNTIME" in
			podman|docker) ;;
			*)
				printf 'ERROR: CONTAINER_RUNTIME=%s is not supported.\n' \
					"$CONTAINER_RUNTIME" >&2
				echo 'Set CONTAINER_RUNTIME to "podman" or "docker".' >&2
				return 1
				;;
		esac
		if ! command -v "$CONTAINER_RUNTIME" >/dev/null 2>&1; then
			printf 'ERROR: CONTAINER_RUNTIME=%s but %s is not installed.\n' \
				"$CONTAINER_RUNTIME" "$CONTAINER_RUNTIME" >&2
			return 1
		fi
		RUNTIME="$CONTAINER_RUNTIME"
		return 0
	fi

	local _rt
	for _rt in podman docker; do
		if command -v "$_rt" >/dev/null 2>&1 \
				&& "$_rt" info >/dev/null 2>&1; then
			RUNTIME="$_rt"
			return 0
		fi
	done

	return 1
}

# Detect the broker's network interface.
#
# Priority:
#   1. BROKER_INTERFACE env var (if set)
#   2. Default route interface (ip -4 route)
#   3. Fallback: eno1
#
# Prints: interface name
detect_broker_interface() {
	if [ -n "${BROKER_INTERFACE:-}" ]; then
		echo "$BROKER_INTERFACE"
		return 0
	fi

	local _iface
	_iface="$(ip -4 route show default 2>/dev/null \
		| awk '/default/ {print $5; exit}')"

	if [ -n "$_iface" ]; then
		echo "$_iface"
		return 0
	fi

	echo 'eno1'
}

# Detect the broker bind IP address.
#
# Priority:
#   1. BROKER_HOST_IP env var (if set)
#   2. macOS: 127.0.0.1 (loopback)
#   3. Linux: IPv4 of the detected interface
#
# Prints: IP address
# Returns: 0 on success, 1 if detection fails
detect_broker_ip() {
	if [ -n "${BROKER_HOST_IP:-}" ]; then
		echo "$BROKER_HOST_IP"
		return 0
	fi

	if [ "$(uname -s)" = 'Darwin' ]; then
		echo '127.0.0.1'
		return 0
	fi

	local _iface _ip
	_iface="$(detect_broker_interface)"
	_ip="$(ip -4 addr show "$_iface" 2>/dev/null \
		| awk '/inet / { split($2, a, "/"); print a[1]; exit }')" || true

	if [ -n "$_ip" ]; then
		echo "$_ip"
		return 0
	fi

	return 1
}

# Detect the Rust host target triple.
#
# Prints: host triple (e.g., x86_64-unknown-linux-gnu)
# Returns: 0 on success, 1 if rustc is not available
detect_host_target() {
	rustc -vV | sed -n 's/^host: //p'
}
