# ARS Toolhead Sensor-- Audio Board Spike

This is a research spike into audio-output hardware for the ARS
toolhead sensor node, dated 2026-07-19.  Status: recommendation
pending EE review and gate G3 bench data (pinout.md Spike Gates,
G3).  This document does not change the pin map of record-- see
Pin-Map Impact below for the delta that applies only if G3
overturns the provisional PWM path.

## Budget Rules

Two budget classes were surveyed, keyed to whether the existing
Adafruit MAX9744 amplifier stays in the signal chain:

- **DAC-only** (MAX9744 stays): under $30.
- **DAC+amp** (MAX9744 retired): under $50.

US availability only (in stock at a US-shippable vendor).  Every
price below carries a source URL and an as-of date; every spec
carries a datasheet or vendor-page citation.  Facts that could not
be sourced are marked UNVERIFIED, never presented as fact.

## Recommendation

### Primary: Adafruit PCM5102 I2S DAC breakout (PID 6250)

MAX9744 stays.  $4.95, <https://www.adafruit.com/product/6250>, as
of 2026-07-19.  This is the DAC-only class (under $30; actual spend
$4.95).  It has the best sourced excitation fidelity in either
survey-- 112 dB SNR, -93 dB THD+N at -1 dBFS, DC-coupled
ground-centered output (Adafruit product page)-- exactly what
stepped/swept-sine and MLS source quality demands.  It is
MCLK-less: an internal BCK-derived PLL means SAI1 drives only
BCK/WS/DIN with zero register init, no I2C, and no configuration
sequence-- the cheapest possible RTIC/Embassy integration on the
N657.  Digital I/O is native 3.3 V, and the board exposes a
hardware MU mute strap.  Keeping the MAX9744 preserves the entire
existing amp integration (I2C1 volume at 0x4B on PH9/PC1, PD0 mute,
external PVDD plan) and its 20 W / 4 ohm headroom, which no sub-$50
replacement matches with sourced clean-output numbers.  At $4.95
the two-bench reality is trivially covered on the DAC side ($9.90
for both rigs); the open second-rig item is a second MAX9744, not
the DAC.  This board is purchased now but adopted only if spike
gate G3 overturns the provisional TIM1-PWM path-- at $4.95 it is
cheap G3 insurance either way.

### Fallback: Adafruit MAX98357A I2S 3W Class-D amp breakout (PID 3006)

MAX9744 goes.  $5.95, <https://www.adafruit.com/product/3006>, as
of 2026-07-19.  This is the DAC+amp class (under $50; actual spend
$5.95).  Engaged if the MAX9744 chain fails at bench
(level-matching, noise, or hardware fault) or a second MAX9744
proves unsourceable for the remote rig.  One board collapses
DAC+amp, needs only BCK/LRCLK/DATA (no MCLK, no I2C, no init), has
a GPIO shutdown pin for mute, is US stocked, and is cheap enough to
buy per rig.  Its costs are sourced and real: 3.2 W into 4 ohm is
specified only at 10% THD+N (near clipping), so clean headroom is
marginal and its measurement-grade fidelity is unproven-- it must
pass a bench THD/idle-noise check before any measurement use.

### Rejected outright

- **UDA1334A** (PID 3678)-- fidelity numerically unsourced, so it
  is unrankable against the PCM510x family.
- **Aus3D TAS5822M** and **Sonocotta TAS5825M**-- both fail the US
  availability limit despite good sourced THD figures.
- **eBay MA12070P clone**-- price/stock unverified, undocumented
  PCB, and the only candidate demanding an MCLK-equivalent
  reference clock from SAI1.

## Decision Matrix

Columns: F = fidelity, P = power, I = integration, M = mute/pop,
B = budget, S = supply, A = availability, each scored out of 5
(UNVR = UNVERIFIED, not scored).  Full citations are in Candidate
Summaries below.

