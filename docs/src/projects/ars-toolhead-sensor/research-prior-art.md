# ARS Toolhead Sensor: Prior Art and Design Research

Synthesis of four research passes covering excitation design, FDM
acoustic-monitoring prior art, resonance feature extraction, and
CNN/IP landscape.  Two of the four passes ran without live literature
search access and drew on canonical signal-processing references from
training knowledge; those citations are flagged and should be
re-verified before formal use (papers, patent filings).  Web-sourced
citations from the other two passes are linked inline.

## 1. Executive Summary

The proposed device-- an active exciter plus MEMS microphone clamped
to a Bambu Lab H2C toolhead, measuring resonance shifts to infer
filament state-- appears to be a **novel application of an
established technique**.  Acoustic Resonance Spectroscopy itself is
decades-old public art (Dipen Sinha, Los Alamos, ~1989; commercial
use in pharmaceutical tablet ID, ordnance inspection, and fill
verification)[^ars-wiki], but no published FDM/FFF work was found
that actively excites the toolhead/filament path and measures the
response.  Existing 3D-printing acoustic monitoring is passive:
ultrasonic acoustic-emission (AE) burst detection at 100 kHz-1 MHz,
or room-microphone MFCC/CNN classifiers for audible clog signatures.
Neither provides an accuracy baseline for this application.

Key conclusions, at a glance:

- **Excitation:** LFSR-generated MLS as the primary steady-state
  excitation; Schroeder-phase multitone for high-rate narrowband
  inference; Farina log-sweep (ESS) reserved for quiescent
  characterization passes.  All three research angles independently
  concluded the "white noise beat swept sine on old PC hardware"
  observation is most likely a measurement-chain artifact, not
  physics.
- **Band/rate:** 48 kHz is comfortable but not load-bearing for the
  1.2/3.4 kHz provisional bands; the bands themselves are the risky
  assumption (fan blade-pass frequency plausibly overlaps both) and
  must be bench-verified before being locked in.
- **Features:** peak frequency (3-bin Goertzel + parabolic
  interpolation), Q/damping (half-power bandwidth or ring-down), and
  *relative* amplitude ratios-- never absolute amplitude, which is
  the coupling-corrupted feature across all comparable studies.
- **CNN:** log-mel spectrogram + small CNN, reusing ST's validated
  Neural-ART reference pipeline for the STM32N6 (16 kHz, 64 mel x 96
  cols)[^st-n6-audio].  Store raw waveforms plus rich metadata now;
  do not lock a feature representation at capture time.
- **IP:** no blocking patent surfaced, but Stratasys actively
  litigates extruder-sensing patents[^stratasys-suit]; a
  professional freedom-to-operate search is advised before any
  commercial claims.

## 2. Excitation-Family Recommendation

### Ranking

Ranked for this device (peak-limited small exciter, fixed-point DSP,
shared DAC/ADC clock on one MCU):

1. **Binary MLS (LFSR-generated)-- primary steady-state mode.**
   Crest factor ~0 dB (every sample at full amplitude) maximizes
   delivered RMS power from a peak-limited exciter (Schroeder 1970;
   Rife & Vanderkooy 1989).  FRF recovery is a circular
   cross-correlation against the known +/-1 sequence (fast Hadamard
   transform), cheap in fixed point, with ~sqrt(N) noise suppression
   from synchronous averaging.  Known weakness: exciter
   nonlinearity folds into the recovered impulse response as an
   irreducible noise floor rather than separating cleanly
   (Vanderkooy 1994)-- characterize the exciter's distortion level
   before committing to MLS alone.
2. **Schroeder-phase multitone (NCO-generated)-- high-rate
   narrowband mode.**  Concentrates all energy at the tracked
   resonance bins plus flanking sentinels, maximizing per-bin SNR;
   with integer-period records it gives leakage-free per-bin FRF and
   distortion estimates (Pintelon & Schoukens 2012).  Pairs directly
   with Goertzel detection at exactly those bins.  Do not rely on
   it alone while 1.2/3.4 kHz remain unconfirmed.
3. **Exponential sine sweep (Farina ESS)-- characterization mode.**
   Best-in-class isolation of the linear response: harmonic
   distortion products separate in time and can be windowed out
   (Farina 2000).  Costly and drift-sensitive over a slow sweep, so
   reserve for quiescent events (boot calibration, filament change).
