Feature: Security Initialization
  The device must initialize its hardware RNG and establish
  a TLS connection to the MQTT broker.

  Background:
    Given I have captured RTT output from a smoke test

  Scenario: Hardware RNG initialization
    Then the RTT log should match "Hardware RNG initialized|RNG"

  Scenario: TLS handshake
    Then the RTT log should match "TLS 1\\.3 handshake OK|TLS.*handshake"