| Option | Class | Price | F | P | I | M | B | S | A | Verdict |
|--------|-------|-------|---|---|---|---|---|---|---|---------|
| Adafruit PCM5102 (PID 6250) | DAC-only | $4.95 | 5 | 5 | 5 | 4 | 5 | 5 | 5 | **PRIMARY** |
| Adafruit MAX98357A (PID 3006) | DAC+amp | $5.95 | 2 | 2 | 5 | 4 | 5 | 5 | 5 | **FALLBACK** |
| Adafruit PCM5122 (PID 6421) | DAC-only | $7.50 | 5 | 5 | 4 | 4 | 5 | 5 | 5 | ALTERNATE |
| Adafruit PCM5100 (PID 6251) | DAC-only | $3.95 | 3 | 5 | 5 | 4 | 5 | 5 | 5 | REJECT |
| Adafruit UDA1334A (PID 3678) | DAC-only | $6.95 | UNVR | 5 | 4 | 4 | 5 | 5 | 3 | REJECT |
| Aus3D TAS5822M breakout | DAC+amp | ~$21-22 | 4 | UNVR | 3 | 2 | 4 | 3 | 1 | REJECT |
| Sonocotta Louder Hat Plus | DAC+amp | $25.00 | 4 | 5 | 2 | 2 | 5 | 3 | 1 | REJECT |
| Generic "PL-DD-160W" MA12070P | DAC+amp | ~$42+ship | 3 | 5 | 1 | 1 | 3 | 3 | 1 | REJECT |

## Candidate Summaries

### DAC-only (MAX9744 stays)

#### Adafruit PCM5102 I2S DAC breakout (PID 6250)-- PRIMARY