4. **White/pink random noise-- diagnostic only.**  Worst crest
   factor (~9-12 dB after clipping), needs Welch-averaged H1/H2
   estimation with coherence (Bendat & Piersol 2010).  Its one
   virtue-- robustness to drift and distortion via averaging-- is
   better obtained from repeated MLS or burst-random.

### Resolving the "noise beat sweep on old PC hardware" observation

Theory says a swept sine should not lose to white noise for linear
FRF measurement.  Three independent, mundane explanations fit the
old-PC setup, and all three passes converged on them:

- **DAC/ADC clock desynchronization.**  Consumer sound cards
  commonly lacked a shared sample clock; sweep deconvolution needs
  tight time alignment, while noise-based Welch averaging is
  indifferent to it.  This is the single most likely culprit.
- **Time variance during the sweep.**  A single-pass sweep probes
  each band at a different absolute time; any thermal or mechanical
  drift biases the FRF.  Broadband noise probes all bands
  quasi-simultaneously (Bendat & Piersol 2010, ch. 11).
- **Exciter clipping/nonlinearity or sweep-too-short vs. resonance
  decay time.**  Averaged noise estimation with coherence weighting
  is more forgiving of a distorting exciter than a single sweep
  pass.

**Verdict:** treat the bench observation as a probable hardware
artifact, not evidence against sweeps.  **Action:** re-run a
controlled MLS vs. ESS vs. noise A/B on the STM32N6 (shared,
synchronous sample clock), checking exciter clipping and sweep
duration against measured decay time, and log raw broadband
responses during bring-up so the anomaly can be root-caused offline.
Confidence that the artifact explanation is correct: moderate-high;
it remains unproven until the A/B runs.

## 3. Frequency Band and Sample-Rate Assessment

**48 kHz: acceptable, not binding.**  Nyquist at 24 kHz gives >6x
margin over 3.4 kHz.  The real sizing constraints are elsewhere:

- **MLS length vs. update rate.**  At 48 kHz, a 1023-tap MLS gives
  ~46.9 Hz bin resolution and a ~21.3 ms period; longer sequences
  trade latency for resolution.  Size sequence order against the
  required filament-state update rate, not against Nyquist.
- **Anti-aliasing is a hard requirement.**  Stepper driver PWM
  chopping commonly sits at 20-50 kHz[^ti-stepper]; any chopper
  below 24 kHz aliases directly into a 48 kHz capture and can
  masquerade as resonance content.  Verify the H2C's driver chopper
  frequency or add analog anti-alias filtering ahead of the ADC.
- **The CNN branch does not need 48 kHz.**  ST's validated STM32N6
  audio pipeline runs at 16 kHz[^st-n6-audio], which is ample for
  1.2/3.4 kHz content.  Capture at the native DSP rate; downsample
  for the CNN branch.
- **48 kHz cannot cover ultrasonic AE.**  Passive-AE clog/fault
  literature operates at 100 kHz-1 MHz; if AE-style detection is
  ever wanted, budget a separate channel.

**1.2/3.4 kHz: flag, do not confirm.**  Three concerns:

- **Fan blade-pass overlap (serious).**  BPF = blades x RPM/60 for
  typical 7-9 blade hot-end blowers at 8k-19k RPM computes to
  roughly 1-2.5 kHz fundamental and 2-5 kHz second
  harmonic[^bpf]-- plausibly overlapping *both* provisional bands.
  This is a calculation, not a measurement on the actual H2C fan.
  Bench-sweep with the fan at multiple PWM duties and steppers
  active before locking center frequencies.
- **A passive FDM vibration study found dominant structural
  resonances at 180-370 Hz**[^sensor-pos], well below the assumed
  bands.  Different mechanism (whole-gantry vibration vs. a driven
  local resonance), so not a contradiction, but a reminder that the
  bands are assumptions until measured.
- **Temperature drift.**  Generic resonator TCFs of tens of ppm/C
  across a 200-300 C hot-end swing argue for a temperature-indexed
  or differential baseline, not one static calibrated frequency.
  UNCONFIRMED for this specific structure; needs bench measurement.

