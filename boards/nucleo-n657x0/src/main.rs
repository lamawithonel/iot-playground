//! ARS toolhead sensor-- NUCLEO-N657X0-Q board crate.
//!
//! Planned RTIC 2.x task topology, per
//! `docs/src/projects/ars-toolhead-sensor/README.md`:
//!
//! - `init`: clocks, pins, audio output path, ADC setup
//! - `sweep_engine`: swept-sine synthesis feeding the MAX9744
//! - `capture`: ADC capture windows synchronized to the sweep
//! - `analysis`: FFT, feature extraction, and classification
//! - `telemetry`: labels and features over the framework MQTT
//!   stack once ported
//! - `idle`: WFI sleep
//!
//! Framework rules apply: RTIC 2.x scheduling, Embassy HAL only
//! (no embassy-executor), `no_std`, no heap, defmt logging.
//!
//! The default feature set is still a deliberate `compile_error!`
//! scaffold guard.  Under the `g1-spike` feature the crate builds
//! and boots: the gate G0 RAM-boot flow was bench-verified
//! 2026-07-21 (see the board page's bring-up section for the load
//! flow of record).  `g1-spike` now also carries the phase-1
//! implementation (the `app` module below)-- `button_task` and the
//! `AMP_MUTE_N`/`AMP_I2C` inits are real, interrupt-driven code;
//! `capture`/`sweep_engine` stay empty hardware-task shells (see
//! their own doc comments) pending ADC1/TIM1 landing in
//! embassy-stm32 for this chip.
//!
//! The additive `net` feature (which pulls in `g1-spike`) layers on
//! on-chip Ethernet: ETH1 over RMII to the on-board LAN8742A PHY,
//! feeding an embassy-net DHCPv4 stack driven from one RTIC
//! software task (`network_task`).  It adds nothing to the phase-1
//! tasks, so `g1-spike` alone is unchanged.
//!
//! See `pins.rs` for the pin map of record in code form.

#![no_std]
#![no_main]
#![deny(warnings)]
#![deny(unsafe_code)]

#[cfg(not(feature = "g1-spike"))]
compile_error!("scaffold only -- see boards/nucleo-n657x0/README.md");

// Plain `&str` consts, no embassy deps-- compiles in every feature
// configuration.  See the module doc for which consts the `app`
// module below actually consumes.
mod pins;

// MAC/seed helpers for the on-chip Ethernet stack; only under the
// additive `net` feature (see `net.rs` and the `network_task` task
// in the `app` module).
#[cfg(feature = "net")]
mod net;

// ── Gate G1 / phase-1 RTIC app ───────────────────────────────────
// Proves embassy-stm32 + rtic 2.x + rtic-monotonics +
// defmt/defmt-rtt/panic-probe compile and run for the Cortex-M55
// (thumbv8m.main-none-eabihf target; there is no distinct
// thumbv8.1m.main rustc target-- ARMv8.1-M/Helium is selected via
// target-cpu/target-feature on top of v8-M mainline), and hosts the
// phase-1 task topology.  `capture`/`sweep_engine` are dormant
// hardware-task shells (empty bodies, no transfer ever configured)
// because ADC1 and TIM1 have no embassy-stm32 0.6.0 peripheral
// singleton for `stm32n657x0` in this crate's locked dependency
// version-- see their doc comments below.  See
// docs/src/projects/ars-toolhead-sensor/pinout.md gates G1-G4.
#[cfg(feature = "g1-spike")]
#[rtic::app(device = embassy_stm32, dispatchers = [USART1, USART2])]
mod app {
    use core::sync::atomic::{AtomicU32, Ordering};

    use defmt_rtt as _;
    use embassy_stm32::exti::ExtiInput;
    use embassy_stm32::gpio::{Level, OutputOpenDrain, Pull, Speed};
    use embassy_stm32::i2c::I2c;
    use embassy_stm32::mode::Async;
    use embassy_stm32::peripherals;
    use embassy_stm32::time::Hertz;
    use embassy_stm32::Peri;
    use panic_probe as _;
    use rtic_monotonics::systick::prelude::*;

