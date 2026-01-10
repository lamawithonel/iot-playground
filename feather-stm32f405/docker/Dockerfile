# Dockerfile for Mosquitto MQTT Broker with TLS support
FROM eclipse-mosquitto:2.0

# Copy mosquitto configuration
COPY mosquitto.conf /mosquitto/config/mosquitto.conf

# Generate ECDSA self-signed certificate for testing
RUN apk add --no-cache openssl && \
    mkdir -p /mosquitto/certs && \
    cd /mosquitto/certs && \
    # Generate ECDSA CA certificate
    openssl ecparam -name secp384r1 -genkey -noout -out ca.key && \
    openssl req -new -x509 -days 365 -key ca.key -out ca.crt \
        -subj "/CN=LamaNet Test CA" && \
    # Generate ECDSA server certificate
    openssl ecparam -name secp384r1 -genkey -noout -out server.key && \
    openssl req -new -key server.key -out server.csr \
        -subj "/CN=192.168.1.1" && \
    openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key \
        -CAcreateserial -out server.crt -days 365 -sha384 && \
    rm server.csr && \
    chmod 644 /mosquitto/certs/* && \
    apk del openssl

# Expose ports
EXPOSE 1883 8883

# Run mosquitto
CMD ["/usr/sbin/mosquitto", "-c", "/mosquitto/config/mosquitto.conf"]