## 4. DSP Feature Set and Record-Schema Implications

RUS/material-ID literature treats **peak frequency** and
**damping/attenuation** as the two robust discriminators; amplitude
is explicitly the noisy, coupling-dependent one[^rus].  Per capture,
per tracked mode:

- **Peak frequency:** 3 Goertzel bins straddling the expected peak
  (ST DT0089 fixed-point Goertzel[^dt0089]) + parabolic
  interpolation for sub-bin resolution[^quadinterp].
- **Q/damping**, by one of two methods, both Goertzel-friendly:
  half-power bandwidth (Q = f0/BW_3dB, extra offset bins) or
  ring-down decay (Q = pi * f0 * tau from the envelope after
  excitation stops)[^ringdown].  Ring-down pairs naturally with
  burst/MLS excitation but requires a silent gap of several decay
  time-constants in the burst schedule.
- **Relative amplitude only:** the ratio between the two modes, or
  vs. a same-session unloaded baseline.  Two independent FDM
  studies found amplitude features first-order corrupted by
  mount/coupling variation[^sensor-pos][^gatech-ae].
- **Coherence (gamma^2) per bin** (Bendat & Piersol estimators):
  low coherence flags noise contamination (fan, stepper) or
  nonlinear corruption; use it to gate samples out of CNN training
  data rather than silently poisoning labels.
- **Time-domain RMS/energy envelope:** AE literature found
  time-domain energy features the most state-sensitive for some
  transitions[^gatech-ae]; the real-time path can ignore them, but
  the training record should not discard them.

Record-schema fields implied (see also section 8): raw waveform,
excitation descriptor (family, MLS order/seed, tone set, amplitude),
per-mode {peak_freq, Q, method}, amplitude ratios, per-bin
coherence, RMS envelope, hot-end/cold-end temperatures, fan PWM,
step rate/motion state, clamp/mount ID and preload, filament labels
(present/absent, material, color), session baseline reference.

## 5. CNN Input-Representation Guidance

- **Default: log-mel spectrogram + small CNN.**  Log-mel
  consistently beats STFT and MFCC inputs in comparative studies
  (97.1% vs. 95.2% vs. 93.8% in one[^mel-cmp]; 96.7% vs. 95.8% vs.
  85.2% in another[^gunshot]).
- **Reuse ST's silicon-validated pipeline shape** for the exact
  target chip: 16 kHz, 400-sample window / 160 hop, 64 mel bins x
  96 frames, 8-bit-quantized YAMNet-1024 backbone, 144 KiB
  activations[^st-n6-audio].  Do not hand-roll spectrogram math ST
  already ships.
- **Budget spectrogram compute:** TinyML sources report on-device
  spectrogram extraction can nearly double pipeline latency vs.
  inference alone[^tinychirp]; less of a concern with the
  Neural-ART NPU but still a real budget line.
- **Keep a handcrafted-feature fallback:** Goertzel magnitudes and
  band-energy ratios into a tiny FC/1D-CNN, cheap because the DSP
  path computes them anyway; handcrafted+deep ensembles beat either
  alone in general audio ML.
- **Do not lock the representation at capture time.**  The
  FDM-acoustics field has no converged standard[^am-survey]; store
  raw waveforms + metadata so features can be recomputed.
- **Distrust published accuracy numbers.**  The 80-95% figures in
  FDM audio/AE classifiers come from single-printer, single-material
  lab datasets[^am-survey]; plan validation across printers,
  materials, colors, and ambient conditions.
- **Calibrate out the exciter/mic transfer function** before
  treating spectral features as plant resonance, or the CNN learns
  the excitation chain instead of the filament (classic FRF
  confound; Farina 2000, Bendat & Piersol 2010).

## 6. Practical Pitfalls

- **Clamp coupling is the dominant error source.**  A
  nozzle-mounted sensor showed 71% higher fault-detection
  sensitivity than frame/bed mounts, and models did not generalize
  across geometries[^sensor-pos]; industry mounting guidance
  confirms loose/partial contact lowers apparent resonance and adds
  spurious peaks[^wilcoxon].  Design for fixed, repeatable preload
  (registration features, controlled torque); record clamp ID and
  preload; prefer within-session loaded-vs-unloaded deltas over
  cross-session absolute references.