    use super::pins;

    systick_monotonic!(Mono, 1_000);

    /// Count of `USER_BTN` (PC13/EXTI13) falling-edge presses
    /// observed since boot.
    ///
    /// Incremented only by [`button_task`]; nothing reads it back
    /// yet in phase-1.  A future telemetry task is the intended
    /// consumer, per `docs/src/projects/ars-toolhead-sensor/
    /// README.md`'s planned `telemetry` task.
    static USER_BTN_EVENTS: AtomicU32 = AtomicU32::new(0);

    // USER_BTN (PC13/EXTI13) interrupt binding for embassy's async
    // EXTI driver.  A raw RTIC `#[task(binds = EXTI13)]` hardware
    // task cannot safely coexist with this: embassy's `ExtiInput`
    // owns pending-bit clearing and waker registration for the
    // vector internally, and duplicating that at the PAC level
    // would need `unsafe`, forbidden in this crate.  `button_task`
    // below is still fully interrupt-driven (no polling)-- it just
    // isn't a literal `#[task(binds = ...)]` hardware task.  Mirrors
    // pinout.md's own USER_BTN rationale
    // (`boards/feather-stm32f405/src/counting_exti.rs`).
    embassy_stm32::bind_interrupts!(struct ExtiIrqs {
        EXTI13 => embassy_stm32::exti::InterruptHandler<embassy_stm32::interrupt::typelevel::EXTI13>;
    });

    // ETH1 interrupt binding for the on-chip Ethernet MAC.  Installs
    // a real vector-table handler (same mechanism as `ExtiIrqs`
    // above), disjoint from RTIC's software-task dispatchers-- ETH1
    // is deliberately NOT in the `dispatchers` list.  The handler
    // wakes the embassy-net runner so `network_task` re-runs on each
    // RX/TX event; the CPU is WFI-idle in between.
    #[cfg(feature = "net")]
    embassy_stm32::bind_interrupts!(struct EthIrqs {
        ETH1 => embassy_stm32::eth::InterruptHandler;
    });

    /// ETH1 RMII peripheral bundle handed to [`network_task`] at
    /// spawn time.
    ///
    /// Everything here is a `Peri<'static, _>` token, which is
    /// `Send`, so it can cross the RTIC spawn boundary.  The
    /// embassy-net `Stack`/`Runner` built from these are `!Send`
    /// (they hold `&RefCell<_>`), so-- following the feather
    /// precedent-- the whole stack is constructed and driven inside
    /// the single `network_task` and never leaves it.
    ///
    /// The struct itself is defined in every feature configuration
    /// (its fields are `net`-gated to an empty struct otherwise) so
    /// that `network_task` can stay a normal, always-present RTIC
    /// task.  RTIC 2.2's async dispatcher references every task's
    /// executor unconditionally (`// TODO: Fix cfg` in its codegen),
    /// so a `#[cfg]`-gated-away task would leave a dangling reference
    /// and fail to build under `g1-spike` alone.
    struct EthPeripherals {
        #[cfg(feature = "net")]
        eth1: Peri<'static, peripherals::ETH1>,
        #[cfg(feature = "net")]
        sma: Peri<'static, peripherals::ETH_SMA>,
        #[cfg(feature = "net")]
        ref_clk: Peri<'static, peripherals::PF7>,
        #[cfg(feature = "net")]
        crs: Peri<'static, peripherals::PF10>,
        #[cfg(feature = "net")]
        rxd0: Peri<'static, peripherals::PF14>,
        #[cfg(feature = "net")]
        rxd1: Peri<'static, peripherals::PF15>,
        #[cfg(feature = "net")]
        txd0: Peri<'static, peripherals::PF12>,
        #[cfg(feature = "net")]
        txd1: Peri<'static, peripherals::PF13>,
        #[cfg(feature = "net")]
        tx_en: Peri<'static, peripherals::PF11>,
        #[cfg(feature = "net")]
        mdio: Peri<'static, peripherals::PF4>,
        #[cfg(feature = "net")]
        mdc: Peri<'static, peripherals::PG11>,
    }

