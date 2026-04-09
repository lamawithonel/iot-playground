Feature: Device Boot Sequence
  The device must complete its boot sequence with all
  subsystems initialized.

  Background:
    Given I have captured RTT output from a smoke test

  Scenario: Core system initialization
    Then the RTT log should contain "IoT Playground starting"
    And the RTT log should contain "System initialized"
    And the RTT log should contain "TIM2 monotonic"
    And the RTT log should contain "RTC initialized"
    And the RTT log should contain "I2C1 initialized"

  Scenario: Clock configuration
    Then the RTT log should match "SYSCLK=84MHz|PLL configured"

  Scenario: Sensor initialization
    Then the RTT log should contain "SEN66 initialized" or "continuous measurement started"

  Scenario: Network peripheral initialization
    Then the RTT log should contain "W5500 initialized"
