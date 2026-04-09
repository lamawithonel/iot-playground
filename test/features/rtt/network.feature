Feature: Network Connectivity
  The device must obtain an IP address via DHCP, synchronize
  its clock via SNTP, and calibrate the internal RTC.

  Background:
    Given I have captured RTT output from a smoke test

  Scenario: DHCP address acquisition
    Then the RTT log should match "Network is UP|IP:"

  Scenario: SNTP time synchronization
    Then the RTT log should contain "SNTP sync successful"

  Scenario: RTC wall-clock calibration
    Then the RTT log should contain "Wall-clock calibrated" or "written to internal RTC"
