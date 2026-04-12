Feature: MQTT Sensor Telemetry Ranges
  Sensor readings must be within physically plausible
  ranges once sensors have conditioned.

  Background:
    Given I have captured MQTT messages from a smoke test

  Scenario Outline: <field> values are within range
    When the field "<field>" is present in any message
    Then all "<field>" values should be between <min> and <max>

    Examples:
      | field    | min   | max     |
      | temp_c   | -40.0 | 85.0    |
      | humidity | 0.0   | 100.0   |
      | pm1_0    | 0.0   | 1000.0  |
      | pm2_5    | 0.0   | 1000.0  |
      | pm4_0    | 0.0   | 1000.0  |
      | pm10     | 0.0   | 1000.0  |
      | co2      | 0.0   | 10000.0 |
      | voc      | 0.0   | 500.0   |
      | nox      | 0.0   | 500.0   |

  Scenario Outline: <field> conditions within <threshold> seconds
    When the test ran longer than <threshold> seconds
    Then the last message should have a "<field>" value

    Examples:
      | field    | threshold |
      | temp_c   | 43        |
      | humidity | 43        |
      | voc      | 95        |
      | pm1_0    | 155       |
      | pm2_5    | 155       |
      | pm4_0    | 155       |
      | pm10     | 155       |

    @extended
    Examples:
      | field    | threshold |
      | co2      | 215       |
