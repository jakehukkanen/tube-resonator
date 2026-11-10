//! Input softening bandpass filter and output DC blocker.
//!
//! See `docs/DSP.md` for the full mathematical derivation and transfer functions.

use std::f64::consts::{FRAC_1_SQRT_2, PI};

/// Second-order Direct Form II Transposed biquad filter.
#[derive(Clone, Copy, Debug, Default)]
pub struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Biquad {
    /// Configure as a 2nd-order Robert Bristow-Johnson high-pass filter.
    pub fn set_highpass(&mut self, cutoff_hz: f64, q: f64, sample_rate: f64) {
        let fc = cutoff_hz.clamp(1.0, 0.495 * sample_rate);
        let w0 = 2.0 * PI * fc / sample_rate;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * q);
        let a0_inv = 1.0 / (1.0 + alpha);

        let b0 = (1.0 + cos_w0) * 0.5 * a0_inv;
        self.b0 = b0;
        self.b1 = -(1.0 + cos_w0) * a0_inv;
        self.b2 = b0;
        self.a1 = (-2.0 * cos_w0) * a0_inv;
        self.a2 = (1.0 - alpha) * a0_inv;
    }

    /// Configure as a 2nd-order Robert Bristow-Johnson low-pass filter.
    pub fn set_lowpass(&mut self, cutoff_hz: f64, q: f64, sample_rate: f64) {
        let fc = cutoff_hz.clamp(1.0, 0.495 * sample_rate);
        let w0 = 2.0 * PI * fc / sample_rate;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * q);
        let a0_inv = 1.0 / (1.0 + alpha);

        let b0 = (1.0 - cos_w0) * 0.5 * a0_inv;
        self.b0 = b0;
        self.b1 = (1.0 - cos_w0) * a0_inv;
        self.b2 = b0;
        self.a1 = (-2.0 * cos_w0) * a0_inv;
        self.a2 = (1.0 - alpha) * a0_inv;
    }

    #[inline]
    pub fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// Cascaded 2nd-order Butterworth High-Pass + 2nd-order Butterworth Low-Pass filter.
#[derive(Clone, Debug, Default)]
pub struct SofteningBandpass {
    hp: Biquad,
    lp: Biquad,
    gain: f64,
}

impl SofteningBandpass {
    /// Update filter coefficients from center frequency (Hz) and bandwidth (octaves)
    /// across the 20 Hz..20 kHz audio range.
    pub fn update(&mut self, freq_hz: f64, bandwidth_oct: f64, sample_rate: f64) {
        let min_f = 20.0;
        let max_f = 20000.0;
        let half_bw = 0.5 * bandwidth_oct.clamp(0.1, 12.0);
        let f_center = freq_hz * 1.015;
        let f_hp = (f_center * (-half_bw).exp2()).clamp(min_f, max_f);
        let f_lp = (f_center * half_bw.exp2()).clamp(min_f, max_f);

        self.hp.set_highpass(f_hp, FRAC_1_SQRT_2, sample_rate);
        self.lp.set_lowpass(f_lp, FRAC_1_SQRT_2, sample_rate);
        self.gain = 1.0045;
    }

    #[inline]
    pub fn process(&mut self, x: f64) -> f64 {
        let y_hp = self.hp.process(x);
        self.gain * self.lp.process(y_hp)
    }

    pub fn reset(&mut self) {
        self.hp.reset();
        self.lp.reset();
    }
}

/// First-order high-pass DC blocker (`fc = 29.93 Hz` at the resonator output).
#[derive(Clone, Copy, Debug)]
pub struct DcBlocker {
    r: f64,
    x1: f64,
    y1: f64,
}

impl Default for DcBlocker {
    fn default() -> Self {
        Self::new(29.93, 44100.0)
    }
}

impl DcBlocker {
    pub fn new(cutoff_hz: f64, sample_rate: f64) -> Self {
        let mut s = Self {
            r: 0.995735,
            x1: 0.0,
            y1: 0.0,
        };
        s.set_cutoff(cutoff_hz, sample_rate);
        s
    }

    pub fn set_cutoff(&mut self, cutoff_hz: f64, sample_rate: f64) {
        let fc = cutoff_hz.clamp(1.0, 200.0);
        self.r = (1.0 - (2.0 * PI * fc / sample_rate)).clamp(0.9, 0.9999);
    }

    #[inline]
    pub fn process(&mut self, x: f64) -> f64 {
        let y = x - self.x1 + self.r * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }
}
