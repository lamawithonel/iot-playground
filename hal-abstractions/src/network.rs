//! Network link and configuration readiness abstraction
//!
//! `iot-net::manager::wait_for_config` gates every board's startup on
//! `embassy_net::Stack::wait_config_up()` before running any client--
//! but that single await conflates two different facts into one
//! indefinitely-pending future: "no physical link" (W5500 unplugged
//! or not yet wired, on-chip MAC with no cable) and "link up, DHCP
//! still negotiating".  A board cannot tell which one it is stuck on
//! without a second, physical-layer signal alongside the wait.
//!
//! `embassy_net::Stack` is an `embassy-net` type, so it cannot appear
//! in this crate (see the crate's local rules).  This trait instead
//! names the two readiness facts a board's own link driver already
//! has-- the W5500 PHY link register, an on-chip MAC's carrier
//! sense, and `Stack::wait_config_up`'s own await-- so board-level
//! readiness logic (log or fault differently on "no link" vs "DHCP
//! pending") becomes host-testable against a mock instead of welded
//! to a `Stack` that cannot build for the host target at all (see
//! `iot-net/AGENTS.md`).  This is deliberately narrower than the
//! DNS/TCP/TLS/MQTT session abstraction `lib.rs` once reserved a
//! `network` module for-- that shape was superseded by `iot-net`
//! working directly over `embassy-net`, and stays superseded; this
//! trait covers only the readiness gate, not the session itself.

/// Board-side network readiness: physical link state plus IP
/// configuration state.
///
/// Rationale: every current and near-term board (Feather's W5500 SPI
/// link, the N6 toolhead's planned W5500 module) already tracks its
/// own physical link state internally; this trait is the minimal
/// surface for a board to expose that fact next to the IP-config
/// wait `iot-net` already performs, without pulling `embassy-net`
/// into this crate.
pub trait NetworkReadiness {
    /// Whether the physical link is up (cable connected, PHY link
    /// detected)-- independent of IP configuration.
    fn is_link_up(&self) -> bool;

    /// Wait for IP-layer configuration (e.g. a DHCP lease) to
    /// complete.
    ///
    /// Pending forever if configuration never completes, matching
    /// `embassy_net::Stack::wait_config_up`'s semantics exactly so a
    /// board's `Stack`-backed implementation delegates in one line.
    // Single-core RTIC executor; no `Send` bound wanted (see the
    // design's async-story decision).
    #[allow(async_fn_in_trait)]
    async fn wait_config_up(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{now_or_never, MockNetworkReadiness};

    #[test]
    fn fresh_mock_reports_link_down() {
        let link = MockNetworkReadiness::new();
        assert!(!link.is_link_up());
    }

    #[test]
    fn set_link_up_is_reflected_immediately() {
        let mut link = MockNetworkReadiness::new();
        link.set_link_up(true);
        assert!(link.is_link_up());
        link.set_link_up(false);
        assert!(!link.is_link_up());
    }

    #[test]
    fn wait_config_up_is_pending_until_config_completes() {
        let mut link = MockNetworkReadiness::new();
        assert_eq!(now_or_never(link.wait_config_up()), None);

        link.set_config_up(true);
        assert_eq!(now_or_never(link.wait_config_up()), Some(()));
    }

    #[test]
    fn link_state_and_config_state_are_independent() {
        // The whole point of the split: a board can be link-up with
        // DHCP still pending, distinguishable from no-link-at-all--
        // both of which `Stack::wait_config_up` alone reports as the
        // same pending future.
        let mut link = MockNetworkReadiness::new();
        link.set_link_up(true);
        assert!(link.is_link_up());
        assert_eq!(now_or_never(link.wait_config_up()), None);
    }

    /// Generic consumer proof: the shape a board startup gate uses to
    /// stay host-testable instead of welded to `embassy_net::Stack`.
    async fn log_readiness<L: NetworkReadiness>(link: &mut L) -> &'static str {
        if !link.is_link_up() {
            return "no link";
        }
        link.wait_config_up().await;
        "configured"
    }

    #[test]
    fn generic_consumer_bound_is_usable() {
        let mut link = MockNetworkReadiness::new();
        assert_eq!(now_or_never(log_readiness(&mut link)), Some("no link"));

        link.set_link_up(true);
        link.set_config_up(true);
        assert_eq!(now_or_never(log_readiness(&mut link)), Some("configured"));
    }

    #[test]
    #[ignore = "RED: needs a W5500 SPI module wired to the N6 morpho SPI \
                pins (none on the bench today; see the roadmap's network \
                findings) to observe a real PHY link-up transition and \
                correlate it to NetworkReadiness::is_link_up"]
    fn test_real_w5500_link_up_transition_observed_on_bench() {
        todo!(
            "requires a W5500 module wired to the NUCLEO-N657X0-Q morpho \
             SPI pins; no such module is on the bench today"
        )
    }

    #[test]
    #[ignore = "RED: needs a live network with a DHCP server reachable \
                from the bench to measure real lease-acquisition timing; \
                the host mock's wait_config_up only proves the \
                poll/pending contract, not real DHCP latency"]
    fn test_real_dhcp_lease_acquired_within_declared_timeout() {
        todo!(
            "requires a live network and a real embassy-net Stack \
             driving an on-chip MAC or W5500 link; see the roadmap's \
             network findings"
        )
    }
}
