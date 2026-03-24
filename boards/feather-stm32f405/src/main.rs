#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

use defmt_rtt as _; // global logger
use panic_probe as _;
use rtic::app;
use rtic_monotonics::stm32::prelude::*;

mod ccmram;
mod config;
mod device_id;
mod eth;
mod network;
mod sensor;
mod time;
mod tls_buffers;

stm32_tim2_monotonic!(Mono, 1_000_000);

#[app(device = embassy_stm32, peripherals = true, dispatchers = [USART1, USART2, USART3])]
mod app {
    use super::*;
    use defmt::{info, warn};
    use embassy_futures::join::join3;
    use embassy_stm32::exti::ExtiInput;
    use embassy_stm32::gpio::{Level, Output, Pull, Speed};
    use embassy_stm32::i2c::I2c;
    use embassy_stm32::peripherals;
    use embassy_stm32::rcc::{Hse, HseMode, LsConfig, LseConfig, LseMode};
    use embassy_stm32::rtc::{Rtc, RtcConfig};
    use embassy_stm32::spi::{self, Spi};
    use embassy_stm32::time::Hertz;
    use rtic_sync::channel::{Receiver, Sender};

    use network::{manager, NetworkClient as _, SntpClient};
    use sensor::Sen66Reading;

    /// Channel capacity for sensor readings
    const SENSOR_CHANNEL_CAP: usize = 2;

    type SpiPeripheral = embassy_stm32::Peri<'static, peripherals::SPI2>;
    type PinPB13 = embassy_stm32::Peri<'static, peripherals::PB13>;
    type PinPB15 = embassy_stm32::Peri<'static, peripherals::PB15>;
    type PinPB14 = embassy_stm32::Peri<'static, peripherals::PB14>;
    type PinPC6 = embassy_stm32::Peri<'static, peripherals::PC6>;
    type PinPC3 = embassy_stm32::Peri<'static, peripherals::PC3>;
    type PinPC2 = embassy_stm32::Peri<'static, peripherals::PC2>;
    type ExtiChannel = embassy_stm32::Peri<'static, peripherals::EXTI2>;
    type DmaTx = embassy_stm32::Peri<'static, peripherals::DMA1_CH4>;
    type DmaRx = embassy_stm32::Peri<'static, peripherals::DMA1_CH3>;

    struct NetworkPeripherals {
        spi: SpiPeripheral,
        sck: PinPB13,
        mosi: PinPB15,
        miso: PinPB14,
        cs: PinPC6,
        reset: PinPC3,
        int: PinPC2,
        exti: ExtiChannel,
        dma_tx: DmaTx,
        dma_rx: DmaRx,
    }

    // RNG interrupt binding for hardware random number generator
    embassy_stm32::bind_interrupts!(struct RngIrqs {
        RNG => embassy_stm32::rng::InterruptHandler<peripherals::RNG>;
    });

