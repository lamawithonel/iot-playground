#!/usr/bin/env bash
# Stop and remove the Mosquitto MQTT test broker container.

set -o errexit
set -o nounset
set -o pipefail

_container_name='mosquitto-tls'

_main() {
	if podman ps -a --format '{{.Names}}' | grep -q "^${_container_name}$"; then
		echo 'Stopping Mosquitto MQTT broker...'
		podman stop "$_container_name" 2>/dev/null || true
		podman rm "$_container_name" 2>/dev/null || true
		echo 'Broker stopped and removed.'
	else
		echo 'Broker is not running.'
	fi
}

_main
