//! MQTT topic and payload formatting
//!
//! Pure formatting functions for MQTT topics and JSON telemetry
//! payloads.  These are platform-agnostic and tested on the host.
//! The actual MQTT client (TLS, TCP, connection management) remains
//! in the board crate.

use heapless::String;

use super::error::MqttError;
use crate::time::Timestamp;
use hal_abstractions::sensor::EnvironmentalReading;

/// Maximum MQTT topic length
///
/// Format: `device/{client_id}/telemetry` where client_id is ~34
/// chars.  Total: 7 + 34 + 10 = 51 chars; 64 provides safety
/// margin.
pub const MAX_TOPIC_LEN: usize = 64;

/// MQTT client configuration
#[derive(Debug, Clone, Copy)]
pub struct MqttConfig {
    /// Broker hostname (for DNS and TLS SNI)
    pub broker_host: &'static str,
    /// Broker port (typically 8883 for MQTTS)
    pub broker_port: u16,
    /// MQTT keep-alive interval in seconds (0 = infinite)
    pub keep_alive_secs: u16,
    /// Clean start flag (true = new session)
    pub clean_start: bool,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker_host: "192.168.1.1",
            broker_port: 8883,
            keep_alive_secs: 60,
            clean_start: true,
        }
    }
}

/// Format an MQTT topic: `device/{client_id}/{subtopic}`
///
/// Validates that neither `client_id` nor `subtopic` contain MQTT
/// wildcard characters (`+`, `#`) or null bytes.
pub fn format_mqtt_topic(
    client_id: &str,
    subtopic: &str,
) -> Result<String<MAX_TOPIC_LEN>, MqttError> {
    if client_id.contains('+') || client_id.contains('#') || client_id.contains('\0') {
        return Err(MqttError::ProtocolError);
    }
    if subtopic.contains('+') || subtopic.contains('#') || subtopic.contains('\0') {
        return Err(MqttError::ProtocolError);
    }

    let mut topic = String::<MAX_TOPIC_LEN>::new();
    topic
        .push_str("device/")
        .map_err(|_| MqttError::BufferError)?;
    topic
        .push_str(client_id)
        .map_err(|_| MqttError::BufferError)?;
    topic.push('/').map_err(|_| MqttError::BufferError)?;
    topic
        .push_str(subtopic)
        .map_err(|_| MqttError::BufferError)?;

    Ok(topic)
}

/// Format a JSON telemetry payload with optional sensor readings
///
/// Produces a JSON object with message metadata and, when a sensor
/// reading is available, environmental data fields.  Fixed-point
/// values are formatted as decimal JSON numbers (e.g., `225` →
/// `22.5`).
pub fn format_json_payload<R: EnvironmentalReading>(
    msg_id: u32,
    ts: &Timestamp,
    reading: Option<&R>,
) -> Result<String<256>, MqttError> {
    use core::fmt::Write;

    let mut buf = String::<256>::new();
    write!(
        &mut buf,
        "{{\"msg_id\":{},\"timestamp\":{},\"micros\":{}",
        msg_id, ts.unix_secs, ts.micros
    )
    .map_err(|_| MqttError::BufferError)?;

    if let Some(r) = reading {
        write_deci_field(&mut buf, ",\"pm1_0\":", r.pm1_0_deci())?;
        write_deci_field(&mut buf, ",\"pm2_5\":", r.pm2_5_deci())?;
        write_deci_field(&mut buf, ",\"pm4_0\":", r.pm4_0_deci())?;
        write_deci_field(&mut buf, ",\"pm10\":", r.pm10_deci())?;
        write_int_field(&mut buf, ",\"co2\":", r.co2_ppm())?;
        write_deci_field(&mut buf, ",\"voc\":", r.voc_index_deci())?;
        write_deci_field(&mut buf, ",\"nox\":", r.nox_index_deci())?;
        write_deci_field(&mut buf, ",\"temp_c\":", r.temperature_deci())?;
        write_deci_field(&mut buf, ",\"humidity\":", r.humidity_deci())?;
    }

    buf.push('}').map_err(|_| MqttError::BufferError)?;

    Ok(buf)
}

/// Write a deci-scaled field as a decimal number (e.g., 225 →
/// "22.5")
pub fn write_deci_field(
    buf: &mut String<256>,
    key: &str,
    val: Option<i32>,
) -> Result<(), MqttError> {
    use core::fmt::Write;

    if let Some(v) = val {
        let sign = if v < 0 { "-" } else { "" };
        let abs_v = v.unsigned_abs();
        let whole = abs_v / 10;
        let frac = abs_v % 10;
        write!(buf, "{}{}{}.{}", key, sign, whole, frac).map_err(|_| MqttError::BufferError)?;
    }

    Ok(())
}