- **Temperature drift:** index the unloaded baseline by the hot-end
  thermistor, or add a differential reference channel (sect. 3).
- **Printer noise is structured, not white.**  Running-printer
  acoustics are information-dense across the audible
  band[^quietprint]; fan BPF and stepper harmonics move with
  controller state.  Prefer filters referenced to known controller
  state (fan PWM, step rate) over fixed notches.
- **Aliasing from stepper choppers** below 24 kHz (sect. 3).
- **MLS + background noise + exciter nonlinearity:** MLS
  correlation has no leakage protection against nonstationary
  background and folds distortion into the noise floor (Vanderkooy
  1994); do not run MLS inference during heavy motion without
  accounting for it, and measure exciter distortion early.
- **Ring-down timing:** reserve silent gaps of several tau after
  each burst or the ring-down Q method silently degrades.
- **Material dependence:** PLA vs. ABS show different
  temperature-dependent damping near Tg[^dma] (abstract-level only,
  paywalled)-- expect per-material feature shifts.

## 7. Novelty and IP Landscape

- **Method is old; application appears new.**  ARS as a technique
  is public art from ~1989[^ars-wiki] with a commercial lineage
  (Adaptive Resonance Technology and successors: ordnance,
  pharma tablet verification, fill level).  No FDM/FFF publication
  using active excitation-response resonance sensing for filament
  state was found by any pass.
- **Closest published analogs are passive:** ultrasonic AE + HSMM
  machine-condition monitoring (Wu et al., ~2016-17, citation
  details unverified)[^gatech-ae], room-mic MFCC/CNN clog
  detection, and two 2026 preprints on CNN acoustic fault
  detection[^cnn-preprint] and multimodal fusion[^fusion-preprint]
  (both read at abstract depth only; PDFs resisted extraction--
  manual read recommended).
- **No blocking patent surfaced.**  The nearest 3D-printer
  filament-sensing patent found (TW201725108A) is optical/
  encoder-based and reads as distinguishable[^tw-patent].  Adjacent
  ARS patents (US10,502,793 battery NARS; EP3488505B1 optical
  pump-probe) are different domains/modalities.
- **Real caution: Stratasys.**  US9,168,698 and US10,556,381
  (extruder contact-force sensing) are being actively asserted
  against Bambu Lab[^stratasys-suit].  Different modality, but it
  shows enforced IP around extruder-state sensing broadly.
- **Recommendations:** frame any claims around the *application*
  (toolhead-mounted, filament-path resonance sensing in FDM), not
  the generic ARS method; run a professional freedom-to-operate
  search (including the ART lineage and Bambu/Prusa filings) before
  commercial claims; a scoped web search is not a substitute.
- **No published accuracy baseline exists** for acoustic
  filament-presence sensing; deployed runout sensors are mechanical
  or optical.  Novelty cuts both ways: extra bench-characterization
  time is required because no failure-mode catalog exists to crib.

## 8. Design Inputs for Task #18 (ars-synth) and the Record Schema

Generator requirements (ars-synth):

- MLS generator: parameterized LFSR (order n for length 2^n - 1,
  seed, taps); golden vectors for at least orders 9-11 (511, 1023,
  2047 taps) at the native sample rate.
- NCO multitone generator: Schroeder-phase, arbitrary bin set;
  golden case = {1.2 kHz, 3.4 kHz} + flanking sentinel bins, with
  per-bin phase table recorded.
- ESS generator: Farina exponential sweep with configurable
  f1/f2/duration plus the matched inverse filter as a golden
  artifact.
- Burst scheduling: every burst mode carries an explicit post-burst
  silent gap parameter (>= several expected decay time-constants)
  to support ring-down Q.
- Golden analysis vectors: known synthetic 2-pole resonators (f0,
  Q) driven by each excitation family, with expected Goertzel
  3-bin magnitudes, parabolic-interpolated f0, half-power Q,
  ring-down tau, and per-bin coherence-- RED/GREEN targets for the
  fixed-point DSP path.
- Noise models for robustness tests: fan BPF tone + harmonics
  (sweepable 1-2.5 kHz fundamental), stepper harmonic comb, and an
  aliased chopper tone case.

