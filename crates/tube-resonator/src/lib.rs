//! `tube-resonator`: VST3 and CLAP waveguide resonator plugin built with `nih-plug`.
//!
//! Features:
//! - `Tune` snaps directly to exact chromatic notes (`C0` to `C4`, defaulting to `F1 = 43.65 Hz`,
//!   and resetting to `C1 = 32.70 Hz` on `Cmd/Ctrl + Click` or double-click)
//! - `Filter Freq` adjustable across 20 Hz..20 kHz (default `67.0 Hz`, `2.00 oct`)
//! - Independent `Pitch LFO` and wavefolded 32-sample `Decay LFO` modulation
//! - Output `DC Filter` toggle (default ON)

use std::num::NonZeroU32;
use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use tube_resonator_dsp::{TubeResonator, TubeResonatorParams, midi_note_name, midi_note_to_hz};

mod editor;

pub struct TubeResonatorPlugin {
    params: Arc<TubeResonatorPluginParams>,
    dsp: TubeResonator,
}

#[derive(Params)]
pub struct TubeResonatorPluginParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    /// Chromatic note selector (`12 = C0 = 16.35 Hz` .. `60 = C4 = 261.63 Hz`, default `29 = F1 = 43.65 Hz`).
    #[id = "tune"]
    pub tune_note: IntParam,

    #[id = "fine"]
    pub fine_cents: FloatParam,

    #[id = "lr_detune"]
    pub lr_detune: FloatParam,

    #[id = "decay"]
    pub decay_ms: FloatParam,

    #[id = "damping"]
    pub damping: FloatParam,

    #[id = "reflect"]
    pub reflect: FloatParam,

    #[id = "width"]
    pub width: FloatParam,

    #[id = "lfo_amt"]
    pub lfo_amount: FloatParam,

    #[id = "lfo_rate"]
    pub lfo_rate_hz: FloatParam,

    #[id = "lfo_phase"]
    pub lfo_stereo_phase_deg: FloatParam,

    #[id = "dlfo_amt"]
    pub decay_lfo_amount: FloatParam,

    #[id = "dlfo_rate"]
    pub decay_lfo_rate_hz: FloatParam,

    #[id = "flt_on"]
    pub filter_enabled: BoolParam,

    #[id = "flt_freq"]
    pub filter_freq_hz: FloatParam,

    #[id = "flt_bw"]
    pub filter_bandwidth_oct: FloatParam,

    #[id = "dc_flt"]
    pub dc_blocker_enabled: BoolParam,

    #[id = "direct"]
    pub direct: FloatParam,

    #[id = "gain"]
    pub gain_db: FloatParam,

    #[id = "dry_wet"]
    pub dry_wet: FloatParam,
}

impl Default for TubeResonatorPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(TubeResonatorPluginParams::default()),
            dsp: TubeResonator::new(44100.0),
        }
    }
}

