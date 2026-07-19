#![deny(unsafe_code)]
#![deny(warnings)]
//! Network configuration structures

/// SNTP client configuration
#[derive(Debug, Clone)]
pub struct SntpConfig {
    /// NTP servers to try (in order)
    pub servers: &'static [&'static str],
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Number of retry attempts per server
    pub retry_count: usize,
    /// Maximum accepted stratum level (1-15)
    pub max_stratum: u8,
}

impl Default for SntpConfig {
    fn default() -> Self {
        Self {
            servers: &["pool.ntp.org", "time.google.com", "time.cloudflare.com"],
            timeout_ms: 5000,
            retry_count: 3,
            max_stratum: 3,
        }
    }
}
