//! Complete stereo Tube Resonator processor (`TubeResonator`) and parameter struct (`TubeResonatorParams`).
//!
//! Designed for zero-dependency modularity so it can be embedded directly into any
//! Rust audio plugin or multi-effect processor.

use crate::filter::{DcBlocker, SofteningBandpass};
use crate::lfo::{DecayLfo, PitchLfo};
use crate::resonator::WaveguideResonator;

/// Convert a MIDI note number (e.g. `29` for F1 = 43.65 Hz) to frequency in Hz (`A4 = 69 = 440 Hz`).
#[inline]
pub fn midi_note_to_hz(note: i32) -> f32 {
    440.0 * ((note - 69) as f32 / 12.0).exp2()
}

/// Format a MIDI note number (`0..=127`) as a standard note name (`C0..B8`, where `24 = C1`, `29 = F1`, `60 = C4`).
pub fn midi_note_name(note: i32) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let idx = note.rem_euclid(12) as usize;
    let octave = note.div_euclid(12) - 1;
    format!("{}{}", NAMES[idx], octave)
}

/// Parameters for [`TubeResonator`].
#[derive(Clone, Copy, Debug)]
pub struct TubeResonatorParams {
    /// Fundamental frequency in Hz (default `43.65353 Hz` = F1).
    pub tune_hz: f32,
    /// Fine tuning offset in cents (`-50.0..=50.0`, default `0.0`).
    pub fine_cents: f32,
    /// Stereo pitch detune between L and R resonators (`-1.0..=1.0`, default `0.0`).
    pub lr_detune: f32,
    /// Decay time in milliseconds (`10.0..=5000.0`, default `50.0 ms`).
    pub decay_ms: f32,
    /// Loop high-frequency retention / damping (`0.0..=1.0`, default `1.0` = 100%).
    pub damping: f32,
    /// End reflection polarity (`0.0..=1.0`, default `1.0` = 100% open tube).
    pub reflect: f32,
    /// Stereo width (`0.0..=1.0`, default `0.0` = mono sum, `1.0` = full stereo).
    pub width: f32,
    /// Enable LFO pitch modulation (default `true`).
    pub lfo_enabled: bool,
    /// Pitch LFO modulation amount (`0.0..=1.0`, where `1.0` = +/- 12 semitones, default `0.5`).
    pub lfo_amount: f32,
    /// Pitch LFO rate in Hz (`0.01..=30.0`, default `1.0 Hz`).
    pub lfo_rate_hz: f32,
    /// Pitch LFO stereo phase offset in degrees (`0.0..=360.0`, default `0.0`).
    pub lfo_stereo_phase_deg: f32,
    /// Decay LFO modulation amount (`0.0..=1.0`, default `0.0`).
    pub decay_lfo_amount: f32,
    /// Decay LFO rate in Hz (`0.01..=30.0`, default `1.0 Hz`).
    pub decay_lfo_rate_hz: f32,
    /// Enable the pre-resonator softening bandpass filter (default `true`).
    pub filter_enabled: bool,
    /// Filter center frequency in Hz (`20.0..=20000.0`, default `67.0 Hz`).
    pub filter_freq_hz: f32,
    /// Filter bandwidth in octaves (`0.5..=9.0`, default `2.0`).
    pub filter_bandwidth_oct: f32,
    /// Enable output DC-blocking high-pass filter at 29.93 Hz (default `true`).
    pub dc_blocker_enabled: bool,
    /// Direct unfiltered input passthrough (`0.0..=1.0`, default `0.0`).
    pub direct: f32,
    /// Output wet gain in dB (`-24.0..=24.0`, default `0.0 dB`).
    pub gain_db: f32,
    /// Dry/Wet mix (`0.0..=1.0`, default `1.0` = 100% wet).
    pub dry_wet: f32,
    /// Engage 0 dBFS safety limiter on wet output (default `true`).
    pub safety_limiter: bool,
}

impl Default for TubeResonatorParams {
    fn default() -> Self {
        Self {
            tune_hz: midi_note_to_hz(29), // F1 = 43.65353 Hz
            fine_cents: 0.0,
            lr_detune: 0.0,
            decay_ms: 50.0,
            damping: 1.0,
            reflect: 1.0,
            width: 0.0,
            lfo_enabled: true,
            lfo_amount: 0.50,
            lfo_rate_hz: 1.00,
            lfo_stereo_phase_deg: 0.0,
            decay_lfo_amount: 0.0,
            decay_lfo_rate_hz: 1.00,
            filter_enabled: true,
            filter_freq_hz: 67.0,
            filter_bandwidth_oct: 2.0,
            dc_blocker_enabled: true,
            direct: 0.0,
            gain_db: 0.0,
            dry_wet: 1.0,
            safety_limiter: true,
        }
    }
}

/// Complete stereo Tube Resonator effect processor.
#[derive(Clone, Debug)]
pub struct TubeResonator {
    sample_rate: f64,
    filter_l: SofteningBandpass,
    filter_r: SofteningBandpass,
    res_l: WaveguideResonator,
    res_r: WaveguideResonator,
    dc_l: DcBlocker,
    dc_r: DcBlocker,
    lfo: PitchLfo,
    decay_lfo: DecayLfo,
    limiter_env: f64,
}

