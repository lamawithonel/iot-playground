#!/bin/bash
# Launch script for Mosquitto MQTT broker with TLS support
# Binds to interface eno1 (192.168.1.1:8883)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INTERFACE="eno1"
HOST_IP="192.168.1.1"

echo "=== Mosquitto MQTT Broker with TLS ==="
echo "Script directory: $SCRIPT_DIR"
echo "Binding to interface: $INTERFACE ($HOST_IP)"
echo ""

# Check if interface exists
if ! ip addr show "$INTERFACE" &>/dev/null; then
    echo "ERROR: Interface $INTERFACE not found!"
    echo "Available interfaces:"
    ip -br addr
    exit 1
fi

# Get the IP address of the interface
ACTUAL_IP=$(ip -4 addr show "$INTERFACE" | grep -oP '(?<=inet\s)\d+(\.\d+){3}')
if [ -z "$ACTUAL_IP" ]; then
    echo "ERROR: No IPv4 address found on interface $INTERFACE"
    exit 1
fi

echo "Interface $INTERFACE has IP: $ACTUAL_IP"
if [ "$ACTUAL_IP" != "$HOST_IP" ]; then
    echo "WARNING: Expected IP $HOST_IP but found $ACTUAL_IP"
    echo "Using actual IP: $ACTUAL_IP"
    HOST_IP="$ACTUAL_IP"
fi

# Stop and remove existing container if it exists
echo ""
echo "Cleaning up existing container..."
docker stop mosquitto-tls 2>/dev/null || true
docker rm mosquitto-tls 2>/dev/null || true

# Build the Docker image
echo ""
echo "Building Docker image..."
cd "$SCRIPT_DIR"
docker build -t mosquitto-tls:latest .

# Create volumes for persistence
echo ""
echo "Creating Docker volumes..."
docker volume create mosquitto-data 2>/dev/null || true
docker volume create mosquitto-log 2>/dev/null || true

# Run the container
echo ""
echo "Starting Mosquitto MQTT broker..."
docker run -d \
    --name mosquitto-tls \
    --restart unless-stopped \
    -p ${HOST_IP}:1883:1883 \
    -p ${HOST_IP}:8883:8883 \
    -v mosquitto-data:/mosquitto/data \
    -v mosquitto-log:/mosquitto/log \
    mosquitto-tls:latest

# Wait for container to start
sleep 2

# Check if container is running
if ! docker ps | grep -q mosquitto-tls; then
    echo ""
    echo "ERROR: Container failed to start!"
    echo "Container logs:"
    docker logs mosquitto-tls
    exit 1
fi

echo ""
echo "=== Mosquitto MQTT Broker Started Successfully ==="
echo ""
echo "MQTT (non-TLS):  ${HOST_IP}:1883"
echo "MQTTS (TLS):     ${HOST_IP}:8883"
echo ""
echo "To view logs:    docker logs -f mosquitto-tls"
echo "To stop:         docker stop mosquitto-tls"
echo "To remove:       docker rm mosquitto-tls"
echo ""
echo "Test connection with:"
echo "  mosquitto_sub -h ${HOST_IP} -p 8883 --cafile <path-to-ca.crt> -t test/topic"
echo ""
echo "Viewing live logs (Ctrl+C to exit)..."
docker logs -f mosquitto-tls