impl Default for TubeResonatorPluginParams {
    fn default() -> Self {
        let def = TubeResonatorParams::default();
        Self {
            editor_state: editor::default_state(),

            // MIDI 12 = C0 (16.35 Hz), 24 = C1 (32.70 Hz), 29 = F1 (43.65 Hz), 36 = C2 (65.41 Hz), 60 = C4 (261.63 Hz)
            tune_note: IntParam::new(
                "Tune",
                29,
                IntRange::Linear { min: 12, max: 60 },
            )
            .with_value_to_string(Arc::new(|note| {
                format!("{} ({:.2} Hz)", midi_note_name(note), midi_note_to_hz(note))
            })),

            fine_cents: FloatParam::new(
                "Fine",
                def.fine_cents,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ct")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            lr_detune: FloatParam::new(
                "L/R Detune",
                def.lr_detune,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),

            decay_ms: FloatParam::new(
                "Decay",
                def.decay_ms,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 5000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(15.0))
            .with_unit(" ms")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            damping: FloatParam::new(
                "Damping",
                def.damping,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),

            reflect: FloatParam::new(
                "Reflect",
                def.reflect,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),

            width: FloatParam::new(
                "Stereo Width",
                def.width,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),

            lfo_amount: FloatParam::new(
                "Pitch LFO Amount",
                def.lfo_amount,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),

            lfo_rate_hz: FloatParam::new(
                "Pitch LFO Rate",
                def.lfo_rate_hz,
                FloatRange::Skewed {
                    min: 0.01,
                    max: 30.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(10.0))
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            lfo_stereo_phase_deg: FloatParam::new(
                "Pitch LFO Phase",
                def.lfo_stereo_phase_deg,
                FloatRange::Linear {
                    min: 0.0,
                    max: 360.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" deg")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            decay_lfo_amount: FloatParam::new(
                "Decay LFO Amount",
                def.decay_lfo_amount,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),

            decay_lfo_rate_hz: FloatParam::new(
                "Decay LFO Rate",
                def.decay_lfo_rate_hz,
                FloatRange::Skewed {
                    min: 0.01,
                    max: 30.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(10.0))
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            filter_enabled: BoolParam::new("Filter On", def.filter_enabled),

            filter_freq_hz: FloatParam::new(
                "Filter Freq",
                def.filter_freq_hz,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.2),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(10.0))
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            filter_bandwidth_oct: FloatParam::new(
                "Filter Width",
                def.filter_bandwidth_oct,
                FloatRange::Linear { min: 0.5, max: 9.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" oct")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            dc_blocker_enabled: BoolParam::new("DC Filter", def.dc_blocker_enabled),

            direct: FloatParam::new(
                "Direct",
                def.direct,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),

            gain_db: FloatParam::new(
                "Gain",
                def.gain_db,
                FloatRange::Linear {
                    min: -24.0,
                    max: 24.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            dry_wet: FloatParam::new(
                "Dry/Wet",
                def.dry_wet,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),
        }
    }
}

impl Plugin for TubeResonatorPlugin {
    const NAME: &'static str = "Tube Resonator";
    const VENDOR: &'static str = "Jake Hukkanen";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.dsp.set_sample_rate(buffer_config.sample_rate as f64);
        self.dsp.reset();
        true
    }

    fn reset(&mut self) {
        self.dsp.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let num_samples = buffer.samples();
        let channels = buffer.as_slice();
        if channels.len() < 2 {
            return ProcessStatus::Normal;
        }

        let (left_slice, right_rest) = channels.split_at_mut(1);
        let left = &mut left_slice[0][..num_samples];
        let right = &mut right_rest[0][..num_samples];

        // Process in small sub-blocks so smoothed parameters update smoothly
        const SUB_BLOCK: usize = 32;
        let mut offset = 0;
        while offset < num_samples {
            let len = (num_samples - offset).min(SUB_BLOCK);
            let step = len as u32;

            let lfo_amt = self.params.lfo_amount.smoothed.next_step(step);
            let dsp_params = TubeResonatorParams {
                tune_hz: midi_note_to_hz(self.params.tune_note.value()),
                fine_cents: self.params.fine_cents.smoothed.next_step(step),
                lr_detune: self.params.lr_detune.smoothed.next_step(step),
                decay_ms: self.params.decay_ms.smoothed.next_step(step),
                damping: self.params.damping.smoothed.next_step(step),
                reflect: self.params.reflect.smoothed.next_step(step),
                width: self.params.width.smoothed.next_step(step),
                lfo_enabled: lfo_amt > 0.0,
                lfo_amount: lfo_amt,
                lfo_rate_hz: self.params.lfo_rate_hz.smoothed.next_step(step),
                lfo_stereo_phase_deg: self.params.lfo_stereo_phase_deg.smoothed.next_step(step),
                decay_lfo_amount: self.params.decay_lfo_amount.smoothed.next_step(step),
                decay_lfo_rate_hz: self.params.decay_lfo_rate_hz.smoothed.next_step(step),
                filter_enabled: self.params.filter_enabled.value(),
                filter_freq_hz: self.params.filter_freq_hz.smoothed.next_step(step),
                filter_bandwidth_oct: self.params.filter_bandwidth_oct.smoothed.next_step(step),
                dc_blocker_enabled: self.params.dc_blocker_enabled.value(),
                direct: self.params.direct.smoothed.next_step(step),
                gain_db: self.params.gain_db.smoothed.next_step(step),
                dry_wet: self.params.dry_wet.smoothed.next_step(step),
                safety_limiter: true,
            };

            self.dsp.process_block(
                &mut left[offset..offset + len],
                &mut right[offset..offset + len],
                &dsp_params,
            );
            offset += len;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for TubeResonatorPlugin {
    const CLAP_ID: &'static str = "com.jakehukkanen.tube-resonator";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Waveguide resonator and softening bandpass filter");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Filter,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for TubeResonatorPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"JH_TubeResonator";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Filter,
        Vst3SubCategory::Delay,
    ];
}

nih_export_clap!(TubeResonatorPlugin);
nih_export_vst3!(TubeResonatorPlugin);