impl Default for TubeResonator {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

impl TubeResonator {
    pub fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            sample_rate: sample_rate.max(8000.0),
            filter_l: SofteningBandpass::default(),
            filter_r: SofteningBandpass::default(),
            res_l: WaveguideResonator::new(),
            res_r: WaveguideResonator::new(),
            dc_l: DcBlocker::new(29.93, sample_rate),
            dc_r: DcBlocker::new(29.93, sample_rate),
            lfo: PitchLfo::new(0.0),
            decay_lfo: DecayLfo::new(0.0),
            limiter_env: 1.0,
        };
        s.set_sample_rate(sample_rate);
        s
    }

    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate.max(8000.0);
        self.dc_l.set_cutoff(29.93, self.sample_rate);
        self.dc_r.set_cutoff(29.93, self.sample_rate);
    }

    pub fn reset(&mut self) {
        self.filter_l.reset();
        self.filter_r.reset();
        self.res_l.reset();
        self.res_r.reset();
        self.dc_l.reset();
        self.dc_r.reset();
        self.limiter_env = 1.0;
    }

    /// Explicitly set the Pitch LFO phase in radians (useful for deterministic offline renders
    /// or DAW transport sync).
    pub fn set_lfo_phase_rad(&mut self, phase_rad: f64) {
        self.lfo.set_phase_rad(phase_rad);
    }

    /// Process a stereo block of samples in place.
    pub fn process_block(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        params: &TubeResonatorParams,
    ) {
        let n = left.len().min(right.len());
        let sr = self.sample_rate;

        self.filter_l.update(
            params.filter_freq_hz as f64,
            params.filter_bandwidth_oct as f64,
            sr,
        );
        self.filter_r.update(
            params.filter_freq_hz as f64,
            params.filter_bandwidth_oct as f64,
            sr,
        );

        let base_tune = (params.tune_hz as f64) * ((params.fine_cents as f64) / 1200.0).exp2();
        let detune_st = (params.lr_detune as f64).clamp(-1.0, 1.0);
        let detune_mul_l = (-0.5 * detune_st / 12.0).exp2();
        let detune_mul_r = (0.5 * detune_st / 12.0).exp2();

        let decay_s = (params.decay_ms as f64) * 0.001;
        let damping = params.damping as f64;
        let reflect = params.reflect as f64;
        let width = (params.width as f64).clamp(0.0, 1.0);
        let direct = (params.direct as f64).clamp(0.0, 1.0);
        let wet_gain = 10.0_f64.powf((params.gain_db as f64) / 20.0);
        let dry_wet = (params.dry_wet as f64).clamp(0.0, 1.0);

        // Input stereo width blending:
        // At Width = 0%, L and R inputs sum to mono (0.5 * (L + R)).
        // At Width = 100%, L and R remain independent stereo channels.
        let mid_mix = 0.5 * (1.0 - width);

        for i in 0..n {
            let dry_l = left[i] as f64;
            let dry_r = right[i] as f64;

            let in_l = (1.0 - mid_mix) * dry_l + mid_mix * dry_r;
            let in_r = (1.0 - mid_mix) * dry_r + mid_mix * dry_l;

            let exc_l = if params.filter_enabled {
                self.filter_l.process(in_l)
            } else {
                in_l
            };
            let exc_r = if params.filter_enabled {
                self.filter_r.process(in_r)
            } else {
                in_r
            };

            let (lfo_l, lfo_r) = self.lfo.next_pitch_multipliers(
                params.lfo_enabled,
                params.lfo_amount as f64,
                params.lfo_rate_hz as f64,
                params.lfo_stereo_phase_deg as f64,
                sr,
            );
            let inst_decay_s = self.decay_lfo.next_decay_seconds(
                decay_s,
                params.decay_lfo_amount as f64,
                params.decay_lfo_rate_hz as f64,
                sr,
            );

            let f_l = base_tune * detune_mul_l * lfo_l;
            let f_r = base_tune * detune_mul_r * lfo_r;

            let out_l = self
                .res_l
                .process_sample(exc_l, f_l, inst_decay_s, damping, reflect, sr);
            let out_r = if (width < 1e-6)
                && (detune_st.abs() < 1e-6)
                && (params.lfo_stereo_phase_deg.abs() < 1e-6)
            {
                let _ = self
                    .res_r
                    .process_sample(exc_r, f_r, inst_decay_s, damping, reflect, sr);
                out_l
            } else {
                self.res_r
                    .process_sample(exc_r, f_r, inst_decay_s, damping, reflect, sr)
            };

            let mut wet_l = if params.dc_blocker_enabled {
                self.dc_l.process(out_l)
            } else {
                out_l
            };
            let mut wet_r = if params.dc_blocker_enabled {
                self.dc_r.process(out_r)
            } else {
                out_r
            };

            if direct > 0.0 {
                wet_l += direct * in_l;
                wet_r += direct * in_r;
            }

            wet_l *= wet_gain;
            wet_r *= wet_gain;

            if params.safety_limiter {
                let peak = wet_l.abs().max(wet_r.abs());
                let target_g = if peak > 1.0 { 1.0 / peak } else { 1.0 };
                let coeff = if target_g < self.limiter_env {
                    0.2
                } else {
                    0.0005
                };
                self.limiter_env += coeff * (target_g - self.limiter_env);
                wet_l = (wet_l * self.limiter_env).clamp(-1.0, 1.0);
                wet_r = (wet_r * self.limiter_env).clamp(-1.0, 1.0);
            }

            left[i] = ((1.0 - dry_wet) * dry_l + dry_wet * wet_l) as f32;
            right[i] = ((1.0 - dry_wet) * dry_r + dry_wet * wet_r) as f32;
        }
    }
}
