# Mosquitto MQTT Broker Docker Setup

This directory contains a Docker setup for running a local Mosquitto MQTT broker with TLS support for testing the STM32F405 TLS handshake implementation.

## Quick Start

```bash
cd docker
./start-mosquitto.sh
```

The script will:
1. Build a Docker image with Mosquitto and self-signed certificates
2. Start the broker on `192.168.1.1:8883` (interface `eno1`)
3. Display live logs

## Configuration

### Ports
- **1883**: MQTT (non-encrypted)
- **8883**: MQTTS (TLS encrypted)

### TLS Settings
- **ECDSA** self-signed certificate generated at build time (required for embedded-tls compatibility)
- CN (Common Name) set to `192.168.1.1` to match the STM32 client configuration
- Supports TLS 1.3
- Uses ECDSA P-256 (secp256r1) curve for keys
- Compatible with embedded-tls cipher suite: TLS_AES_128_GCM_SHA256

### Cipher Suites
The broker supports TLS 1.3 cipher suites including:
- `TLS_AES_128_GCM_SHA256` ✓ **Compatible with embedded-tls**
- `TLS_AES_256_GCM_SHA384`
- `TLS_CHACHA20_POLY1305_SHA256`

**Note:** embedded-tls requires ECDSA certificates. The Dockerfile generates ECDSA (not RSA) certificates for compatibility.

## Manual Docker Commands

### Build
```bash
docker build -t mosquitto-tls:latest .
```

### Run
```bash
docker run -d \
    --name mosquitto-tls \
    -p 192.168.1.1:1883:1883 \
    -p 192.168.1.1:8883:8883 \
    -v mosquitto-data:/mosquitto/data \
    -v mosquitto-log:/mosquitto/log \
    mosquitto-tls:latest
```

### View Logs
```bash
docker logs -f mosquitto-tls
```

### Stop
```bash
docker stop mosquitto-tls
```

### Remove
```bash
docker rm mosquitto-tls
```

## Testing the TLS Connection

### Using mosquitto_sub (MQTT client)
```bash
# Extract the CA certificate from the container
docker cp mosquitto-tls:/mosquitto/certs/ca.crt ./ca.crt

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
If interface `eno1` doesn't exist on your system, modify the script:
```bash
INTERFACE="eth0"  # or your actual interface name
```

### View available interfaces
```bash
ip -br addr
```

### Certificate issues
The certificates are regenerated each time you rebuild the image. If you need persistent certificates, mount a volume:
```bash
-v ./certs:/mosquitto/certs
```

## Notes

- The broker allows anonymous connections for testing
- Self-signed certificates will trigger warnings in production clients
- For production use, replace with proper CA-signed certificates
- The container automatically restarts unless explicitly stopped
