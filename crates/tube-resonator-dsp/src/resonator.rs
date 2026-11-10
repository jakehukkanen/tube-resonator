//! Waveguide Tube Resonator (`WaveguideResonator`).
//!
//! See `docs/DSP.md` for full physical modeling and DSP equations.

use std::f64::consts::PI;

/// Maximum delay buffer length in samples (supports down to ~10 Hz at 192 kHz).
const MAX_DELAY_SAMPLES: usize = 32768;
const DELAY_MASK: usize = MAX_DELAY_SAMPLES - 1;

/// Reference acoustic frequency (Hz) for 1/f0 level normalization.
const REF_FREQ_HZ: f64 = 44.0;
/// Open-tube acoustic end-correction ratio (~10.4 cents).
const END_CORRECTION_RATIO: f64 = 1.006;
/// Reference excitation gain at `REF_FREQ_HZ`.
const REF_EXCITATION_GAIN: f64 = -5.88;

/// Single-channel fractional-delay waveguide resonator.
#[derive(Clone, Debug)]
pub struct WaveguideResonator {
    buffer: Vec<f64>,
    write_pos: usize,
    lp_state: f64,
}

impl Default for WaveguideResonator {
    fn default() -> Self {
        Self::new()
    }
}

impl WaveguideResonator {
    pub fn new() -> Self {
        Self {
            buffer: vec![0.0; MAX_DELAY_SAMPLES],
            write_pos: 0,
            lp_state: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
        self.lp_state = 0.0;
    }

    /// Read from the circular delay buffer at `delay_samples` behind `self.write_pos`
    /// using 4-point cubic Lagrange interpolation for smooth LFO pitch sweeps.
    #[inline]
    fn read_cubic(&self, delay_samples: f64) -> f64 {
        let d = delay_samples.clamp(2.0, (MAX_DELAY_SAMPLES - 4) as f64);
        let d_int = d.floor() as usize;
        let frac = d - (d_int as f64);

        let i_m1 = self.write_pos.wrapping_sub(d_int - 1) & DELAY_MASK;
        let i_0 = self.write_pos.wrapping_sub(d_int) & DELAY_MASK;
        let i_1 = self.write_pos.wrapping_sub(d_int + 1) & DELAY_MASK;
        let i_2 = self.write_pos.wrapping_sub(d_int + 2) & DELAY_MASK;

        let ym1 = self.buffer[i_m1];
        let y0 = self.buffer[i_0];
        let y1 = self.buffer[i_1];
        let y2 = self.buffer[i_2];

        // 4-point cubic Lagrange interpolation
        let c0 = y0;
        let c1 = 0.5 * (y1 - ym1);
        let c2 = ym1 - 2.5 * y0 + 2.0 * y1 - 0.5 * y2;
        let c3 = 0.5 * (y2 - ym1) + 1.5 * (y0 - y1);

        ((c3 * frac + c2) * frac + c1) * frac + c0
    }

    /// Process a single sample through the waveguide loop.
    ///
    /// - `tune_hz`: instantaneous fundamental frequency (including LFO modulation).
    /// - `decay_s`: decay time in seconds (`0.050` for 50 ms).
    /// - `damping_0_1`: loop high-frequency retention (`0.0..=1.0`, default `1.0` = least damped).
    /// - `reflect_0_1`: end reflection polarity (`0.0..=1.0`, default `1.0` = open tube, `0.0` = closed tube).
    #[inline]
    pub fn process_sample(
        &mut self,
        input: f64,
        tune_hz: f64,
        decay_s: f64,
        damping_0_1: f64,
        reflect_0_1: f64,
        sample_rate: f64,
    ) -> f64 {
        let f0 = (tune_hz * END_CORRECTION_RATIO).clamp(15.0, 5000.0);
        let period_samples = sample_rate / f0;
        let half_period_samples = 0.5 * period_samples;

        // Loop decay per period: exp(-4 / (f0 * decay_s))
        let decay_clamped = decay_s.max(0.005);
        let g_decay = (-4.0 / (f0 * decay_clamped)).exp();

        // Reflect controls boundary polarity: 1.0 -> +1 (open tube, all harmonics),
        // 0.0 -> -1 (closed tube, odd harmonics only).
        let reflection_sign = 2.0 * reflect_0_1.clamp(0.0, 1.0) - 1.0;

        // Damping controls the one-pole loop low-pass cutoff (100% -> ~18 kHz; 0% -> ~400 Hz).
        let damping = damping_0_1.clamp(0.0, 1.0);
        let lp_a = if damping >= 0.999 {
            0.22 * (sample_rate / 44100.0).recip().clamp(0.1, 0.9)
        } else {
            let lp_cutoff = 400.0 * (40.0_f64).powf(damping);
            (-2.0 * PI * lp_cutoff / sample_rate).exp().clamp(0.0, 0.995)
        };
        let loop_delay_comp = lp_a / (1.0 - lp_a);

        // Read feedback from one full period ago (compensated for loop filter group delay)
        let delayed_full = self.read_cubic((period_samples - loop_delay_comp).max(2.0));
        self.lp_state = (1.0 - lp_a) * delayed_full + lp_a * self.lp_state;

        // Write into waveguide delay line
        self.buffer[self.write_pos] = input + g_decay * reflection_sign * self.lp_state;

        // Read output at half-period tap, scaled by 1/f0 pitch normalization
        let delayed_half = self.read_cubic((half_period_samples - 0.5 * loop_delay_comp).max(2.0));
        let pitch_norm_gain = REF_EXCITATION_GAIN * (REF_FREQ_HZ / f0);
        let out = pitch_norm_gain * delayed_half;

        self.write_pos = (self.write_pos + 1) & DELAY_MASK;
        out
    }
}
