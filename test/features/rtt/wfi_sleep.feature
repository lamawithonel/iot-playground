Feature: WFI Sleep Mode
  The device must enter WFI sleep between interrupt events
  to minimize CPU activity.  The idle task increments a
  wake counter after each WFI return, which the publish
  loop logs per cycle.

  Background:
    Given I have captured RTT output from a smoke test

  Scenario: Idle task enters WFI loop
    Then the RTT log should contain "entering WFI loop"

  Scenario: WFI wake counter is active
    Then the RTT log should match "wfi_wakes: \d+" given at least 15 seconds

  Scenario: Wake count indicates sleep, not busy-wait
    Then the RTT log should not match "wfi_wakes: \d{6,}" given at least 15 seconds