    /// RTIC shared resources.
    ///
    /// Empty in phase-1: no state is contended across task
    /// priorities yet.
    #[shared]
    struct Shared {}

    /// RTIC local (task-exclusive) resources.
    #[local]
    struct Local {
        /// AMP_MUTE_N (PD0), open-drain GPIO output, driven low
        /// (muted) at boot.  Open-drain per pinout.md: the surveyed
        /// facts do not identify which rail the amp board's R15
        /// pull-up returns to, so the MCU must never actively drive
        /// the net high.  Owned by [`sweep_engine`]: it is the
        /// natural long-term owner since a sweep should stay muted
        /// between plays and unmuted only while one is active.  Pin
        /// source: pinout.md ("Amplifier mute control; drive LOW to
        /// mute").
        amp_mute_n: OutputOpenDrain<'static>,
        /// USER_BTN (PC13/EXTI13), async embassy EXTI input, owned
        /// by [`button_task`].  `Pull::Up` + falling-edge assumes
        /// idle-high/pressed-low wiring for the on-board B1 button;
        /// bench-unverified.
        user_btn: ExtiInput<'static, Async>,
        /// GPDMA1 channel 0, reserved for the ADC1 capture
        /// transfer.  Unconfigured: ADC1 has no embassy-stm32
        /// 0.6.0 peripheral singleton for `stm32n657x0` (no
        /// register-block metadata in `stm32-metapac` 21.0.0).
        /// See [`capture`].
        capture_dma_ch: Peri<'static, peripherals::GPDMA1_CH0>,
        /// GPDMA1 channel 1, reserved for the TIM1 duty-stream
        /// transfer.  Unconfigured: TIM1 has no embassy-stm32
        /// 0.6.0 peripheral singleton for `stm32n657x0` either
        /// (only TIM9, the time driver, is generated for this
        /// chip).  See [`sweep_engine`].
        sweep_dma_ch: Peri<'static, peripherals::GPDMA1_CH1>,
    }

    /// RTIC init.  Claims PD0/PC13/EXTI13/I2C1(PH9,PC1)/
    /// GPDMA1_CH0/GPDMA1_CH1, starts the SysTick monotonic, and
    /// spawns [`analysis`] and [`button_task`].
    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        // Embassy-stm32 API drift (bench finding, embassy git rev
        // 12e4b1adde4e01b23506822c621451f8e6199c81 pin): the default
        // `rcc::Config` now sets `supply_config: SupplyConfig::Smps`,
        // and `embassy_stm32::init`'s power-sequencing code spins
        // forever on `PWR.voscr().actvosrdy()` for that path-- this
        // board's SMPS regulator never reports ready, hanging boot
        // before the first defmt log line (bench-reproduced: core
        // parked at `power_supply_config`'s ready-wait loop,
        // `src/rcc/n6.rs`).  `SupplyConfig::External` matches both
        // ST's own Nucleo FSBL reference (`SystemClock_Config`'s
        // `PWR_EXTERNAL_SOURCE_SUPPLY`) and embassy's own DK example
        // (commit c2e43ba76, "mostly built for the DK"), and clears
        // the same ready-wait promptly on this hardware.  Everything
        // else stays default-- see the sysclk placeholder note below.
        let mut config = embassy_stm32::Config::default();
        config.rcc.supply_config = embassy_stm32::rcc::SupplyConfig::External;
        let p = embassy_stm32::init(config);

        // Sysclk value is a placeholder: beyond the supply-config
        // fix above, `embassy_stm32::init` still runs with the
        // otherwise-default RCC config here, not an explicit PLL/
        // clock-tree config, so the true post-boot sysclk is
        // unconfirmed.  Monotonic timing accuracy is not a phase-1
        // concern (no on-device work tonight-- compile + clippy
        // only).
        Mono::start(cx.core.SYST, 200_000_000);

        defmt::info!("g1: nucleo-n657x0 phase-1 alive");

