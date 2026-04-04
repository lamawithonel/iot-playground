#!/usr/bin/env bash
# Stop and remove the Mosquitto MQTT test broker container.

set -o errexit
set -o nounset
set -o pipefail

_container_name='mosquitto-tls'

# Find which runtime owns the container.  If CONTAINER_RUNTIME
# is set, use it directly.  Otherwise, search both runtimes for
# the container to handle the case where it was started with a
# different default.
_detect_runtime() {
	if [ -n "${CONTAINER_RUNTIME:-}" ]; then
		_runtime="$CONTAINER_RUNTIME"
		case "$_runtime" in
			podman|docker) ;;
			*)
				printf 'ERROR: CONTAINER_RUNTIME=%s is not supported.\n' \
					"$_runtime"
				echo 'Set CONTAINER_RUNTIME to "podman" or "docker".'
				exit 1
				;;
		esac
		if ! command -v "$_runtime" >/dev/null 2>&1; then
			printf 'ERROR: CONTAINER_RUNTIME=%s but %s is not installed.\n' \
				"$_runtime" "$_runtime"
			exit 1
		fi
		return 0
	fi

	local _candidate
	for _candidate in podman docker; do
		if command -v "$_candidate" >/dev/null 2>&1 \
				&& "$_candidate" ps -a --format '{{.Names}}' 2>/dev/null \
				| grep -q "^${_container_name}$"; then
			_runtime="$_candidate"
			return 0
		fi
	done

	# Container not found in either — pick the first available
	# runtime so the "not running" message still works.
	if command -v podman >/dev/null 2>&1; then
		_runtime='podman'
	elif command -v docker >/dev/null 2>&1; then
		_runtime='docker'
	else
		echo 'ERROR: No container runtime found.'
		exit 1
	fi
}

_main() {
	_detect_runtime

	if "$_runtime" ps -a --format '{{.Names}}' \
			| grep -q "^${_container_name}$"; then
		echo 'Stopping Mosquitto MQTT broker...'
		"$_runtime" stop "$_container_name" 2>/dev/null || true
		"$_runtime" rm "$_container_name" 2>/dev/null || true
		echo 'Broker stopped and removed.'
	else
		echo 'Broker is not running.'
	fi
}

_main