/// Write an integer field (e.g., CO₂ in ppm)
pub fn write_int_field<T: core::fmt::Display>(
    buf: &mut String<256>,
    key: &str,
    val: Option<T>,
) -> Result<(), MqttError> {
    use core::fmt::Write;

    if let Some(v) = val {
        write!(buf, "{}{}", key, v).map_err(|_| MqttError::BufferError)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MqttConfig::default();
        assert_eq!(config.broker_host, "192.168.1.1");
        assert_eq!(config.broker_port, 8883);
        assert_eq!(config.keep_alive_secs, 60);
        assert!(config.clean_start);
    }

    #[test]
    fn test_format_mqtt_topic() {
        let topic = format_mqtt_topic("stm32f405-test123", "telemetry").unwrap();
        assert_eq!(topic.as_str(), "device/stm32f405-test123/telemetry");

        let topic = format_mqtt_topic("stm32f405-test123", "status").unwrap();
        assert_eq!(topic.as_str(), "device/stm32f405-test123/status");

        assert!(topic.len() < MAX_TOPIC_LEN);
    }

    #[test]
    fn test_format_mqtt_topic_buffer_overflow() {
        let long_id = "this_is_a_very_long_client_id_that_exceeds_the\
            _maximum_allowed_topic_length_for_mqtt_messages";
        let result = format_mqtt_topic(long_id, "telemetry");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_mqtt_topic_invalid_characters() {
        assert!(format_mqtt_topic("client+wildcard", "telemetry").is_err());
        assert!(format_mqtt_topic("client#wildcard", "telemetry").is_err());
        assert!(format_mqtt_topic("valid-client", "status+wildcard").is_err());
    }

    #[test]
    fn test_format_json_payload_metadata_only() {
        struct NoSensor;
        impl EnvironmentalReading for NoSensor {}

        let ts = Timestamp::new(1_700_000_000, 123_456);
        let result = format_json_payload(42, &ts, None::<&NoSensor>).unwrap();
        assert_eq!(
            result.as_str(),
            r#"{"msg_id":42,"timestamp":1700000000,"micros":123456}"#
        );
    }

    #[cfg(feature = "sen66")]
    #[test]
    fn test_format_json_payload_with_sensor() {
        use crate::sensor::Sen66Reading;
        let ts = Timestamp::new(1_700_000_000, 0);
        let reading = Sen66Reading {
            pm1_0: Some(52),
            pm2_5: Some(128),
            pm4_0: None,
            pm10: None,
            co2: Some(412),
            voc: None,
            nox: None,
            temp_c: Some(225),
            humidity: Some(452),
        };
        let result = format_json_payload(1, &ts, Some(&reading)).unwrap();
        let s = result.as_str();
        assert!(s.contains("\"pm1_0\":5.2"));
        assert!(s.contains("\"pm2_5\":12.8"));
        assert!(s.contains("\"co2\":412"));
        assert!(s.contains("\"temp_c\":22.5"));
        assert!(s.contains("\"humidity\":45.2"));
        // Fields that are None should not appear
        assert!(!s.contains("pm4_0"));
        assert!(!s.contains("pm10"));
        assert!(!s.contains("voc"));
        assert!(!s.contains("nox"));
    }

    #[cfg(feature = "sen66")]
    #[test]
    fn test_format_json_payload_negative_temps() {
        use crate::sensor::Sen66Reading;

        let ts = Timestamp::new(1_700_000_000, 0);
        let reading = Sen66Reading {
            pm1_0: None,
            pm2_5: None,
            pm4_0: None,
            pm10: None,
            co2: None,
            voc: None,
            nox: None,
            temp_c: Some(-1),
            humidity: None,
        };
        let result = format_json_payload(1, &ts, Some(&reading)).unwrap();
        assert!(result.as_str().contains("\"temp_c\":-0.1"));

        let reading = Sen66Reading {
            temp_c: Some(-9),
            ..reading
        };
        let result = format_json_payload(1, &ts, Some(&reading)).unwrap();
        assert!(result.as_str().contains("\"temp_c\":-0.9"));

        let reading = Sen66Reading {
            temp_c: Some(-105),
            ..reading
        };
        let result = format_json_payload(1, &ts, Some(&reading)).unwrap();
        assert!(result.as_str().contains("\"temp_c\":-10.5"));
    }

    #[test]
    fn test_write_deci_field_edge_cases() {
        let mut buf = String::<256>::new();

        // Zero value
        write_deci_field(&mut buf, ",\"x\":", Some(0)).unwrap();
        assert_eq!(buf.as_str(), ",\"x\":0.0");

        // None value — no output
        let len = buf.len();
        write_deci_field(&mut buf, ",\"y\":", None).unwrap();
        assert_eq!(buf.len(), len);

        // Large value
        let mut buf2 = String::<256>::new();
        write_deci_field(&mut buf2, ",\"z\":", Some(99999)).unwrap();
        assert_eq!(buf2.as_str(), ",\"z\":9999.9");
    }
}
