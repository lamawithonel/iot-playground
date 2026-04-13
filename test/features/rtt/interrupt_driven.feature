Feature: Interrupt-Driven Packet Reception
  The device must use hardware interrupts (EXTI2) for W5500
  packet reception, not polling.  The counting EXTI wrapper
  increments a counter on each interrupt-driven wait
  completion, logged per MQTT publish cycle alongside
  wfi_wakes.

  Background:
    Given I have captured RTT output from a smoke test

  Scenario: EXTI2 events are detected during network activity
    Then the RTT log should match "exti2_events: [1-9]\d*" given at least 15 seconds

  Scenario: EXTI2 event count is bounded, not polling
    Then the RTT log should not match "exti2_events: \d{5,}" given at least 15 seconds

  Scenario: Every publish cycle has EXTI2 activity
    Then the RTT log should not match "exti2_events: 0\b" given at least 15 seconds