    // I2C1 interrupt bindings for SEN66 sensor (async DMA mode)
    embassy_stm32::bind_interrupts!(struct I2c1Irqs {
        I2C1_EV => embassy_stm32::i2c::EventInterruptHandler<peripherals::I2C1>;
        I2C1_ER => embassy_stm32::i2c::ErrorInterruptHandler<peripherals::I2C1>;
    });

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: Output<'static>,
    }

    #[init]
    fn init(_cx: init::Context) -> (Shared, Local) {
        info!("IoT Playground starting...");

        // Adafruit Feather STM32F405: 12 MHz HSE, 32.768 kHz LSE (PC14/PC15)
        let mut config = embassy_stm32::Config::default();
        config.rcc.hse = Some(Hse {
            freq: Hertz(12_000_000),
            mode: HseMode::Oscillator,
        });

        // Configure PLL for system clock and RNG (48MHz required for RNG)
        // HSE (12 MHz) / PREDIV(6) = 2 MHz (PLL input)
        // 2 MHz * MUL(168) = 336 MHz (VCO)
        // VCO / DIVP(4) = 84 MHz (SYSCLK)
        // VCO / DIVQ(7) = 48 MHz (USB/RNG clock) ✓
        config.rcc.pll_src = embassy_stm32::rcc::PllSource::HSE;
        config.rcc.pll = Some(embassy_stm32::rcc::Pll {
            prediv: embassy_stm32::rcc::PllPreDiv::DIV6, // 12 MHz / 6 = 2 MHz
            mul: embassy_stm32::rcc::PllMul::MUL168,     // 2 MHz * 168 = 336 MHz (VCO)
            divp: Some(embassy_stm32::rcc::PllPDiv::DIV4), // 336 MHz / 4 = 84 MHz (SYSCLK)
            divq: Some(embassy_stm32::rcc::PllQDiv::DIV7), // 336 MHz / 7 = 48 MHz (RNG)
            divr: None,
        });
        config.rcc.sys = embassy_stm32::rcc::Sysclk::PLL1_P;
        config.rcc.ahb_pre = embassy_stm32::rcc::AHBPrescaler::DIV1; // 84 MHz
        config.rcc.apb1_pre = embassy_stm32::rcc::APBPrescaler::DIV2; // 42 MHz
        config.rcc.apb2_pre = embassy_stm32::rcc::APBPrescaler::DIV1; // 84 MHz

        config.rcc.ls = LsConfig {
            rtc: embassy_stm32::rcc::RtcClockSource::LSE,
            lsi: false,
            lse: Some(LseConfig {
                frequency: Hertz(32_768),
                mode: LseMode::Oscillator(embassy_stm32::rcc::LseDrive::MediumHigh),
            }),
        };

        let p = embassy_stm32::init(config);

        info!("System initialized with HSE (12MHz) and LSE (32.768kHz)");
        info!("PLL configured: SYSCLK=84MHz, PLLQ=48MHz for RNG");

        // TIM2 on APB1: timer clock = 2*APB1 when prescaler != 1
        // Default: APB1 = 42 MHz, TIM2 = 84 MHz
        let timer_clock_hz = 84_000_000;
        Mono::start(timer_clock_hz);
        info!("TIM2 monotonic timer initialized at 1 MHz");

        let rtc_config = RtcConfig::default();
        let rtc = Rtc::new(p.RTC, rtc_config);
        info!("Internal RTC initialized with LSE (32.768kHz, ±20-50ppm accuracy)");

        time::initialize_rtc(rtc);

        let led = Output::new(p.PC1, Level::High, Speed::Low);

        let net_periph = NetworkPeripherals {
            spi: p.SPI2,
            sck: p.PB13,
            mosi: p.PB15,
            miso: p.PB14,
            cs: p.PC6,
            reset: p.PC3,
            int: p.PC2,
            exti: p.EXTI2,
            dma_tx: p.DMA1_CH4,
            dma_rx: p.DMA1_CH3,
        };

        // I2C1 for SEN66 sensor: PB6 (SCL), PB7 (SDA), 400 kHz
        let mut i2c_config = embassy_stm32::i2c::Config::default();
        i2c_config.frequency = Hertz(400_000);

        let i2c = I2c::new(
            p.I2C1, p.PB6, // SCL
            p.PB7, // SDA
            I2c1Irqs, p.DMA1_CH6, // TX: DMA1 Stream 6
            p.DMA1_CH0, // RX: DMA1 Stream 0
            i2c_config,
        );
        info!("I2C1 initialized: 400 kHz, PB6/PB7 (SEN66)");

        // Sensor → network channel
        let (sensor_tx, sensor_rx) = rtic_sync::make_channel!(Sen66Reading, SENSOR_CHANNEL_CAP);

        heartbeat::spawn().ok();
        sensor_task::spawn(i2c, sensor_tx).ok();
        network_task::spawn(net_periph, p.RNG, sensor_rx).ok();

        (Shared {}, Local { led })
    }

    /// Heartbeat task
    #[task(priority = 1, local = [led])]
    async fn heartbeat(cx: heartbeat::Context) {
        info!("Heartbeat task started");
        loop {
            cx.local.led.set_high();
            Mono::delay(100.millis()).await;
            cx.local.led.set_low();
            Mono::delay(4900.millis()).await;
        }
    }

    /// Sensor task — reads SEN66 environmental sensor periodically
    ///
    /// Initializes I2C sensor driver, then reads all measurements
    /// at the configured sample interval and sends them to the
    /// network task via channel.  Sub-sensor readings are suppressed
    /// during their respective conditioning periods (see
    /// [`sensor::ConditioningState`]).
    #[task(priority = 1)]
    async fn sensor_task(
        _cx: sensor_task::Context,
        i2c: I2c<'static, embassy_stm32::mode::Async, embassy_stm32::i2c::Master>,
        mut sender: Sender<'static, Sen66Reading, SENSOR_CHANNEL_CAP>,
    ) -> ! {
        info!("Sensor task started — initializing SEN66");

        let delay = embassy_time::Delay;

        let mut sen66 = match sensor::sen66::init(delay, i2c).await {
            Ok(s) => {
                info!("SEN66 initialized, continuous measurement started");
                s
            }
            Err(e) => {
                defmt::error!("SEN66 init failed: {:?} — sensor task halted", e);
                loop {
                    Mono::delay(60_000.millis()).await;
                }
            }
        };

        // Wait for first sample to be ready
        Mono::delay((sensor::INITIAL_DELAY_SECS * 1_000).millis()).await;

        let mut state = sensor::new_conditioning_state();

        loop {
            let reading = sensor::sen66::read(&mut sen66, &mut state).await;

            if sender.try_send(reading).is_err() {
                warn!("Sensor channel full — dropping newest reading");
            }

            Mono::delay((config::SAMPLE_INTERVAL_SECS * 1_000).millis()).await;
        }
    }

    /// Network task - orchestrates network stack and protocol clients
    ///
    /// Stack is !Send and must remain within this task.
    #[task(priority = 1)]
    async fn network_task(
        _cx: network_task::Context,
        periph: NetworkPeripherals,
        rng_periph: embassy_stm32::Peri<'static, peripherals::RNG>,
        sensor_rx: Receiver<'static, Sen66Reading, SENSOR_CHANNEL_CAP>,
    ) -> ! {
        use embassy_net::{Config, StackResources};
        use static_cell::StaticCell;

        info!("Network task started");

        // Setup ethernet peripherals
        let mut spi_config = spi::Config::default();
        spi_config.frequency = Hertz(10_000_000); // 10 MHz for W5500

        let spi = Spi::new(
            periph.spi,
            periph.sck,
            periph.mosi,
            periph.miso,
            periph.dma_tx,
            periph.dma_rx,
            spi_config,
        );

        let cs = Output::new(periph.cs, Level::High, Speed::VeryHigh);
        let reset = Output::new(periph.reset, Level::High, Speed::Low);
        let int = ExtiInput::new(periph.int, periph.exti, Pull::Up);

        let eth_periph = eth::EthPeripherals {
            spi,
            cs,
            reset,
            int,
        };

        let mac_addr = [0x02, 0x00, 0x00, 0x12, 0x34, 0x56];
        let (device, w5500_runner) = eth::init_w5500(eth_periph, mac_addr).await;

        // Socket budget: DHCP(1) + DNS(1) + SNTP(1) + MQTT/TLS(1) + margin(1)
        static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
        let (stack, mut net_runner) = embassy_net::new(
            device,
            Config::dhcpv4(Default::default()),
            RESOURCES.init(StackResources::new()),
            0x1234_5678_u64,
        );
        info!("Network stack initialized with DHCP");

        let app_logic = async {
            manager::wait_for_config(&stack).await;
            run_clients(&stack, rng_periph, sensor_rx).await;
        };

        join3(w5500_runner.run(), net_runner.run(), app_logic).await;
    }

    async fn run_clients(
        stack: &embassy_net::Stack<'static>,
        rng_periph: embassy_stm32::Peri<'static, peripherals::RNG>,
        sensor_rx: Receiver<'static, Sen66Reading, SENSOR_CHANNEL_CAP>,
    ) -> ! {
        use embassy_stm32::rng::Rng;
        use static_cell::StaticCell;

        // --- SNTP time sync ---
        let mut sntp = SntpClient::new();
        info!("Initializing SNTP time synchronization with RTC (LSE)...");
        match sntp.run(stack).await {
            Ok(ts) => info!(
                "SNTP sync successful: {}.{:06} UTC (written to internal RTC)",
                ts.unix_secs, ts.micros
            ),
            Err(e) => warn!("SNTP initialization failed: {:?}", e),
        }

        // --- Hardware RNG ---
        info!("Initializing hardware RNG for TLS...");
        let mut rng = Rng::new(rng_periph, RngIrqs);
        info!("Hardware RNG initialized");

        // --- Static buffers for MQTT (RTIC StaticCell pattern) ---
        //
        // These never-freed buffers are safe in this never-returning
        // async task because:
        // 1. network_task runs at priority 1 with no resource sharing
        // 2. Buffers are exclusively owned by this task
        // 3. RTIC 2.x async tasks that never return can safely use
        //    function-local statics
        static MQTT_BUFFER: StaticCell<[u8; 2048]> = StaticCell::new();
        static TCP_RX_BUFFER: StaticCell<[u8; 4096]> = StaticCell::new();
        static TCP_TX_BUFFER: StaticCell<[u8; 4096]> = StaticCell::new();

        let mqtt_buffer = MQTT_BUFFER.init([0u8; 2048]);
        let tcp_rx_buffer = TCP_RX_BUFFER.init([0u8; 4096]);
        let tcp_tx_buffer = TCP_TX_BUFFER.init([0u8; 4096]);

        // --- Persistent MQTT connection with auto-reconnect ---
        let mqtt_config = network::MqttConfig {
            broker_host: "192.168.1.1",
            broker_port: 8883,
            keep_alive_secs: 60,
            clean_start: true,
            publish_interval_secs: config::SAMPLE_INTERVAL_SECS,
        };
        let mut mqtt_client = network::MqttClient::new(mqtt_config);

        info!(
            "Starting persistent MQTT connection ({}s publish interval)",
            config::SAMPLE_INTERVAL_SECS,
        );

        let mut buffers = network::MqttBuffers {
            mqtt: mqtt_buffer,
            tcp_rx: tcp_rx_buffer,
            tcp_tx: tcp_tx_buffer,
        };

        // Never returns — reconnects automatically on failure
        mqtt_client
            .run(stack, &mut rng, &mut buffers, sensor_rx)
            .await
    }

    /// RTIC idle task - WFI sleep mode when no tasks active
    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        info!("Idle task started - entering WFI loop");
        loop {
            cortex_m::asm::wfi();
        }
    }
}
