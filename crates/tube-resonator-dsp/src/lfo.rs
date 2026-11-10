//! Low-Frequency Oscillators for resonator pitch (`PitchLfo`) and decay (`DecayLfo`) modulation.
//!
//! See `docs/DSP.md` for full mathematical details.

use std::f64::consts::TAU;

/// Stereo LFO supporting exponential sine pitch modulation (`[-amount, +amount]` octaves,
/// where `1.0` = +/- 12 semitones).
#[derive(Clone, Copy, Debug, Default)]
pub struct PitchLfo {
    phase: f64,
}

impl PitchLfo {
    pub fn new(initial_phase_rad: f64) -> Self {
        Self {
            phase: initial_phase_rad.rem_euclid(TAU),
        }
    }

    /// Reset the LFO phase (in radians).
    pub fn set_phase_rad(&mut self, phase_rad: f64) {
        self.phase = phase_rad.rem_euclid(TAU);
    }

    /// Reset the LFO phase (in degrees, `0.0..=360.0`).
    pub fn set_phase_deg(&mut self, phase_deg: f64) {
        self.set_phase_rad(phase_deg.to_radians());
    }

    /// Advance the LFO by one sample and return `(pitch_mul_l, pitch_mul_r)`
    /// where `f_inst = f_tune * pitch_mul`.
    #[inline]
    pub fn next_pitch_multipliers(
        &mut self,
        enabled: bool,
        amount_0_to_1: f64,
        rate_hz: f64,
        stereo_phase_offset_deg: f64,
        sample_rate: f64,
    ) -> (f64, f64) {
        let cur_phase = self.phase;
        self.phase = (self.phase + TAU * rate_hz / sample_rate).rem_euclid(TAU);

        if !enabled || amount_0_to_1 <= 0.0 {
            return (1.0, 1.0);
        }

        let amt = amount_0_to_1.clamp(0.0, 1.0);
        let mod_l = amt * cur_phase.sin();
        let offset_rad = stereo_phase_offset_deg.to_radians();
        let mod_r = if offset_rad.abs() < 1e-9 {
            mod_l
        } else {
            amt * (cur_phase + offset_rad).sin()
        };

        (mod_l.exp2(), mod_r.exp2())
    }
}

/// Stepped 32-sample sub-block LFO for modulating the resonator `Decay` parameter in normalized
/// knob space (`10 ms .. 5000 ms`) with wavefolding at boundaries.
#[derive(Clone, Copy, Debug, Default)]
pub struct DecayLfo {
    phase: f64,
    step_counter: u32,
    held_val: f64,
}

impl DecayLfo {
    pub fn new(initial_phase_rad: f64) -> Self {
        let phase = initial_phase_rad.rem_euclid(TAU);
        Self {
            phase,
            step_counter: 0,
            held_val: 0.0,
        }
    }

    pub fn set_phase_rad(&mut self, phase_rad: f64) {
        self.phase = phase_rad.rem_euclid(TAU);
        self.step_counter = 0;
        self.held_val = 0.0;
    }

    /// Reflect an arbitrary value `v` into `[0.0, 1.0]` like a triangle wave (wavefolding)
    /// so the knob trajectory never stalls flat against the `0.0` or `1.0` stops.
    #[inline]
    fn fold_0_to_1(v: f64) -> f64 {
        let period2 = v.rem_euclid(2.0);
        if period2 <= 1.0 {
            period2
        } else {
            2.0 - period2
        }
    }

    /// Advance the Decay LFO by one sample and return the modulated decay time in seconds.
    #[inline]
    pub fn next_decay_seconds(
        &mut self,
        base_decay_s: f64,
        amount_0_to_1: f64,
        rate_hz: f64,
        sample_rate: f64,
    ) -> f64 {
        const CONTROL_STEP_SAMPLES: u32 = 32;

        if amount_0_to_1 <= 0.0 {
            self.phase = (self.phase + TAU * rate_hz / sample_rate).rem_euclid(TAU);
            self.step_counter = 0;
            return base_decay_s;
        }

        if self.step_counter == 0 {
            let p = self.phase;
            self.held_val = 0.72 * p.sin() + 0.20 * (2.0 * p).sin() + 0.08 * (3.0 * p).cos();
        }
        self.step_counter = (self.step_counter + 1) % CONTROL_STEP_SAMPLES;
        self.phase = (self.phase + TAU * rate_hz / sample_rate).rem_euclid(TAU);

        let base_ms = (base_decay_s * 1000.0).clamp(10.0, 5000.0);
        let x_base = ((base_ms - 10.0) / 4990.0).clamp(0.0, 1.0).powf(0.25);

        let amt = amount_0_to_1.clamp(0.0, 1.0);
        let x_mod = Self::fold_0_to_1(x_base + amt * 0.75 * self.held_val);
        let mod_ms = 10.0 + 4990.0 * x_mod.powi(4);

        (mod_ms * 0.001).clamp(0.010, 5.0)
    }
}
