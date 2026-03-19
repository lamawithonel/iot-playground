# Mosquitto MQTT Broker Test Setup

This directory contains a container setup for running a local Mosquitto
MQTT broker with TLS support, used for testing the STM32F405 TLS
handshake and MQTT client implementation.

## Quick Start

```bash
# From the repository root:
mise run broker:start

# Or directly:
test/broker/start-broker.sh
```

The script will:

1. Build a container image with Mosquitto and self-signed ECDSA
   certificates
2. Start the broker on `192.168.1.1:8883` (interface `eno1`)

View logs with `mise run broker:logs` (or `podman logs -f
mosquitto-tls`).

## Stopping the Broker

```bash
mise run broker:stop

# Or directly:
test/broker/stop-broker.sh
```

## Configuration

### Ports

- **1883**: MQTT (non-encrypted)
- **8883**: MQTTS (TLS encrypted)

### TLS Settings

- **ECDSA** self-signed certificate generated at build time (required
  for embedded-tls compatibility)
- CN (Common Name) set to `192.168.1.1` to match the STM32 client
  configuration
- Supports TLS 1.2 and TLS 1.3
- Uses ECDSA P-384 (secp384r1) curve for keys
- Compatible with embedded-tls cipher suite: TLS_AES_128_GCM_SHA256

### Cipher Suites

The broker supports TLS 1.3 cipher suites including:

- `TLS_AES_128_GCM_SHA256` ✓ **Compatible with embedded-tls**
- `TLS_AES_256_GCM_SHA384`
- `TLS_CHACHA20_POLY1305_SHA256`

TLS 1.2 fallback ciphers use ECDHE-ECDSA (matching the ECDSA
certificates).

> **Note:** embedded-tls requires ECDSA certificates.  The
> Containerfile generates ECDSA (not RSA) certificates for
> compatibility.

## Manual Podman Commands

### Build

```bash
podman build -t mosquitto-tls:latest test/broker/
```

### Run

```bash
podman run -d \
    --name mosquitto-tls \
    -p 192.168.1.1:1883:1883 \
    -p 192.168.1.1:8883:8883 \
    -v mosquitto-data:/mosquitto/data \
    -v mosquitto-log:/mosquitto/log \
    mosquitto-tls:latest
```

### View Logs

```bash
podman logs -f mosquitto-tls

# Or:
mise run broker:logs
```

### Stop and Remove

```bash
podman stop mosquitto-tls
podman rm mosquitto-tls

# Or:
mise run broker:stop
```

## Testing the TLS Connection

### Using mosquitto_sub (MQTT client)

```bash
# Extract the CA certificate from the container
podman cp mosquitto-tls:/mosquitto/certs/ca.crt ./ca.crt

# Subscribe to a topic over TLS
mosquitto_sub -h 192.168.1.1 -p 8883 --cafile ca.crt -t test/topic
```

### Using openssl

```bash
# Test TLS handshake
openssl s_client -connect 192.168.1.1:8883 -showcerts

# Test with specific TLS version
openssl s_client -connect 192.168.1.1:8883 -tls1_3
```

### Check supported ciphers with nmap

```bash
nmap --script ssl-enum-ciphers -p 8883 192.168.1.1
```

## Troubleshooting

### Interface not found

If interface `eno1` doesn't exist on your system, edit
`start-broker.sh` and change `_interface`:

```bash
_interface='eth0'  # or your actual interface name
```

### View available interfaces

```bash
ip -br addr
```

### Certificate issues

The certificates are regenerated each time the image is rebuilt.  If
you need persistent certificates, mount a volume:

```bash
-v ./certs:/mosquitto/certs
```

## Notes

- The broker allows anonymous connections for testing
- Self-signed certificates will trigger warnings in production clients
- For production use, replace with proper CA-signed certificates
- The container automatically restarts unless explicitly stopped
