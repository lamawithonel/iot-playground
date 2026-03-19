#!/usr/bin/env bash
# Launch Mosquitto MQTT broker with TLS support via Podman.
# Binds to a host interface for device testing.

set -o errexit
set -o nounset
set -o pipefail

_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_interface='eno1'
_host_ip='192.168.1.1'
_container_name='mosquitto-tls'
_image_name='mosquitto-tls:latest'

_main() {
	echo '=== Mosquitto MQTT Broker with TLS ==='
	printf 'Script directory: %s\n' "$_script_dir"
	printf 'Binding to interface: %s (%s)\n\n' "$_interface" "$_host_ip"

	_check_interface
	_detect_ip
	_cleanup_existing
	_build_image
	_create_volumes
	_start_container
	_verify_running
	_show_status
}

_check_interface() {
	if ! ip addr show "$_interface" >/dev/null 2>&1; then
		echo 'ERROR: Interface not found!'
		echo 'Available interfaces:'
		ip -br addr
		exit 1
	fi
}

_detect_ip() {
	local _actual_ip
	_actual_ip=$(
		ip -4 addr show "$_interface" \
			| grep -oP '(?<=inet\s)\d+(\.\d+){3}'
	) || true

	if [ -z "$_actual_ip" ]; then
		printf 'ERROR: No IPv4 address found on interface %s\n' \
			"$_interface"
		exit 1
	fi

	printf 'Interface %s has IP: %s\n' "$_interface" "$_actual_ip"
	if [ "$_actual_ip" != "$_host_ip" ]; then
		printf 'ERROR: Expected interface %s to have IP %s but found %s\n' \
			"$_interface" "$_host_ip" "$_actual_ip"
		echo 'The container TLS certificate is generated with CN=192.168.1.1.'
		echo 'A mismatched IP will cause TLS hostname verification failures.'
		echo 'Adjust your network configuration or rebuild the image with'
		echo 'the correct CN, then rerun this script.'
		exit 1
	fi
}

_cleanup_existing() {
	echo ''
	echo 'Cleaning up existing container...'
	podman stop "$_container_name" 2>/dev/null || true
	podman rm "$_container_name" 2>/dev/null || true
}

_build_image() {
	echo ''
	echo 'Building container image...'
	podman build -t "$_image_name" "$_script_dir"
}

_create_volumes() {
	echo ''
	echo 'Creating volumes...'
	podman volume create mosquitto-data 2>/dev/null || true
	podman volume create mosquitto-log 2>/dev/null || true
}

_start_container() {
	echo ''
	echo 'Starting Mosquitto MQTT broker...'
	podman run -d \
		--name "$_container_name" \
		--restart unless-stopped \
		-p "${_host_ip}:1883:1883" \
		-p "${_host_ip}:8883:8883" \
		-v mosquitto-data:/mosquitto/data \
		-v mosquitto-log:/mosquitto/log \
		"$_image_name"
}

_verify_running() {
	sleep 2
	if ! podman ps --format '{{.Names}}' | grep -qx "$_container_name"; then
		echo ''
		echo 'ERROR: Container failed to start!'
		echo 'Container logs:'
		podman logs "$_container_name"
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