        // AMP_MUTE_N (PD0): drive low at boot so the amp stays
        // muted before any real audio content exists.  No AF; GPIO
        // output, open-drain per pinout.md (the amp board's R15
        // pull-up rail is unidentified, so the MCU must not drive
        // the net high itself).
        defmt::info!("AMP_MUTE_N ({}): muted low", pins::AMP_MUTE_N);
        let amp_mute_n = OutputOpenDrain::new(p.PD0, Level::Low, Speed::Low);

        // USER_BTN (PC13/EXTI13): async ExtiInput-- see the
        // `ExtiIrqs` binding comment above for why this isn't a raw
        // RTIC hardware-task bind.
        defmt::info!("USER_BTN ({}): EXTI13 armed", pins::USER_BTN);
        let user_btn = ExtiInput::new(p.PC13, p.EXTI13, Pull::Up, ExtiIrqs);

        // AMP_I2C (I2C1, PH9 SCL / PC1 SDA): blocking init only--
        // no transactions attempted.  Gate G4 (MAX9744 present at
        // 0x4B) is unconfirmed, so real ownership/storage for
        // volume-control writes is deferred to G4; the instance is
        // constructed and dropped here purely to prove the
        // peripheral claims and clock enable compile and run.
        defmt::info!(
            "AMP_I2C: I2C1 SCL={} SDA={} (init-only)",
            pins::AMP_I2C_SCL,
            pins::AMP_I2C_SDA
        );
        let mut i2c_config = embassy_stm32::i2c::Config::default();
        i2c_config.frequency = Hertz(100_000);
        let _amp_i2c = I2c::new_blocking(p.I2C1, p.PH9, p.PC1, i2c_config);

        // ── ETH1 (RMII) claim -> network_task ──────────────────────
        // Under `net`, claim ETH1, the SMA (MDIO/MDC controller), and
        // the 7-wire RMII pin set and hand them to `network_task`,
        // which owns the whole embassy-net stack (see its doc comment
        // for the PHY identity/provenance and the single-task
        // rationale).  Only `Send` peripheral tokens cross the spawn
        // boundary.  Under `g1-spike` alone the bundle is empty and
        // the task is a no-op-- `network_task` is always spawned so
        // RTIC's dispatcher glue resolves in both feature sets (see
        // `EthPeripherals`).
        #[cfg(feature = "net")]
        let eth_periph = EthPeripherals {
            eth1: p.ETH1,
            sma: p.ETH_SMA,
            ref_clk: p.PF7,
            crs: p.PF10,
            rxd0: p.PF14,
            rxd1: p.PF15,
            txd0: p.PF12,
            txd1: p.PF13,
            tx_en: p.PF11,
            mdio: p.PF4,
            mdc: p.PG11,
        };
        #[cfg(not(feature = "net"))]
        let eth_periph = EthPeripherals {};
        network_task::spawn(eth_periph).ok();

        // GPDMA1_CH0/CH1 are claimed here and handed to `capture`/
        // `sweep_engine` below via `local`; neither has a transfer
        // configured yet (see their doc comments).
        analysis::spawn().ok();
        button_task::spawn().ok();