TI PCM5102 chip.  $4.95,
<https://www.adafruit.com/product/6250>, as of 2026-07-19, US
stock.  112 dB SNR, -93 dB THD+N at -1 dBFS, DC-coupled
ground-centered 2.1 Vrms full-scale output, 8-384 kHz / 16-32 bit,
MCLK-less BCK-derived internal PLL, 3.3 V native LVCMOS I2S input,
hardware MU mute strap.  Source: Adafruit product page;
<https://learn.adafruit.com/adafruit-pcm510x-i2s-dac/pinouts>.  Note:
the older PID 3492 board of the same name is discontinued and
reassigned to an unrelated PDM microphone breakout-- do not follow
tutorials citing PID 3492 (confirmed 2026-07-19,
<https://www.adafruit.com/product/3492>).

#### Adafruit PCM5122 I2S DAC breakout (PID 6421)-- ALTERNATE

TI PCM5122 chip.  $7.50,
<https://www.adafruit.com/product/6421>, as of 2026-07-19, US
stock (34 units).  Same sourced 112 dB SNR / -93 dB THD+N family
figures as the PCM5102, DC-coupled 2.1 Vrms output, up to 384 kHz.
Digital inputs are 1.8 V-or-3.3 V failsafe LVCMOS.  Same hardware
mute pin and zero-config default, but also supports optional I2C
or SPI register control (volume/mute) if that becomes a
requirement later.  Source: Adafruit product page.  Pick this over
the PCM5102 only if DAC-side I2C volume/mute control becomes a
requirement; confirm the MODE strap defaults to hardware mode
before wiring only the four I2S lines.

#### Adafruit PCM5100 I2S DAC breakout (PID 6251)-- REJECT

TI PCM5100 chip.  $3.95,
<https://www.adafruit.com/product/6251>, as of 2026-07-19, US
stock.  100 dB SNR, -90 dB THD+N at -1 dBFS-- lowest fidelity of
the PCM510x family.  REJECT: saves $1 over the PCM5102 while
giving up 12 dB SNR and 3 dB THD+N on a measurement-grade source.

#### Adafruit UDA1334A decoder breakout (PID 3678)-- REJECT

NXP UDA1334A chip.  $6.95,
<https://www.adafruit.com/product/3678>, as of 2026-07-19, US
stock (59 units).  MCLK-less, 3.3-5 V digital tolerance, explicit
hardware MUTE pin on the header.  REJECT: THD+N/SNR are not
numerically sourced-- the Adafruit page gives only a qualitative
sweep claim-- so fidelity cannot be ranked, and it costs more than
the fully-sourced PCM5102.

#### Other rejected DAC-only candidates

- Generic "GY-PCM5102" clones (HiLetgo, Comimark, QCCAN,
  PAMEENCOS, and similar Amazon/AliExpress listings)-- same
  PCM5102A chip as the baseline being beaten, inconsistent seller
  documentation, no confirmed mute-pin exposure.
- DIYINHK ES9023 DAC board (diyinhk.com)-- confirmed end-of-life /
  out of stock at the vendor as of 2026-07-19.
- Generic Amazon/AliExpress ES9023 decoder boards-- listing pages
  returned HTTP 403/500 on repeated fetch attempts; current
  price/stock and datasheet-grade specs could not be verified.
- Generic Amazon ES9038Q2M decoder boards-- same unfetchable-
  listing problem; several bundle an on-board headphone amp or
  optical/coax input selection, breaking the DAC-only requirement;
  authentic dual-mono implementations exceed the $30 cap.
- SparkFun and Adafruit MAX98357A boards, evaluated here as
  DAC-only candidates-- these are DAC+amp combos that would
  duplicate the already-owned MAX9744 rather than feed its line
  input (see the DAC+amp group, where the Adafruit board is
  evaluated on its own merits as an amp replacement).
- "Beyond ES9023 PCM1794" branded combo-marketing clone boards
  (diymore/Aideepen-style)-- same undocumented-BOM problem as the
  GY-PCM5102 baseline.

### DAC+amp (MAX9744 retired)

#### Adafruit MAX98357A I2S 3W Class-D amp breakout (PID 3006)-- FALLBACK

Maxim/Analog Devices MAX98357A chip.  $5.95,
<https://www.adafruit.com/product/3006>, as of 2026-07-19, US
stock (also DigiKey, Micro Center).  3.2 W into 4 ohm at 10%
THD+N, 1.8 W into 8 ohm at 10% THD+N, 2.7-5.5 V supply, filterless
spread-spectrum Class-D output, PSRR 77 dB typical at 1 kHz.
Sources: Adafruit product page;
<https://cdn-shop.adafruit.com/product-files/3006/MAX98357A-MAX98357B.pdf>.
No MCLK required (BCLK + LRCLK + DATA only), 3.3 V-compatible
digital inputs, no I2C-- gain is a hardware strap and SD is a
hardware shutdown/mute pin.  Mono only; the 10% THD+N figure is
near-clipping, so clean output for spectral measurement is likely
well under 3.2 W; switching frequency is UNVERIFIED in the sources
reviewed.

#### Other rejected DAC+amp candidates

- Aus3D TAS5822M Breakout Board (TI TAS5822M)-- $33 AUD (approx.
  $21-22 USD),
  <https://aus3d.com.au/products/tas5822m-breakout-board>, as of
  2026-07-19.  REJECT: ships from Australia only, 10-21 day
  standard post, fails the US-availability limit despite the best
  sourced THD+N (<=0.06% at 1 W / 1 kHz / 24 V, TI datasheet) of
  the smart-amp candidates.
- Sonocotta Louder Raspberry Hat Plus (TI TAS5825M)-- $25.00,
  <https://www.tindie.com/products/sonocotta/louder-raspberry-hat-plus/>,
  as of 2026-07-19.  REJECT: ships from Poland with standard US
  shipping on tariff hold (UPS surcharge only), fails the
  US-availability limit; also carries the largest bring-up burden
  (on-board DSP path must be proven bypassed/flat).
- Generic "PL-DD-160W" MA12070P board (Infineon MA12070P, eBay
  Shenzhen sellers)-- approx. EUR 37.85 (~$42 USD) plus $9-30
  shipping, <https://www.ebay.com/itm/353688009579>, as of
  2026-07-19 (price approximate, live listing fetch timed out).
  REJECT: unverified price/provenance, undocumented clone PCB, and
  the only candidate requiring an MCLK-equivalent reference clock
  from SAI1.
- HiFiBerry Amp2 (TAS5756M)-- $55.95,
  <https://www.pishop.us/product/hifiberry-amp2/>, as of
  2026-07-19.  REJECT: exceeds the $50 budget.
- JustBoom Amp HAT (TAS5756M)-- $64.90 at Seeed Studio,
  <https://www.seeedstudio.com/JustBoom-Amp-HAT-for-the-Raspberry-Pi-p-2846.html>,
  as of 2026-07-19; flagged discontinued at The Pi Hut,
  <https://thepihut.com/products/justboom-amp-hat>, as of
  2026-07-19.  REJECT: exceeds budget and carries discontinuation
  risk.
- TI/vendor official evaluation boards (TAS5806MDEVM, TAS5825MEVM,
  MA12070P EVAL boards)-- EVM-tier pricing (hundreds of dollars),
  no unit under the $50 cap; e.g. TAS5806MDEVM,
  <https://www.digikey.com/en/products/detail/texas-instruments/TAS5806MDEVM/10434493>.
  REJECT: no evaluation board found under the $50 cap for these
  chips.
- Sonocotta Loud-ESP32-Plus / Louder-ESP32(-Plus)-- REJECT: wrong
  architecture, each integrates its own on-board ESP32 host
  instead of accepting an external I2S feed from an independent
  SAI1 master.
- tonyp7 TAS5806M-Audio-Amplifier (open-source design,
  <https://github.com/tonyp7/TAS5806M-Audio-Amplifier>)-- REJECT:
  MIT-licensed gerbers/BOM only, no assembled unit for sale found
  at any vendor.
- Generic AliExpress/eBay "I2S TPA3116" boards-- REJECT: bolt a
  separate DAC ahead of an analog Class-D stage, reintroducing an
  extra DAC-plus-analog-gain stage that a direct I2S-to-Class-D
  replacement is meant to avoid.

## Pin-Map Impact

Delta only-- the pin map of record in pinout.md is unchanged
until gate G3 overturns the provisional TIM1-PWM path.  If G3
overturns it, both the primary and fallback options apply the
same SAI1 change; the primary additionally keeps the amp I2C rows
unchanged, while the fallback retires them.

Applies to [pinout.md](./pinout.md).

**Primary path (PCM5102, MAX9744 retained):**

1. Row AUDIO_PWM (PE9, TIM1_CH1 AF1, GPDMA1 tim1_upd_dma REQSEL
   18) becomes obsolete, along with the external RC low-pass note
   and the "PE8 stays free / CH1N unused" note.
2. Add exactly three SAI1 rows-- SAI1 bit clock (to DAC BCK), SAI1
   frame sync (to DAC WSEL), SAI1 data out (to DAC DIN).  No MCLK
   row is needed: the PCM5102 (and every recommended board) derives
   its clocks from BCK via an internal PLL, so the SAI1 path's
   pin cost is three pins, not four.  Concrete N657 pin/AF
   assignments for SAI1 are not yet in the map and were not
   surveyed here-- selecting them from the DS14791 AF tables and
   clearing them against the Nucleo on-board-function tables
   (Ethernet, USB/PD, camera, VCP in UM3417) is the G3 revision
   work item.
3. The `sweep_engine` GPDMA channel re-targets from
   tim1_upd_dma REQSEL 18 to the SAI1 TX request (REQSEL number to
   be pulled from RM0486 Table 98).
4. Rows AMP_I2C_SCL (PH9), AMP_I2C_SDA (PC1), and AMP_MUTE_N (PD0)
   are unchanged-- the MAX9744 keeps I2C volume at 0x4B and mute on
   PD0; the PCM5102 runs in zero-config hardware mode and adds
   nothing to I2C1.
5. Optional: the DAC's MU strap could take a spare GPIO as a
   second mute layer, or be hardwired at zero pin cost-- EE review
   decides.
6. Decision 1 and the AUDIO_PWM rationale section get rewritten;
   gate G3 closes.

**Fallback path (MAX98357A, MAX9744 retired):** same three SAI1
rows and DMA re-target as above, plus: rows AMP_I2C_SCL (PH9) and
AMP_I2C_SDA (PC1) become obsolete-- the MAX98357A has no I2C, so
the private I2C1 bus is freed entirely; PD0 repurposes from
MAX9744 MUTE_INV (drive low to mute through Q4) to the MAX98357A
SD pin (hardware shutdown, which also carries a mono channel-select
strap function per Adafruit), and the polarity/semantics note in
that row must change; Decision 5's external 4.5-14 V PVDD supply
plan is replaced by the MAX98357A's single 2.7-5.5 V rail.

**Alternate only (PCM5122):** if it were later switched out of
hardware mode into I2C control, it would join I2C1 on PH9/PC1 and
needs an address-conflict check against the MAX9744 at 0x4B.

## Open Questions

- Second-rig amplifier sourcing: is the Adafruit MAX9744 board
  still purchasable (price, stock) for Engineer S's remote rig?
  Neither survey covered it.  If unsourceable, rig 2 must run the
  MAX98357A fallback, making the two rigs' analog chains differ--
  decide whether that comparability loss is acceptable before
  ordering.
- Level matching DAC-to-amp: PCM510x full-scale is 2.1 Vrms
  (~5.94 Vpp) versus the MAX9744's documented ~3 Vpp input ceiling
  (MAX9744.pdf pp.11-12 via pinout.md).  This is an inference from
  two separately sourced numbers, not a measurement.  EE to specify
  a resistive pad at JP2 LEFTIN or a digital full-scale ceiling
  policy, and to confirm which preserves source SNR better.
- SAI1 pin selection on the N657: concrete pins/AFs for SAI1
  SCK/FS/SD were never surveyed.  Pull DS14791 AF tables and clear
  against UM3417 Nucleo on-board functions (Ethernet, USB-PD,
  camera, VCP) before the G3 map revision; also pull the SAI1 TX
  GPDMA REQSEL from RM0486 Table 98.
- embassy-stm32 SAI driver coverage for stm32n657x0 under RTIC 2
  (extension of existing gate G1): SAI support is unverified just
  as ADC/TIM/I2C were; confirm or plan the PAC-level fallback
  before committing firmware effort.
- MAX98357A fallback qualification: clean-output (e.g. 1% THD+N)
  power into 4 ohm and switching frequency are UNVERIFIED-- pull
  the full ADI datasheet, then bench-measure THD+N, idle noise
  floor, and SD-pin mute/click behavior before any measurement-
  window use.
- Board-level idle tones and mute/pop for the PCM5102: the MU
  strap is sourced but pop behavior on mute/unmute transitions and
  idle-tone floor are chip/board bench items-- fold into the
  existing G3 bench SNR/THD measurement.
- Existing pinout.md open question remains live because the
  MAX9744 stays: which rail pulls up the amp header MUTE_INV net
  (R15), and whether open-drain PD0 plus power-up pop behavior
  justifies wiring SHDN after all.
- PCM5122 alternate only: confirm the MODE strap genuinely
  defaults to hardware (zero-config) mode before wiring just the
  four I2S lines, and check its I2C address range (0x4C-0x4F class
  parts) against the MAX9744 at 0x4B if register control is ever
  enabled.
- UDA1334A only if reconsidered: numeric THD+N/SNR from the NXP
  datasheet and NXP lifecycle status (active vs NRND)-- both
  unsourced in the survey.

## Purchase List

- Rig 1 (primary): 1x Adafruit PCM5102 I2S DAC breakout, PID
  6250-- $4.95 (<https://www.adafruit.com/product/6250>, as of
  2026-07-19).  MAX9744 already owned; rig-1 new spend: $4.95.
- Rig 2 (remote, Engineer S): 1x Adafruit PCM5102 I2S DAC
  breakout, PID 6250-- $4.95 (same source/date).  Two-rig DAC
  subtotal: $9.90.
- Fallback insurance (recommended now, one unit): 1x Adafruit
  MAX98357A I2S 3W Class-D amp breakout, PID 3006-- $5.95
  (<https://www.adafruit.com/product/3006>, as of 2026-07-19).
  Serves as the rig-2 amp contingency if a second MAX9744 cannot
  be sourced.
- Optional second fallback unit (only if both rigs must run the
  fallback chain): 1x MAX98357A, PID 3006-- $5.95 (same
  source/date).
- Rig 2 amplifier (primary chain): 1x Adafruit MAX9744-- price and
  availability UNVERIFIED, not surveyed; resolve the open question
  above before ordering.  Not counted in the totals below.
- Passive pad resistors for DAC-to-MAX9744 level matching-- values
  pending EE review (see Open Questions); jellybean parts, expected
  from existing bench stock, no sourced price.

### Totals (boards with sourced prices, excluding shipping)

| Scenario | Total |
|----------|-------|
| One rig, primary only | $4.95 |
| One rig, primary + fallback insurance | $10.90 |
| Two rigs, primary DACs only | $9.90 |
| Worst case: 2x PCM5102 + 2x MAX98357A | $21.80 |

All figures sit well under both budget caps ($30 DAC-only, $50
DAC+amp).
