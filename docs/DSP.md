# Tube Resonator DSP Architecture & Mathematical Model

This document details the digital signal processing (DSP) algorithms implemented in [`crates/tube-resonator-dsp`](../crates/tube-resonator-dsp).

---

## 1. Pre-Resonator Softening Filter (`SofteningBandpass`)

Rather than a standard constant-$Q$ 2nd-order bandpass biquad, the pre-resonator filter is a series cascade of two 2nd-order Butterworth filters ($Q = 1 / \sqrt{2} \approx 0.7071$):

1. **2nd-order Butterworth High-Pass Filter** ($+12\text{ dB/oct}$ slope)
2. **2nd-order Butterworth Low-Pass Filter** ($-12\text{ dB/oct}$ slope)

Given a center frequency $f_{\text{freq}}$ (with a $1.015\times$ internal calibration factor, $f_c = 1.015 \cdot f_{\text{freq}}$) and bandwidth $\text{BW}$ in octaves, the high-pass and low-pass cutoff frequencies are symmetric in log-frequency space:

$$f_{\text{HP}} = \text{clamp}\!\left(f_c \cdot 2^{-\text{BW}/2},\; 20\text{ Hz},\; 20000\text{ Hz}\right)$$

$$f_{\text{LP}} = \text{clamp}\!\left(f_c \cdot 2^{+\text{BW}/2},\; 20\text{ Hz},\; 20000\text{ Hz}\right)$$

At the default settings (`Freq = 67.0 Hz`, `Width = 2.00 oct`), $f_{\text{HP}} \approx 34.0\text{ Hz}$ and $f_{\text{LP}} \approx 136.0\text{ Hz}$, gently rounding sharp sawtooth or FM transients before they excite the waveguide delay line.

---

## 2. Fractional-Delay Waveguide Resonator (`WaveguideResonator`)

### 2.1 Loop Period & Half-Period Output Tap
For a nominal `Tune` frequency $f_{\text{tune}}$, the effective acoustic fundamental is:

$$f_0 = 1.006 \cdot f_{\text{tune}}$$

(~10.4 cents sharper due to open-tube acoustic end correction and loop filter phase).

- **Excitation & Recirculation (`period_samples`):** Incoming filtered audio enters the circular delay line and recirculates around a full-period loop ($D_{\text{loop}} = f_s / f_0 - D_{\text{lp}}$, where $D_{\text{lp}} = a_{\text{lp}} / (1 - a_{\text{lp}})$ compensates for the one-pole loop damping filter's group delay).
- **Half-Period Output Tap (`half_period_samples`):** The output is read from a half-period tap ($D_{\text{half}} = \tfrac{1}{2} D_{\text{loop}}$) using **4-point cubic Lagrange interpolation** for artifact-free fractional delay modulation during LFO pitch sweeps.
- **Pitch-Normalized Excitation Gain:** To keep output level consistent across the `C0`–`C4` range despite changing delay-line density, the output tap is scaled by:
  $$A_0(f_0) = -5.88 \cdot \frac{44.0\text{ Hz}}{f_0}$$

### 2.2 Pitch-Independent Decay Law (`Decay`)
The feedback gain per period $g_{\text{decay}}$ is computed so that the RT60 / decay rate in milliseconds is independent of the resonator's fundamental frequency $f_0$:

$$g_{\text{decay}}(f_0, \tau_{\text{decay}}) = \exp\!\left(-\frac{4.0}{f_0 \cdot \tau_{\text{decay}}}\right)$$

At $\tau_{\text{decay}} = 50\text{ ms}$ (`0.050 s`), the envelope decays at a constant $-694.9\text{ dB/s}$ ($g_{\text{decay}} \approx 0.1634$ per period at $44\text{ Hz}$).

### 2.3 Boundary Reflection Polarity (`Reflect`)
The `Reflect` parameter ($r \in [0.0, 1.0]$) crossfades the boundary reflection sign between $-1$ and $+1$:

$$\text{sign}_{\text{reflect}} = 2 r - 1$$

- **`Reflect = 100%` ($\text{sign} = +1$, open cylindrical tube):** Reinforces both even and odd harmonics ($f_0, 2f_0, 3f_0, \dots$).
- **`Reflect = 0%` ($\text{sign} = -1$, closed cylindrical tube):** Inverts every period, canceling even harmonics and reinforcing only odd harmonics ($f_0, 3f_0, 5f_0, \dots$) an octave lower for a hollow, square/clarinet-like timbre.

### 2.4 Loop Reflection Filter (`Damping`)
A one-pole low-pass filter inside the feedback loop models high-frequency energy loss at the tube walls and open ends:

$$y_{\text{lp}}[n] = (1 - a_{\text{lp}}) \cdot x_{\text{delayed}}[n] + a_{\text{lp}} \cdot y_{\text{lp}}[n - 1]$$

where `Damping = 100%` sets the highest cutoff (~18 kHz, brightest metallic ring) and `Damping = 0%` lowers the loop cutoff toward `400 Hz` for dark, muted resonance.

---

## 3. Modulation Oscillators (`PitchLfo` & `DecayLfo`)

### 3.1 Stereo Pitch LFO (`PitchLfo`)
Modulates Left and Right resonator fundamentals exponentially with a maximum depth of **$\pm 12\text{ semitones}$ ($\pm 1\text{ octave}$)** at `100%` Amount:

$$f_L(t) = f_{\text{base}, L} \cdot 2^{\,\text{amount} \cdot \sin(2\pi f_{\text{rate}} t)}$$

$$f_R(t) = f_{\text{base}, R} \cdot 2^{\,\text{amount} \cdot \sin(2\pi f_{\text{rate}} t + \phi)}$$

where $\phi$ is the stereo `Phase` offset (`0°..360°`).

### 3.2 Wavefolded 32-Sample Stepped Decay LFO (`DecayLfo`)
Designed specifically to generate metallic waveguide-gain transients that multiband upward/downward compressors (such as OTT) pull forward:

1. **Normalized Knob-Space Mapping:** Converts `Decay` ($\tau_{\text{decay}} \in [10\text{ ms}, 5000\text{ ms}]$) into its normalized skewed knob coordinate $x_{\text{base}} \in [0, 1]$ via $\tau_{\text{ms}}(x) = 10 + 4990 x^4$.
2. **32-Sample Sub-Block Staircase:** Samples a multi-harmonic waveform ($0.72\sin\theta + 0.20\sin 2\theta + 0.08\cos 3\theta$) strictly once every **32 samples** ($\approx 1.38\text{ kHz}$ step rate at $44.1\text{ kHz}$), matching the control-rate sub-block stepping of manual knob automation.
3. **Boundary Wavefolding:** Reflects excursions outside $[0.0, 1.0]$ back into the active range so the decay time never stalls flat at the `10 ms` floor.

---

## 4. Output Stage (`DcBlocker`, `Direct`, Safety Limiter)

- **DC Filter (`DcBlocker`):** First-order high-pass filter at $f_c = 29.93\text{ Hz}$ ($H(z) = \frac{1 - z^{-1}}{1 - R z^{-1}}$) removing sub-audio DC offset introduced by asymmetric waveforms or high feedback.
- **Direct (`direct`):** Mixes the width-processed, unfiltered input signal directly into the wet resonator output prior to `Gain` and the 0 dBFS safety limiter.
