# Tube Resonator — User Guide

**Tube Resonator** is a waveguide cylindrical-tube resonator and dual-Butterworth softening bandpass filter plugin (VST3 & CLAP). It is designed for vocal/formant bass sound design, metallic bass physical modeling, and early-2010s dubstep/brostep growls.

---

## Mouse & Keyboard Shortcuts

| Action | Behavior |
| :--- | :--- |
| **Click + Drag Vertically** | Adjust knob value |
| **Shift + Drag Vertically** | Fine / high-precision adjustment |
| **Cmd + Click** (macOS) / **Ctrl + Click** (Win/Linux) | Reset control to its default value (**Note:** `Tune` resets to **C1 (32.70 Hz)**) |
| **Double-Click** or **Right-Click** | Reset control to its default value (`Tune` resets to **C1**) |
| **Mouse Scroll Wheel / Trackpad** | Step parameter up/down (`Shift + Scroll` for fine steps) |

---

## Left Section — Primary Controls

### 1. Resonator
Controls the core waveguide delay-line resonator.

- **`TUNE`** (`C0` to `C4`, default **`F1 (43.65 Hz)`**, resets to **`C1 (32.70 Hz)`** on `Cmd/Ctrl+Click`):
  Sets the fundamental pitch of the tube resonator, snapping to exact chromatic semitones. Low fundamentals (`C1`–`G1`, ~32–49 Hz) create deep, throaty vowel formants when swept by the Pitch LFO.
- **`DECAY`** (`10.0 ms` to `5000.0 ms`, default **`50.0 ms`**):
  Controls how long energy rings inside the waveguide loop. Short values (`20–80 ms`) produce tight, punchy vocal formants; high values (`300–5000 ms`) create long metallic ringing and self-oscillating feedback tails.
- **`DAMPING`** (`0.0%` to `100.0%`, default **`100.0%`**):
  Controls the low-pass filter inside the waveguide feedback loop. At `100%`, high-frequency reflections ring out brightly; turning `Damping` down toward `0%` absorbs high frequencies on each reflection for a darker, more muted tube tone.

### 2. Filter
A pre-resonator softening bandpass filter (cascaded 2nd-order Butterworth high-pass + low-pass, 12 dB/oct per side) placed *before* the waveguide delay line.

- **`FILTER` (LED Toggle)** (default **ON**):
  Enables or bypasses the pre-resonator softening filter.
- **`FREQ`** (`20.0 Hz` to `20000.0 Hz`, default **`67.0 Hz`**):
  Sets the center frequency of the softening passband. Keeping `Freq` low (`50–120 Hz`) strips away harsh upper sawtooth buzz before it hits the delay line, producing a round, liquid growl rather than harsh comb-filtering.
- **`WIDTH`** (`0.50 oct` to `9.00 oct`, default **`2.00 oct`**):
  Sets the width of the passband in octaves around `Freq`. Narrower values (`0.5–2.0 oct`) focus the excitation tightly; wider values let more sub and high-frequency content into the resonator.

### 3. Pitch LFO
Modulates the fundamental pitch of the Left and Right waveguide resonators exponentially using a sine wave.

- **`AMOUNT`** (`0.0%` to `100.0%`, default **`50.0%`**):
  Sets the pitch modulation depth. At `100%`, the resonator sweeps **$\pm 12\text{ semitones}$ ($\pm 1\text{ octave}$)** around `Tune`. At `50%`, it sweeps $\pm 6\text{ semitones}$ (one tritone up and down). Setting `Amount` to `0.0%` turns pitch modulation off.
- **`RATE`** (`0.01 Hz` to `30.00 Hz`, default **`1.00 Hz`**):
  Sets the speed of the Pitch LFO in Hz.
- **`PHASE`** (`0.0 deg` to `360.0 deg`, default **`0.0 deg`**):
  Sets the **stereo phase offset** between the Left and Right Pitch LFOs. At `0°`, both channels sweep pitch together in mono; increasing `Phase` toward `180°` sweeps the Left and Right channels in opposite directions for wide stereo formant movement.

### 4. Decay LFO
A wavefolded, 32-sample stepped LFO that modulates the `Decay` knob position back and forth to create rhythmic feedback bursts and ~1.4 kHz control-rate zipper harmonics that multiband compressors (like OTT) pull forward aggressively.

- **`AMOUNT`** (`0.0%` to `100.0%`, default **`0.0%`**):
  Controls how far the LFO sweeps along the `Decay` curve around your current `Decay` knob setting. Excursions past `10 ms` or `5000 ms` wavefold (bounce) back into the active range so movement never stalls.
- **`RATE`** (`0.01 Hz` to `30.00 Hz`, default **`1.00 Hz`**):
  Sets the speed of the Decay LFO in Hz.

---

## Right Section — Timbre, Stereo & Output Controls

### 5. Timbre & Detune
Fine-tunes the pitch, stereo spread, and harmonic structure of the waveguide.

- **`FINE`** (`-50.0 ct` to `+50.0 ct`, default **`0.0 ct`**):
  Fine-tunes the resonator pitch in cents (1/100th of a semitone) relative to the chromatic `Tune` note.
- **`L/R DETUNE`** (`-100.0%` to `+100.0%`, default **`0.0%`**):
  Detunes the Left and Right resonators in opposite directions (up to $\pm 0.5\text{ semitones}$) to create stereo chorus/phase width even when `Phase = 0°`.
- **`REFLECT`** (`0.0%` to `100.0%`, default **`100.0%`**):
  Controls the acoustic reflection polarity at the end of the tube:
  - **`100%` (Open Tube):** Positive reflection ($+1$), reinforcing **all harmonics** (even and odd) for a classic brassy/vocal growl.
  - **`0%` (Closed Tube):** Inverted reflection ($-1$), canceling even harmonics and reinforcing **odd harmonics only** an octave lower for a hollow, square-wave/woodwind character.
  - **`50%`:** Cancels loop feedback altogether (single half-period reflection only).

### 6. Output & Mix
Controls stereo input routing, DC filtering, raw signal passthrough, and final wet/dry mix.

- **`DC FILTER` (LED Toggle)** (default **ON**):
  Engages a 29.93 Hz high-pass DC blocker on the wet resonator output to keep sub-audio DC offset out of downstream distortion and multiband compression plugins.
- **`STEREO WIDTH`** (`0.0%` to `100.0%`, default **`0.0%`**):
  Controls the stereo separation of the input signal entering the Left and Right resonators. At `0%`, Left and Right inputs are summed to mono (`0.5 * (L + R)`); at `100%`, Left and Right channels are processed independently.
- **`DIRECT`** (`0.0%` to `100.0%`, default **`0.0%`**):
  Mixes the unfiltered, un-resonated input signal directly into the wet path before the output gain and safety limiter—useful for blending crisp high-frequency synth sizzle back on top of the softened resonator body.
- **`GAIN`** (`-24.0 dB` to `+24.0 dB`, default **`0.0 dB`**):
  Adjusts the wet resonator level prior to the internal 0 dBFS safety limiter and Dry/Wet mixer.
- **`DRY / WET`** (`0.0%` to `100.0%`, default **`100.0%`**):
  Blends between the dry unprocessed input (`0%`) and the processed wet output (`100%`).