        (
            Shared {},
            Local {
                amp_mute_n,
                user_btn,
                capture_dma_ch: p.GPDMA1_CH0,
                sweep_dma_ch: p.GPDMA1_CH1,
            },
        )
    }

    /// ADC capture GPDMA hardware task-- bound to GPDMA1 channel 0
    /// completion, owns [`Local::capture_dma_ch`].
    ///
    /// Empty body: nothing configures a transfer on this channel,
    /// so this interrupt never fires in phase-1.  Blocked on gate
    /// G1 runtime (confirming GPDMA linked-list/circular-capture
    /// mode) and gate G2 (adaptation-amp capture quality), and more
    /// fundamentally on ADC1 itself: `stm32-metapac` 21.0.0 has
    /// `registers: None` for both the "ADC1" and "ADC12_COMMON"
    /// peripheral entries for this chip, so `embassy_stm32::adc`
    /// cannot target ADC1 here in any mode, not even blocking
    /// calibration-only init.  See pinout.md gates G1-G2.
    #[task(binds = GPDMA1_CHANNEL0, local = [capture_dma_ch], priority = 3)]
    fn capture(_cx: capture::Context) {}

    /// TIM1 duty-stream GPDMA hardware task-- bound to GPDMA1
    /// channel 1 completion, owns [`Local::sweep_dma_ch`] and
    /// [`Local::amp_mute_n`].
    ///
    /// Empty body, same dormant status as [`capture`]: TIM1 is
    /// entirely absent as a peripheral singleton for `stm32n657x0`
    /// in embassy-stm32 0.6.0 (only TIM9, already claimed as the
    /// time driver, is generated for this chip)-- there is no PWM
    /// API surface to build a carrier skeleton against yet.  See
    /// pinout.md gate G3.
    #[task(binds = GPDMA1_CHANNEL1, local = [sweep_dma_ch, amp_mute_n], priority = 3)]
    fn sweep_engine(_cx: sweep_engine::Context) {}

    /// USER_BTN (PC13/EXTI13) press counter.
    ///
    /// Awaits falling edges on the async `ExtiInput`
    /// ([`Local::user_btn`])-- interrupt-driven throughout, no
    /// polling.  Mirrors `feather-stm32f405`'s `counting_exti.rs`
    /// telemetry style, per pinout.md's USER_BTN rationale.
    #[task(local = [user_btn], priority = 1)]
    async fn button_task(cx: button_task::Context) {
        loop {
            cx.local.user_btn.wait_for_falling_edge().await;
            let n = USER_BTN_EVENTS.fetch_add(1, Ordering::Relaxed) + 1;
            defmt::info!("user_btn: press {}", n);
        }
    }

    /// FFT / feature-extraction task.  Spawned once from [`init`];
    /// empty in phase-1 beyond the liveness log below.
    ///
    /// The planned hand-off is an `rtic_sync::channel::Receiver<
    /// CaptureWindow, 2>` fed by [`capture`], mirroring
    /// `feather-stm32f405`'s `SENSOR_CHANNEL_CAP = 2` precedent.
    /// Not instantiated yet: `capture` has no real producer (see
    /// its doc comment), and holding an unused `Sender` trips
    /// `#![deny(warnings)]`.  Lands together with the `rtic-sync`
    /// dependency once gate G1 runtime and G2 close.
    ///
    /// The `defmt::info!` below is a permanent liveness check, not
    /// a diagnostic scaffold: it is the on-device proof that RTIC
    /// software-task dispatch (spawn from [`init`], then the
    /// priority-2 dispatcher taking over) actually ran, distinct
    /// from `init`'s own log lines-- see the board page's bring-up
    /// section for why that distinction mattered on this chip.
    #[task(priority = 2)]
    async fn analysis(_cx: analysis::Context) {
        defmt::info!("analysis: started");
    }

    /// On-chip Ethernet + embassy-net DHCPv4 task.
    ///
    /// Builds ETH1 in RMII mode against the on-board Microchip
    /// LAN8742A PHY, then a DHCPv4 embassy-net stack, and drives both
    /// the stack runner and a link/lease monitor to completion (i.e.
    /// forever) via `join`.  The stack is `!Send`, so-- following the
    /// `feather-stm32f405` `network_task` precedent-- it is built and
    /// driven entirely inside this one task; only the `Send`
    /// peripheral tokens crossed the spawn boundary in
    /// [`EthPeripherals`].
    ///
    /// PHY identity and the 7-wire RMII + MDIO/MDC pin map come from
    /// ST's own CubeMX project for this exact board (STM32CubeN6,
    /// NUCLEO-N657X0-Q `Nx_TCP_Echo_Server.ioc`:
    /// `ETH1.MediaInterface=HAL_ETH_RMII_MODE`, `NETXDUO.LAN_8742=1`).
    /// `GenericPhy::new_auto` (inside `Ethernet::new`) probes the SMI
    /// bus and self-selects the PHY address, so none is hardcoded.
    ///
    /// No cache/MPU coherency code: the Cortex-M55 D-cache is off at
    /// reset and nothing in this crate enables it, so the ETH DMA
    /// descriptors and buffers are coherent by construction.  This
    /// intentionally departs from embassy's DK speedtest example,
    /// which keeps the D-cache on and carves a non-cacheable region
    /// with `unsafe` MPU writes-- forbidden here by
    /// `#![deny(unsafe_code)]`.  The trade is throughput headroom,
    /// irrelevant to this low-rate telemetry node.
    ///
    /// Priority 2 (same tier as [`analysis`]) so the RX ring is
    /// serviced promptly once the ETH1 interrupt (`EthIrqs`) wakes
    /// the runner; the CPU is WFI-idle between packets.  Under `net`
    /// it never returns; under `g1-spike` alone the body is a no-op
    /// and the task simply completes (the task is always present so
    /// RTIC's dispatcher glue resolves-- see [`EthPeripherals`]).
    #[task(priority = 2)]
    async fn network_task(_cx: network_task::Context, periph: EthPeripherals) {
        // g1-spike alone: nothing to drive; consume the empty bundle.
        #[cfg(not(feature = "net"))]
        let _ = periph;

        #[cfg(feature = "net")]
        {
            use embassy_futures::join::join;
            use embassy_net::{Config as NetConfig, StackResources};
            use embassy_stm32::eth::{Ethernet, PacketQueue};
            use static_cell::StaticCell;

            // Small rings suffice for DHCP + ICMP and keep RAM well
            // clear of the 128K window's top edge (memory.x).
            static PACKETS: StaticCell<PacketQueue<4, 4>> = StaticCell::new();
            let packets = PACKETS.init(PacketQueue::new());

            let mac = crate::net::mac_address();
            defmt::info!(
                "ETH1 RMII: LAN8742A, MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0],
                mac[1],
                mac[2],
                mac[3],
                mac[4],
                mac[5]
            );

            let device = Ethernet::new(
                packets,
                periph.eth1,
                EthIrqs,
                periph.ref_clk, // PF7  ETH1_RMII_REF_CLK
                periph.crs,     // PF10 ETH1_RMII_CRS_DV
                periph.rxd0,    // PF14 ETH1_RMII_RXD0
                periph.rxd1,    // PF15 ETH1_RMII_RXD1
                periph.txd0,    // PF12 ETH1_RMII_TXD0
                periph.txd1,    // PF13 ETH1_RMII_TXD1
                periph.tx_en,   // PF11 ETH1_RMII_TX_EN
                mac,
                periph.sma,
                periph.mdio, // PF4  ETH1_MDIO
                periph.mdc,  // PG11 ETH1_MDC
            );

            // Socket budget: DHCPv4(1) + margin(2).  ICMP echo replies
            // are answered by smoltcp's interface (the embassy-net
            // `auto-icmp-echo-reply` feature, on in Cargo.toml) with
            // no socket slot.
            static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
            let (stack, mut runner) = embassy_net::new(
                device,
                NetConfig::dhcpv4(Default::default()),
                RESOURCES.init(StackResources::new()),
                crate::net::stack_seed(),
            );
            defmt::info!("eth: DHCPv4 stack started, awaiting link + lease");

            // Interrupt-driven monitor: each `wait_*` yields until the
            // runner (woken by ETH1) advances the stack.  Logs link-up
            // and the acquired IPv4 lease (address/prefix + gateway) so
            // the bench can read the leased address from RTT and
            // confirm reachability with a host-side ping.
            let monitor = async {
                loop {
                    stack.wait_link_up().await;
                    defmt::info!("eth: link up");

                    stack.wait_config_up().await;
                    match stack.config_v4() {
                        Some(cfg) => {
                            defmt::info!("dhcp: lease {} gateway {}", cfg.address, cfg.gateway)
                        }
                        None => defmt::warn!("dhcp: config up but no IPv4 config"),
                    }

                    stack.wait_link_down().await;
                    defmt::warn!("eth: link down");
                }
            };

            // Both futures diverge; `join` never resolves, so this
            // statement never returns (mirrors feather's
            // `join3(...).await;`).
            join(runner.run(), monitor).await;
        }
    }

    /// RTIC idle task-- WFI sleep when no task is runnable.
    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }
}
