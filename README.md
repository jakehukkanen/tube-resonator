# Tube Resonator

A waveguide tube resonator with a softening bandpass filter similar to Ableton's Corpus in Pipe mode, written in Rust, with both VST3/CLAP available. Make your old-school Skrillex growls in any DAW!

> ⚠️ **VIBECODED WARNING:** This project (DSP algorithms and VST3/CLAP plugin code) was vibecoded with AI assistance. Read the code, verify it in your DAW with a limiter on your master bus, and use at your own risk!

---

## Documentation

- 📖 **[User Guide (`docs/USER_GUIDE.md`)](./docs/USER_GUIDE.md)** — Complete walkthrough of every knob, switch, keyboard shortcut, and sound-design tips.
- 📐 **[DSP Architecture & Math (`docs/DSP.md`)](./docs/DSP.md)** — Transfer functions, waveguide equations, pitch-independent decay law, and LFO modulation formulas.

---

## Building & Installing the Plugin

### From GitHub Actions / Pre-Built Releases
Cross-platform builds for **macOS (`arm64`)**, **Windows (`x86_64`)**, and **Linux (`x86_64`)** are compiled automatically via [`.github/workflows/build.yml`](./.github/workflows/build.yml).

### Building Locally from Source
```bash
# Run unit tests
cargo test --workspace

# Build release binaries (with local path remapping so personal paths aren't embedded)
RUSTFLAGS="--remap-path-prefix=$PWD=." cargo build --release --workspace
```

### Installing & Bypassing macOS Gatekeeper ("Apple Jail")
On macOS, copy `Tube Resonator.vst3` to `~/Library/Audio/Plug-Ins/VST3/` and `Tube Resonator.clap` to `~/Library/Audio/Plug-Ins/CLAP/`.

Because the plugin bundles are not notarized with a paid Apple Developer ID certificate, macOS Gatekeeper may quarantine downloaded bundles or report that *"Tube Resonator.vst3 is damaged and can't be opened"*. Run the following commands in Terminal to strip the quarantine attribute and apply a local ad-hoc signature:

```bash
# 1. Remove Apple's quarantine extended attributes
xattr -cr ~/Library/Audio/Plug-Ins/VST3/"Tube Resonator.vst3"
xattr -cr ~/Library/Audio/Plug-Ins/CLAP/"Tube Resonator.clap"

# 2. Re-sign locally with an ad-hoc signature
codesign --force --deep --sign - ~/Library/Audio/Plug-Ins/VST3/"Tube Resonator.vst3"
codesign --force --deep --sign - ~/Library/Audio/Plug-Ins/CLAP/"Tube Resonator.clap"
```

On **Windows**, copy `Tube Resonator.vst3` to `C:\Program Files\Common Files\VST3\` and `Tube Resonator.clap` to `C:\Program Files\Common Files\CLAP\`.
On **Linux**, copy them to `~/.vst3/` and `~/.clap/`.

---

## DSP Highlights

1. **The "Softening" Pre-Resonator Filter (`SofteningBandpass`):**
   - Series cascade of a **2nd-order Butterworth High-Pass filter ($Q = 0.7071$, +12 dB/oct)** and a **2nd-order Butterworth Low-Pass filter ($Q = 0.7071$, −12 dB/oct)** across **20 Hz – 20 kHz** (default `Freq = 67.0 Hz`, `Width = 2.00 oct`).
   - Gently rounds off harsh sawtooth/FM transients *before* they excite the waveguide delay line.
2. **Waveguide Tube Resonator (`WaveguideResonator`):**
   - **Note-Snapping `Tune` Control:** Snaps to exact chromatic notes from **C0 (16.35 Hz) to C4 (261.63 Hz)** (default `F1 = 43.65 Hz`; `Cmd/Ctrl + Click` or double-click resets `Tune` to `C1 = 32.70 Hz`).
   - Fractional-delay waveguide loop with 4-point cubic Lagrange interpolation, pitch-independent decay rate, loop `Damping`, boundary `Reflect` polarity (open vs. closed tube), and a 29.93 Hz output `DC Filter` (default ON).
3. **Dual LFO Modulation (`PitchLfo` & `DecayLfo`):**
   - **Pitch LFO:** Exponential sine pitch sweep ($\pm 12\text{ semitones}$ at 100% Amount; default `50%` at `1.00 Hz`) with stereo `Phase` offset.
   - **Decay LFO:** Wavefolded 32-sample sub-block stepped modulator designed to inject ~1.4 kHz control-rate zipper harmonics and feedback bursts that multiband compressors (like OTT) pull forward.

---

## Workspace Structure

```text
tube-resonator/
├── .github/workflows/
│   └── build.yml                  # Cross-platform CI build (macOS, Windows, Linux)
├── crates/
│   ├── tube-resonator-dsp/        # Pure, zero-dependency Rust DSP library
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── filter.rs          # Cascaded 2nd-order Butterworth HP + LP filter & 29.93 Hz DC blocker
│   │       ├── lfo.rs             # Stereo Pitch LFO & wavefolded 32-sample stepped Decay LFO
│   │       ├── resonator.rs       # Fractional-delay waveguide resonator (cubic Lagrange interpolation)
│   │       └── tube_resonator.rs  # Complete stereo processor + TubeResonatorParams struct
│   ├── tube-resonator/            # "Tube Resonator" VST3/CLAP plugin & warm skeuomorphic Vizia GUI
│   └── tube-resonator-render/     # Offline WAV-to-WAV CLI renderer for automated verification
└── docs/
    ├── USER_GUIDE.md              # Full user manual & parameter reference
    └── DSP.md                     # Mathematical derivations & transfer functions
```

---

## License

Licensed under the **GNU General Public License v3.0 or later (GPL-3.0-or-later)**. See [`LICENSE`](./LICENSE) for details.

---

## Acknowledgements

- **System 53**, whose [breakdown of the classic growl bass sound design inspired this project and its default preset](https://www.youtube.com/watch?v=uggzeuncLH0).
- **[`nih-plug`](https://github.com/robbert-vdh/nih-plug)** by Robbert van der Helm for the Rust VST3/CLAP plugin framework and Vizia GUI integration.
- **Claude** (Anthropic) and **Gemini** (Google DeepMind) for assisting with the signal-processing design and Rust DSP/plugin implementation.
