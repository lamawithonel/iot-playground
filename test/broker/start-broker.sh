#!/usr/bin/env bash
# Launch Mosquitto MQTT broker with TLS support.
#
# Supports Podman and Docker.  On Linux, binds to a host
# interface for device testing.  On macOS, binds to 127.0.0.1.
#
# Environment variables:
#   CONTAINER_RUNTIME  Container runtime: podman or docker
#                      (auto-detected if unset)
#   BROKER_HOST_IP     Override the bind IP (e.g., 10.0.0.1)
#   BROKER_INTERFACE   Override the Linux interface (default: eno1)

set -o errexit
set -o nounset
set -o pipefail

_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_repo_root="$(cd "$_script_dir/../.." && pwd)"
_container_name='mosquitto-tls'

_ca_cert="${_repo_root}/.local/certs/ca/root.crt"
_server_cert="${_repo_root}/.local/certs/server/broker.crt"
_server_key="${_repo_root}/.local/private/broker.key"

# Detect container runtime.  Prefer podman, fall back to docker.
# CONTAINER_RUNTIME env var overrides auto-detection.
# Probes reachability (not just binary presence) so a dormant
# Podman install doesn't block a working Docker setup.
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
				&& "$_candidate" info >/dev/null 2>&1; then
			_runtime="$_candidate"
			return 0
		fi
	done

	# No reachable runtime — give an actionable error.
	if command -v podman >/dev/null 2>&1; then
		echo 'ERROR: podman is installed but not reachable.'
		echo 'Start a VM: podman machine start'
	elif command -v docker >/dev/null 2>&1; then
		echo 'ERROR: docker is installed but not reachable.'
		echo 'Start Docker Desktop or Colima.'
	else
		echo 'ERROR: No container runtime found.'
		echo 'Install one of:'
		echo '  Podman: https://podman.io/getting-started/installation'
		echo '  Docker: https://docs.docker.com/get-docker/'
	fi
	exit 1
}

# Set _image_name based on runtime.  Podman prefixes local
# images with localhost/; Docker does not.
_set_image_name() {
	case "$_runtime" in
		podman) _image_name='localhost/mosquitto-tls:latest' ;;
		*)      _image_name='mosquitto-tls:latest' ;;
	esac
}

# Detect SELinux.  Only apply :Z bind-mount suffix when
# SELinux is active (Fedora, RHEL).  Debian, Ubuntu, Arch,
# and macOS do not need it.
_detect_selinux() {
	_selinux_suffix=''
	if command -v getenforce >/dev/null 2>&1; then
		case "$(getenforce 2>/dev/null)" in
			Enforcing|Permissive) _selinux_suffix=',Z' ;;
		esac
	fi
}

# Platform detection: set _host_ip and _interface.
_detect_platform() {
	_platform="$(uname -s)"
	case "$_platform" in
		Darwin)
			_host_ip="${BROKER_HOST_IP:-127.0.0.1}"
			_interface=''
			;;
		Linux)
			_interface="${BROKER_INTERFACE:-eno1}"
			_host_ip="${BROKER_HOST_IP:-}"
			;;
		*)
			printf 'ERROR: Unsupported platform: %s\n' "$_platform"
			exit 1
			;;
	esac
}

_main() {
	_detect_runtime
	_set_image_name
	_detect_selinux
	_detect_platform

	echo '=== Mosquitto MQTT Broker with TLS ==='
	printf 'Runtime: %s\n' "$_runtime"
	printf 'Script directory: %s\n' "$_script_dir"
	if [ "$_platform" = 'Darwin' ]; then
		printf 'Binding to: %s (macOS)\n\n' "$_host_ip"
	else
		printf 'Binding to interface: %s\n\n' "$_interface"
	fi

	_check_runtime_connection
	_check_certs
	if [ -z "$_host_ip" ]; then
		_check_interface
	fi
	_detect_ip
	_verify_cert_san
	_cleanup_existing
	_build_image
	_create_volumes
	_start_container
	_verify_running
	_show_status
}

# Verify the runtime daemon/service is reachable.  Only needed
# when CONTAINER_RUNTIME was explicitly set (auto-detection
# already probes reachability).
_check_runtime_connection() {
	if [ -z "${CONTAINER_RUNTIME:-}" ]; then
		return 0
	fi

	if ! "$_runtime" info >/dev/null 2>&1; then
		printf 'ERROR: %s cannot connect to a container runtime.\n' \
			"$_runtime"
		if [ "$_platform" = 'Darwin' ]; then
			case "$_runtime" in
				podman)
					echo 'On macOS, start a VM with one of:'
					echo '  podman machine start'
					echo '  limactl start podman'
					;;
				docker)
					echo 'On macOS, start a VM with one of:'
					echo '  open -a Docker  (Docker Desktop)'
					echo '  colima start'
					;;
			esac
		fi
		exit 1
	fi
}

