//! `tube-resonator-dsp`: Modular, zero-dependency Rust DSP crate implementing a
//! waveguide tube resonator and dual-Butterworth softening bandpass filter.
//!
//! # Architecture
//! - [`filter::SofteningBandpass`]: Cascaded 2nd-order Butterworth high-pass and
//!   low-pass filters ahead of the resonator delay line.
//! - [`filter::DcBlocker`]: First-order 29.93 Hz high-pass DC blocker on the output.
//! - [`lfo::PitchLfo`] & [`lfo::DecayLfo`]: Stereo pitch LFO and wavefolded 32-sample
//!   stepped decay LFO.
//! - [`resonator::WaveguideResonator`]: Fractional-delay waveguide resonator loop with cubic
//!   Lagrange interpolation, pitch-independent decay rate, and reflection damping.
//! - [`tube_resonator::TubeResonator`]: Complete stereo processor combining all stages.

pub mod filter;
pub mod lfo;
pub mod resonator;
pub mod tube_resonator;

pub use tube_resonator::{
    TubeResonator, TubeResonatorParams, midi_note_name, midi_note_to_hz,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preset_produces_finite_mono_output() {
        let mut dsp = TubeResonator::new(44100.0);
        let params = TubeResonatorParams::default();
        let mut left = vec![0.0_f32; 4096];
        let mut right = vec![0.0_f32; 4096];
        left[0] = 0.5;
        right[0] = 0.5;

        dsp.process_block(&mut left, &mut right, &params);

        let max_val = left.iter().fold(0.0_f32, |a, &b| a.max(b.abs()));
        assert!(max_val > 0.005 && max_val <= 1.0, "max_val = {max_val}");
        // At default Width = 0%, L and R must be bit-identical
        assert_eq!(left, right);
    }

    #[test]
    fn dc_blocker_toggle_removes_dc_offset() {
        let mut dsp = TubeResonator::new(44100.0);
        let mut params = TubeResonatorParams::default();
        params.filter_enabled = false;
        params.lfo_enabled = false;
        params.dc_blocker_enabled = true;

        let mut left = vec![0.01_f32; 22050];
        let mut right = vec![0.01_f32; 22050];
        dsp.process_block(&mut left, &mut right, &params);

        let tail_mean: f32 = left[16000..].iter().sum::<f32>() / (left.len() - 16000) as f32;
        assert!(
            tail_mean.abs() < 1e-3,
            "Expected near-zero DC with dc_blocker_enabled=true, got {tail_mean}"
        );
    }

    #[test]
    fn midi_note_conversion_is_accurate() {
        assert_eq!(midi_note_name(12), "C0");
        assert_eq!(midi_note_name(24), "C1");
        assert_eq!(midi_note_name(29), "F1");
        assert_eq!(midi_note_name(36), "C2");
        assert!((midi_note_to_hz(69) - 440.0).abs() < 1e-5);
        assert!((midi_note_to_hz(29) - 43.65353).abs() < 1e-4);
    }

    #[test]
    fn decay_lfo_modulates_resonator_tail() {
        let mut dsp_static = TubeResonator::new(44100.0);
        let mut dsp_mod = TubeResonator::new(44100.0);

        let mut p_static = TubeResonatorParams::default();
        p_static.decay_lfo_amount = 0.0;

        let mut p_mod = p_static;
        p_mod.decay_lfo_amount = 0.8;
        p_mod.decay_lfo_rate_hz = 4.0;

        let mut l1 = vec![0.0_f32; 8192];
        let mut r1 = vec![0.0_f32; 8192];
        l1[0] = 0.8;
        r1[0] = 0.8;

        let mut l2 = l1.clone();
        let mut r2 = r1.clone();

        dsp_static.process_block(&mut l1, &mut r1, &p_static);
        dsp_mod.process_block(&mut l2, &mut r2, &p_mod);

        let diff: f32 = l1.iter().zip(l2.iter()).map(|(a, b)| (a - b).abs()).sum();
        assert!(diff > 0.1, "Expected Decay LFO to alter tail, diff = {diff}");
    }

    #[test]
    fn pitch_lfo_clean_mode_is_smooth_per_sample_sine() {
        use crate::lfo::PitchLfo;
        let sr = 44100.0;
        let (amt, rate) = (0.5, 3.0);
        let mut lfo = PitchLfo::new(0.0);
        let mut phase: f64 = 0.0;
        for _ in 0..10_000 {
            let (l, r) = lfo.next_pitch_multipliers(true, amt, rate, 0.0, false, sr);
            let expected = (amt * phase.sin()).exp2();
            assert_eq!(l, expected);
            assert_eq!(r, expected);
            phase = (phase + std::f64::consts::TAU * rate / sr).rem_euclid(std::f64::consts::TAU);
        }
    }

    /// Energy of the 3rd-order difference (strong high-frequency emphasis, ~+18 dB/oct).
    fn hf_energy(x: &[f32]) -> f64 {
        x.windows(4)
            .map(|w| {
                let d = (w[3] - 3.0 * w[2] + 3.0 * w[1] - w[0]) as f64;
                d * d
            })
            .sum()
    }

    #[test]
    fn pitch_lfo_dirty_mode_adds_high_frequency_content() {
        let sr = 44100.0;
        let n = 88_200;
        // Sustained 55 Hz sawtooth excitation.
        let input: Vec<f32> = (0..n)
            .map(|i| (2.0 * ((i as f64 * 55.0 / sr).fract()) - 1.0) as f32 * 0.3)
            .collect();

        let render = |dirty: bool| {
            let mut dsp = TubeResonator::new(sr);
            let mut p = TubeResonatorParams::default();
            p.lfo_amount = 0.5;
            p.lfo_rate_hz = 1.0;
            p.lfo_dirty = dirty;
            p.safety_limiter = false;
            let mut l = input.clone();
            let mut r = input.clone();
            dsp.process_block(&mut l, &mut r, &p);
            l
        };

        let clean = render(false);
        let dirty = render(true);
        let ratio_db = 10.0 * (hf_energy(&dirty[4410..]) / hf_energy(&clean[4410..])).log10();
        println!("Dirty vs clean HF energy: {ratio_db:.1} dB");
        assert!(ratio_db > 3.0, "Expected Dirty to add HF content, got {ratio_db:.1} dB");
    }
}