Record-schema fields (one capture record):

- Raw waveform at native sample rate (do not store features only).
- Excitation descriptor: family enum {MLS, multitone, ESS, noise},
  order/seed/taps or bin set/phases or f1/f2/T, drive amplitude,
  burst/gap timing.
- Per-mode features: interpolated peak frequency, Q, Q-method enum,
  amplitude ratio(s) vs. sibling mode and session baseline.
- Per-bin coherence values; a derived quality-gate flag.
- Time-domain RMS/energy envelope (decimated).
- Environment: hot-end and cold-end temperatures, fan PWM duty,
  motion/step-rate state, ambient/session ID.
- Mounting: clamp/mount identifier, preload/torque value.
- Labels: filament present/absent, material, color, jam state.
- Session baseline reference (unloaded capture ID) to enable
  delta-based features across the schema.

Open items to resolve on the bench before these harden:

- MLS vs. ESS vs. noise A/B on target hardware (sect. 2 verdict).
- Fan BPF and stepper-harmonic overlap with 1.2/3.4 kHz; possibly
  relocate bands.
- H2C stepper-driver chopper frequency vs. anti-alias filtering.
- Exciter distortion level (gates MLS-only operation).
- Baseline-vs-temperature curve for the clamped assembly.

## Sources

Canonical references (recalled from training knowledge, no live
search; verify details before formal citation): Schroeder, IEEE
Trans. Inf. Theory 16(1), 1970; Rife & Vanderkooy, JAES 37(6), 1989;
Vanderkooy, JAES 42(4), 1994; Farina, AES 108th Conv. preprint 5093,
2000; Bendat & Piersol, *Random Data*, 4th ed., Wiley, 2010;
Pintelon & Schoukens, *System Identification: A Frequency Domain
Approach*, 2nd ed., Wiley-IEEE, 2012; Wu et al., Int. J. Adv. Manuf.
Technol., ~2016-17 (bibliographic details unverified).

[^ars-wiki]: https://en.wikipedia.org/wiki/Acoustic_resonance_spectroscopy
[^st-n6-audio]: https://github.com/STMicroelectronics/STM32N6-GettingStarted-Audio
[^stratasys-suit]: https://3dprintingindustry.com/news/stratasys-targets-bambu-lab-in-new-patent-infringement-lawsuits-232084/
[^ti-stepper]: https://www.ti.com/lit/an/slvaes8a/slvaes8a.pdf
[^bpf]: https://vibromera.eu/glossary/blade-passing-frequency/
[^sensor-pos]: https://pmc.ncbi.nlm.nih.gov/articles/PMC10490794
[^rus]: https://agupubs.onlinelibrary.wiley.com/doi/full/10.1002/2015JB011932
[^dt0089]: https://www.st.com/resource/en/design_tip/dt0089-the-goertzel-algorithm-to-compute-individual-terms-of-the-discrete-fourier-transform-dft-stmicroelectronics.pdf
[^quadinterp]: https://ccrma.stanford.edu/~jos/sasp/Quadratic_Interpolation_Spectral_Peaks.html
[^ringdown]: https://www.zhinst.com/en/blogs/ring-down-method-rapid-determination-high-q-factor-resonators/
[^gatech-ae]: https://msse.gatech.edu/publication/IJAMT_FDMmonitor_wu.pdf
[^wilcoxon]: https://wilcoxon.com/wp-content/uploads/2018/11/TN21_Accelerometer-mounting-considerations_2018.pdf
[^quietprint]: https://arxiv.org/pdf/2602.02198
[^dma]: https://www.nature.com/articles/s41598-025-26846-9
[^mel-cmp]: https://www.mdpi.com/2076-3417/15/9/4679
[^gunshot]: https://arxiv.org/pdf/2606.19568
[^tinychirp]: https://www.mdpi.com/1424-8220/26/13/3972
[^am-survey]: https://pmc.ncbi.nlm.nih.gov/articles/PMC9738791/
[^cnn-preprint]: https://arxiv.org/abs/2602.16118
[^fusion-preprint]: https://arxiv.org/abs/2602.16108
[^tw-patent]: https://patents.google.com/patent/TW201725108A/en