_check_certs() {
	local _missing=0
	for _f in "$_ca_cert" "$_server_cert" "$_server_key"; do
		if [ ! -f "$_f" ]; then
			printf 'ERROR: Required file not found: %s\n' "$_f"
			_missing=1
		fi
	done
	if [ "$_missing" -eq 1 ]; then
		echo 'Run "mise run tls:server:broker" to generate certs.'
		exit 1
	fi
}

_check_interface() {
	if [ "$_platform" = 'Darwin' ]; then
		return 0
	fi

	if ! ip addr show "$_interface" >/dev/null 2>&1; then
		printf 'ERROR: Interface "%s" not found!\n' "$_interface"
		echo 'Available interfaces:'
		ip -br addr
		echo ''
		echo 'Set BROKER_INTERFACE to override (e.g., eth0).'
		exit 1
	fi
}

_detect_ip() {
	if [ "$_platform" = 'Darwin' ]; then
		return 0
	fi

	if [ -n "$_host_ip" ]; then
		printf 'Using configured IP: %s\n' "$_host_ip"
		return 0
	fi

	local _actual_ip
	_actual_ip=$(
		ip -4 addr show "$_interface" \
			| awk '/inet / { split($2, a, "/"); print a[1]; exit }'
	) || true

	if [ -z "$_actual_ip" ]; then
		printf 'ERROR: No IPv4 address found on interface %s\n' \
			"$_interface"
		exit 1
	fi

	printf 'Interface %s has IP: %s\n' "$_interface" "$_actual_ip"
	_host_ip="$_actual_ip"
}

# Verify the broker cert SAN matches the bind IP.  A mismatch
# causes TLS hostname verification failures on the device.
_verify_cert_san() {
	local _cert_san
	_cert_san=$(
		openssl x509 -in "$_server_cert" -noout -text 2>/dev/null \
			| awk '/Subject Alternative Name/{getline; print}' \
			| awk -F'IP Address:' '{
				for (i = 2; i <= NF; i++) {
					gsub(/^[ \t]+/, "", $i)
					gsub(/[, \t\n]+.*/, "", $i)
					if ($i != "") print $i
				}
			}'
	) || true

	if [ -z "$_cert_san" ]; then
		echo 'WARNING: Could not extract SAN IP from broker cert.'
		echo 'TLS hostname verification may fail.'
		return 0
	fi

	if ! echo "$_cert_san" | grep -qxF "$_host_ip"; then
		echo ''
		printf 'ERROR: Broker cert SAN (%s) does not match' \
			"$_cert_san"
		printf ' bind IP (%s).\n' "$_host_ip"
		echo 'TLS clients will reject the connection.'
		echo ''
		echo 'To fix, regenerate the broker cert:'
		echo '  mise run tls:server:broker -- --force'
		echo ''
		echo 'If that does not resolve it, delete all certs and retry:'
		echo '  rm -rf .local/certs .local/private .cache/signing_requests'
		echo '  mise run tls:server:broker'
		exit 1
	fi
}

_cleanup_existing() {
	echo ''
	echo 'Cleaning up existing container...'
	"$_runtime" stop "$_container_name" 2>/dev/null || true
	"$_runtime" rm "$_container_name" 2>/dev/null || true
}

_build_image() {
	echo ''
	echo 'Building container image...'
	"$_runtime" build -t "$_image_name" "$_script_dir"
}

_create_volumes() {
	echo ''
	echo 'Creating volumes...'
	"$_runtime" volume create mosquitto-data 2>/dev/null || true
	"$_runtime" volume create mosquitto-log 2>/dev/null || true
}

_start_container() {
	echo ''
	echo 'Starting Mosquitto MQTT broker...'
	"$_runtime" run -d \
		--name "$_container_name" \
		--restart unless-stopped \
		-p "${_host_ip}:1883:1883" \
		-p "${_host_ip}:8883:8883" \
		-v mosquitto-data:/mosquitto/data \
		-v mosquitto-log:/mosquitto/log \
		-v "${_ca_cert}:/mosquitto/certs/root.crt:ro${_selinux_suffix}" \
		-v "${_server_cert}:/mosquitto/certs/broker.crt:ro${_selinux_suffix}" \
		-v "${_server_key}:/mosquitto/certs/broker.key:ro${_selinux_suffix}" \
		"$_image_name"
}

_verify_running() {
	sleep 2
	if ! "$_runtime" ps --format '{{.Names}}' \
			| grep -qx "$_container_name"; then
		echo ''
		echo 'ERROR: Container failed to start!'
		echo 'Container logs:'
		"$_runtime" logs "$_container_name"
		exit 1
	fi
}

_show_status() {
	echo ''
	echo '=== Mosquitto MQTT Broker Started Successfully ==='
	echo ''
	printf 'MQTT (non-TLS):  %s:1883\n' "$_host_ip"
	printf 'MQTTS (TLS):     %s:8883\n' "$_host_ip"
	echo ''
	printf 'To view logs:    mise run broker:logs\n'
	printf 'To stop:         mise run broker:stop\n'
}

_main
