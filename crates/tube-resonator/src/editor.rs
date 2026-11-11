//! Warm dark-mode skeuomorphic Vizia GUI for `Tube Resonator`.
//!
//! Layout is split into two hardware-style sections:
//! - **Left Section (Primary Controls)**:
//!   - **Resonator**: `Tune` (default F1, resets to C1 on Cmd/Ctrl+Click or double-click), `Decay`, `Damping`
//!   - **Filter**: `Filter` toggle, `Freq`, `Width`
//!   - **Pitch LFO**: `Dirty` toggle, `Amount`, `Rate`, `Phase`
//!   - **Decay LFO**: `Amount`, `Rate`
//! - **Right Section (Secondary & Output Controls)**:
//!   - **Timbre & Detune**: `Fine`, `L/R Detune`, `Reflect`
//!   - **Output & Mix**: `DC Filter` toggle, `Stereo Width`, `Direct`, `Gain`, and `Dry / Wet` (rightmost)

use std::f32::consts::PI;
use std::sync::Arc;

use nih_plug::prelude::{Editor, Param};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::widgets::util::ModifiersExt;
use nih_plug_vizia::widgets::RawParamEvent;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};

use crate::TubeResonatorPluginParams;

const BASE_WIDTH: u32 = 920;
const BASE_HEIGHT: u32 = 410;
const DEFAULT_SCALE: f64 = 1.25;

/// Vertical pixels needed to sweep a rotary knob from 0% to 100%.
const KNOB_DRAG_PIXELS: f32 = 160.0;
/// Fine drag multiplier when holding Shift.
const KNOB_FINE_MULTIPLIER: f32 = 0.12;

#[derive(Lens)]
pub(crate) struct UiData {
    pub params: Arc<TubeResonatorPluginParams>,
}

impl Model for UiData {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new_with_default_scale_factor(|| (BASE_WIDTH, BASE_HEIGHT), DEFAULT_SCALE)
}

#[derive(Debug, Clone, Copy)]
struct DragState {
    start_y: f32,
    start_val: f32,
}

/// Skeuomorphic 3D rotary knob widget rendered via `femtovg` (`vizia::vg`).
pub struct RotaryKnob {
    param_base: ParamWidgetBase,
    drag_active: bool,
    drag_state: Option<DragState>,
    scrolled_lines: f32,
    bipolar: bool,
    accent_rgb: (u8, u8, u8),
    /// Optional custom normalized reset value (used so `Tune` starts at F1 by default,
    /// but resets to C1 on Cmd/Ctrl+Click or double-click).
    custom_reset_norm: Option<f32>,
}

impl RotaryKnob {
    pub fn new<'a, L, Params, P, FMap>(
        cx: &'a mut Context,
        params: L,
        params_to_param: FMap,
        bipolar: bool,
        accent_rgb: (u8, u8, u8),
        custom_reset_norm: Option<f32>,
    ) -> Handle<'a, Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
    {
        Self {
            param_base: ParamWidgetBase::new(cx, params.clone(), params_to_param),
            drag_active: false,
            drag_state: None,
            scrolled_lines: 0.0,
            bipolar,
            accent_rgb,
            custom_reset_norm,
        }
        .build(
            cx,
            ParamWidgetBase::build_view(params, params_to_param, move |cx, param_data| {
                let norm_lens =
                    param_data.make_lens(|param| param.modulated_normalized_value());
                Binding::new(cx, norm_lens, |cx, _| {
                    cx.needs_redraw();
                    Element::new(cx).hoverable(false);
                });
            }),
        )
    }

    #[inline]
    fn reset_normalized_value(&self) -> f32 {
        self.custom_reset_norm
            .unwrap_or_else(|| self.param_base.default_normalized_value())
    }

    fn reset_to_target(&mut self, cx: &mut EventContext) {
        if self.drag_active {
            self.drag_active = false;
            self.drag_state = None;
            cx.release();
            cx.set_active(false);
            self.param_base.end_set_parameter(cx);
        }
        let target = self.reset_normalized_value();
        self.param_base.begin_set_parameter(cx);
        self.param_base.set_normalized_value(cx, target);
        self.param_base.end_set_parameter(cx);
        cx.needs_redraw();
    }
}

impl View for RotaryKnob {
    fn element(&self) -> Option<&'static str> {
        Some("rotary-knob")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|raw_param_event, _| {
            if let RawParamEvent::ParametersChanged = raw_param_event {
                cx.needs_redraw();
            }
        });

        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left)
            | WindowEvent::MouseTripleClick(MouseButton::Left) => {
                // Support both Cmd+Click (macOS) and Ctrl+Click (Windows/Linux/macOS)
                if cx.modifiers().command() || cx.modifiers().contains(Modifiers::CTRL) {
                    self.reset_to_target(cx);
                } else {
                    self.drag_active = true;
                    cx.capture();
                    cx.focus();
                    cx.set_active(true);
                    self.param_base.begin_set_parameter(cx);
                    self.drag_state = Some(DragState {
                        start_y: cx.mouse().cursory,
                        start_val: self.param_base.unmodulated_normalized_value(),
                    });
                    cx.needs_redraw();
                }
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left)
            | WindowEvent::MouseDown(MouseButton::Right)
            | WindowEvent::MouseDoubleClick(MouseButton::Right)
            | WindowEvent::MouseTripleClick(MouseButton::Right) => {
                self.reset_to_target(cx);
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.drag_active {
                    self.drag_active = false;
                    self.drag_state = None;
                    cx.release();
                    cx.set_active(false);
                    self.param_base.end_set_parameter(cx);
                    cx.needs_redraw();
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(_x, y) => {
                if self.drag_active {
                    let state = self.drag_state.get_or_insert(DragState {
                        start_y: *y,
                        start_val: self.param_base.unmodulated_normalized_value(),
                    });

                    let sensitivity = if cx.modifiers().shift() {
                        KNOB_FINE_MULTIPLIER
                    } else {
                        1.0
                    };

                    // Upward drag (smaller y) increases value
                    let dy = (state.start_y - *y) / (KNOB_DRAG_PIXELS * cx.scale_factor());
                    let new_val = (state.start_val + dy * sensitivity).clamp(0.0, 1.0);
                    self.param_base.set_normalized_value(cx, new_val);

                    state.start_y = *y;
                    state.start_val = new_val;

                    cx.needs_redraw();
                    meta.consume();
                }
            }
            WindowEvent::MouseScroll(_scroll_x, scroll_y) => {
                self.scrolled_lines += scroll_y;
                if self.scrolled_lines.abs() >= 0.5 {
                    let finer = cx.modifiers().shift();
                    if !self.drag_active {
                        self.param_base.begin_set_parameter(cx);
                    }

                    let mut current = self.param_base.unmodulated_normalized_value();
                    if self.param_base.step_count().is_some() {
                        while self.scrolled_lines >= 0.5 {
                            current = self.param_base.next_normalized_step(current, finer);
                            self.param_base.set_normalized_value(cx, current);
                            self.scrolled_lines -= 1.0;
                        }
                        while self.scrolled_lines <= -0.5 {
                            current = self.param_base.previous_normalized_step(current, finer);
                            self.param_base.set_normalized_value(cx, current);
                            self.scrolled_lines += 1.0;
                        }
                    } else {
                        let step = if finer { 0.004 } else { 0.02 };
                        let delta = self.scrolled_lines * step;
                        current = (current + delta).clamp(0.0, 1.0);
                        self.param_base.set_normalized_value(cx, current);
                        self.scrolled_lines = 0.0;
                    }

                    if !self.drag_active {
                        self.param_base.end_set_parameter(cx);
                    }
                    cx.needs_redraw();
                }
                meta.consume();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }

        let dpx = cx.scale_factor();
        let center_x = bounds.x + bounds.w * 0.5;
        let center_y = bounds.y + bounds.h * 0.52;
        let max_r = (bounds.w.min(bounds.h) * 0.5) - 2.0 * dpx;

        let arc_r = max_r * 0.90;
        let body_r = max_r * 0.70;
        let cap_r = max_r * 0.57;

        let norm = self.param_base.modulated_normalized_value().clamp(0.0, 1.0);

        let start_angle = 0.75 * PI;
        let sweep_angle = 1.50 * PI;
        let end_angle = start_angle + sweep_angle;
        let val_angle = start_angle + norm * sweep_angle;

        // 1. Warm bronze/ivory tick marks around the dial
        let num_ticks = 11;
        for i in 0..num_ticks {
            let t = i as f32 / (num_ticks - 1) as f32;
            let a = start_angle + t * sweep_angle;
            let r_in = max_r * 0.94;
            let r_out = max_r * 1.02;
            let mut tick = vg::Path::new();
            tick.move_to(center_x + a.cos() * r_in, center_y + a.sin() * r_in);
            tick.line_to(center_x + a.cos() * r_out, center_y + a.sin() * r_out);
            let is_major = i == 0 || i == num_ticks - 1 || i == num_ticks / 2;
            let mut paint = if is_major {
                vg::Paint::color(vg::Color::rgba(218, 198, 175, 215))
            } else {
                vg::Paint::color(vg::Color::rgba(155, 138, 120, 155))
            };
            paint.set_line_width((if is_major { 1.4 } else { 1.0 }) * dpx);
            canvas.stroke_path(&tick, &paint);
        }

        // 2. Warm dark recessed LED value track
        let mut track_path = vg::Path::new();
        track_path.arc(center_x, center_y, arc_r, start_angle, end_angle, vg::Solidity::Hole);
        let mut track_paint = vg::Paint::color(vg::Color::rgba(30, 25, 21, 245));
        track_paint.set_line_width(3.4 * dpx);
        track_paint.set_line_cap(vg::LineCap::Round);
        canvas.stroke_path(&track_path, &track_paint);

        // 3. Illuminated LED arc
        let (ar, ag, ab) = self.accent_rgb;
        let (arc_a0, arc_a1) = if self.bipolar {
            let mid_angle = start_angle + 0.5 * sweep_angle;
            if val_angle >= mid_angle {
                (mid_angle, val_angle)
            } else {
                (val_angle, mid_angle)
            }
        } else {
            (start_angle, val_angle)
        };

        if (arc_a1 - arc_a0).abs() > 0.015 {
            // Soft outer glow
            let mut glow_path = vg::Path::new();
            glow_path.arc(center_x, center_y, arc_r, arc_a0, arc_a1, vg::Solidity::Hole);
            let mut glow_paint = vg::Paint::color(vg::Color::rgba(ar, ag, ab, 80));
            glow_paint.set_line_width(6.5 * dpx);
            glow_paint.set_line_cap(vg::LineCap::Round);
            canvas.stroke_path(&glow_path, &glow_paint);

            // Crisp core LED arc
            let mut val_path = vg::Path::new();
            val_path.arc(center_x, center_y, arc_r, arc_a0, arc_a1, vg::Solidity::Hole);
            let mut val_paint = vg::Paint::color(vg::Color::rgba(ar, ag, ab, 250));
            val_paint.set_line_width(2.8 * dpx);
            val_paint.set_line_cap(vg::LineCap::Round);
            canvas.stroke_path(&val_path, &val_paint);
        }

        // 4. Cast drop shadow beneath the 3D knob body
        let mut shadow_path = vg::Path::new();
        shadow_path.circle(center_x, center_y + 3.2 * dpx, body_r * 1.03);
        let shadow_paint = vg::Paint::radial_gradient(
            center_x,
            center_y + 3.2 * dpx,
            body_r * 0.5,
            body_r * 1.12,
            vg::Color::rgba(14, 11, 8, 200),
            vg::Color::rgba(14, 11, 8, 0),
        );
        canvas.fill_path(&shadow_path, &shadow_paint);

        // 5. Warm bronze/taupe knurled outer skirt
        let mut body_path = vg::Path::new();
        body_path.circle(center_x, center_y, body_r);
        let body_paint = vg::Paint::linear_gradient(
            center_x,
            center_y - body_r,
            center_x,
            center_y + body_r,
            vg::Color::rgb(112, 98, 86),
            vg::Color::rgb(50, 42, 36),
        );
        canvas.fill_path(&body_path, &body_paint);

        let mut rim_stroke = vg::Paint::color(vg::Color::rgba(24, 20, 17, 235));
        rim_stroke.set_line_width(1.3 * dpx);
        canvas.stroke_path(&body_path, &rim_stroke);

        // Subtle knurl serrations around the skirt
        let serrations = 24;
        for i in 0..serrations {
            let a = (i as f32 / serrations as f32) * 2.0 * PI + norm * 1.2;
            let r0 = cap_r * 1.02;
            let r1 = body_r * 0.96;
            let mut groove = vg::Path::new();
            groove.move_to(center_x + a.cos() * r0, center_y + a.sin() * r0);
            groove.line_to(center_x + a.cos() * r1, center_y + a.sin() * r1);
            let mut gpaint = vg::Paint::color(vg::Color::rgba(32, 26, 22, 135));
            gpaint.set_line_width(1.0 * dpx);
            canvas.stroke_path(&groove, &gpaint);
        }

        // 6. Top warm brushed-bronze metallic cap with radial specular highlight
        let mut cap_path = vg::Path::new();
        cap_path.circle(center_x, center_y, cap_r);
        let cap_paint = vg::Paint::radial_gradient(
            center_x - cap_r * 0.3,
            center_y - cap_r * 0.35,
            cap_r * 0.08,
            cap_r * 1.1,
            vg::Color::rgb(138, 122, 108),
            vg::Color::rgb(62, 53, 46),
        );
        canvas.fill_path(&cap_path, &cap_paint);

        // Top bevel highlight ring
        let mut cap_ring = vg::Paint::linear_gradient(
            center_x,
            center_y - cap_r,
            center_x,
            center_y + cap_r,
            vg::Color::rgba(225, 204, 182, 140),
            vg::Color::rgba(28, 23, 19, 200),
        );
        cap_ring.set_line_width(1.1 * dpx);
        canvas.stroke_path(&cap_path, &cap_ring);

        // 7. Warm cream pointer indicator line on the cap
        let p_in = cap_r * 0.24;
        let p_out = cap_r * 0.88;
        let px0 = center_x + val_angle.cos() * p_in;
        let py0 = center_y + val_angle.sin() * p_in;
        let px1 = center_x + val_angle.cos() * p_out;
        let py1 = center_y + val_angle.sin() * p_out;

        let mut ptr_shadow = vg::Path::new();
        ptr_shadow.move_to(px0, py0 + 1.0 * dpx);
        ptr_shadow.line_to(px1, py1 + 1.0 * dpx);
        let mut ps_paint = vg::Paint::color(vg::Color::rgba(0, 0, 0, 195));
        ps_paint.set_line_width(3.0 * dpx);
        ps_paint.set_line_cap(vg::LineCap::Round);
        canvas.stroke_path(&ptr_shadow, &ps_paint);

        let mut ptr_path = vg::Path::new();
        ptr_path.move_to(px0, py0);
        ptr_path.line_to(px1, py1);
        let mut ptr_paint = vg::Paint::color(vg::Color::rgb(255, 244, 228));
        ptr_paint.set_line_width(2.1 * dpx);
        ptr_paint.set_line_cap(vg::LineCap::Round);
        canvas.stroke_path(&ptr_path, &ptr_paint);
    }
}

/// Skeuomorphic illuminated hardware push-button / LED switch for boolean parameters.
pub struct LedToggleSwitch {
    param_base: ParamWidgetBase,
    accent_rgb: (u8, u8, u8),
}

impl LedToggleSwitch {
    pub fn new<'a, L, Params, P, FMap>(
        cx: &'a mut Context,
        params: L,
        params_to_param: FMap,
        label_text: &'static str,
        accent_rgb: (u8, u8, u8),
    ) -> Handle<'a, Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
    {
        Self {
            param_base: ParamWidgetBase::new(cx, params.clone(), params_to_param),
            accent_rgb,
        }
        .build(
            cx,
            ParamWidgetBase::build_view(params.clone(), params_to_param, move |cx, param_data| {
                let is_on_lens =
                    param_data.make_lens(|param| param.modulated_normalized_value() >= 0.5);
                Binding::new(cx, is_on_lens, move |cx, _| {
                    cx.needs_redraw();
                    HStack::new(cx, |cx| {
                        Element::new(cx)
                            .width(Pixels(16.0))
                            .height(Stretch(1.0))
                            .hoverable(false);
                        Label::new(cx, label_text)
                            .class("switch-label")
                            .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                            .font_weight(FontWeightKeyword::Bold)
                            .hoverable(false);
                    })
                    .child_top(Stretch(1.0))
                    .child_bottom(Stretch(1.0))
                    .child_left(Pixels(7.0))
                    .child_right(Pixels(9.0))
                    .col_between(Pixels(4.0))
                    .hoverable(false);
                });
            }),
        )
        .checked(ParamWidgetBase::make_lens(
            params,
            params_to_param,
            |param| param.modulated_normalized_value() >= 0.5,
        ))
    }

    fn toggle(&self, cx: &mut EventContext) {
        let current = self.param_base.unmodulated_normalized_value();
        let next = if current >= 0.5 { 0.0 } else { 1.0 };
        self.param_base.begin_set_parameter(cx);
        self.param_base.set_normalized_value(cx, next);
        self.param_base.end_set_parameter(cx);
        cx.needs_redraw();
    }
}

impl View for LedToggleSwitch {
    fn element(&self) -> Option<&'static str> {
        Some("led-toggle")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|raw_param_event, _| {
            if let RawParamEvent::ParametersChanged = raw_param_event {
                cx.set_checked(self.param_base.modulated_normalized_value() >= 0.5);
                cx.needs_redraw();
            }
        });

        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left)
            | WindowEvent::MouseDoubleClick(MouseButton::Left)
            | WindowEvent::MouseTripleClick(MouseButton::Left) => {
                if cx.modifiers().command() || cx.modifiers().contains(Modifiers::CTRL) {
                    let def = self.param_base.default_normalized_value();
                    self.param_base.begin_set_parameter(cx);
                    self.param_base.set_normalized_value(cx, def);
                    self.param_base.end_set_parameter(cx);
                    cx.needs_redraw();
                } else {
                    self.toggle(cx);
                }
                meta.consume();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }

        let dpx = cx.scale_factor();
        let is_on = self.param_base.modulated_normalized_value() >= 0.5;
        let (ar, ag, ab) = self.accent_rgb;

        let r = 4.0 * dpx;
        let mut bg = vg::Path::new();
        bg.rounded_rect(bounds.x, bounds.y, bounds.w, bounds.h, r);

        let housing_paint = if is_on {
            vg::Paint::linear_gradient(
                bounds.x,
                bounds.y,
                bounds.x,
                bounds.y + bounds.h,
                vg::Color::rgb(82, 70, 60),
                vg::Color::rgb(52, 44, 38),
            )
        } else {
            vg::Paint::linear_gradient(
                bounds.x,
                bounds.y,
                bounds.x,
                bounds.y + bounds.h,
                vg::Color::rgb(54, 46, 40),
                vg::Color::rgb(38, 32, 28),
            )
        };
        canvas.fill_path(&bg, &housing_paint);

        let mut border = vg::Paint::color(if is_on {
            vg::Color::rgba(ar, ag, ab, 175)
        } else {
            vg::Color::rgba(24, 20, 17, 235)
        });
        border.set_line_width(1.1 * dpx);
        canvas.stroke_path(&bg, &border);

        let led_x = bounds.x + 13.0 * dpx;
        let led_y = bounds.y + bounds.h * 0.5;
        let led_r = 4.0 * dpx;

        if is_on {
            let mut glow = vg::Path::new();
            glow.circle(led_x, led_y, led_r * 2.2);
            let glow_paint = vg::Paint::radial_gradient(
                led_x,
                led_y,
                led_r * 0.2,
                led_r * 2.2,
                vg::Color::rgba(ar, ag, ab, 165),
                vg::Color::rgba(ar, ag, ab, 0),
            );
            canvas.fill_path(&glow, &glow_paint);
        }

        let mut lamp = vg::Path::new();
        lamp.circle(led_x, led_y, led_r);
        let lamp_paint = if is_on {
            vg::Paint::radial_gradient(
                led_x - led_r * 0.25,
                led_y - led_r * 0.25,
                led_r * 0.1,
                led_r,
                vg::Color::rgb(255, 250, 240),
                vg::Color::rgb(ar, ag, ab),
            )
        } else {
            vg::Paint::radial_gradient(
                led_x - led_r * 0.2,
                led_y - led_r * 0.2,
                led_r * 0.1,
                led_r,
                vg::Color::rgb(92, 80, 70),
                vg::Color::rgb(44, 38, 33),
            )
        };
        canvas.fill_path(&lamp, &lamp_paint);

        let mut lamp_rim = vg::Paint::color(vg::Color::rgba(12, 10, 8, 225));
        lamp_rim.set_line_width(1.0 * dpx);
        canvas.stroke_path(&lamp, &lamp_rim);
    }
}

/// Build a complete rotary knob cell with title, 3D skeuomorphic dial, and digital readout screen.
fn build_knob_cell<L, Params, P, FMap>(
    cx: &mut Context,
    params: L,
    params_to_param: FMap,
    title: &'static str,
    bipolar: bool,
    accent_rgb: (u8, u8, u8),
    large: bool,
    custom_reset_norm: Option<f32>,
) where
    L: Lens<Target = Params> + Clone,
    Params: 'static,
    P: Param + 'static,
    FMap: Fn(&Params) -> &P + Copy + 'static,
{
    let cell_w = if large { 90.0 } else { 78.0 };
    let knob_sz = if large { 66.0 } else { 56.0 };

    VStack::new(cx, move |cx| {
        Label::new(cx, title)
            .class("knob-title")
            .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
            .font_weight(FontWeightKeyword::Bold)
            .width(Stretch(1.0))
            .height(Pixels(18.0))
            .child_space(Stretch(1.0));

        RotaryKnob::new(
            cx,
            params.clone(),
            params_to_param,
            bipolar,
            accent_rgb,
            custom_reset_norm,
        )
        .width(Pixels(knob_sz))
        .height(Pixels(knob_sz))
        .left(Stretch(1.0))
        .right(Stretch(1.0));

        HStack::new(
            cx,
            ParamWidgetBase::build_view(params, params_to_param, move |cx, param_data| {
                let readout_lens = param_data.make_lens(|param| {
                    param.normalized_value_to_string(param.modulated_normalized_value(), true)
                });
                Label::new(cx, readout_lens)
                    .class("knob-readout-text")
                    .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                    .font_weight(FontWeightKeyword::Bold)
                    .width(Stretch(1.0))
                    .height(Stretch(1.0))
                    .child_space(Stretch(1.0));
            }),
        )
        .class("knob-readout-box")
        .width(Pixels(cell_w - 6.0))
        .height(Pixels(22.0))
        .left(Stretch(1.0))
        .right(Stretch(1.0));
    })
    .class("knob-cell")
    .width(Pixels(cell_w))
    .height(Auto)
    .row_between(Pixels(3.0));
}

/// Decorative hardware rack screw head.
fn build_rack_screw(cx: &mut Context) {
    Element::new(cx).class("rack-screw");
}

pub(crate) fn create(
    params: Arc<TubeResonatorPluginParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Builtin, move |cx, _| {
        assets::register_noto_sans_bold(cx);
        cx.set_default_font(&[assets::NOTO_SANS]);

        let _ = cx.add_stylesheet(include_style!("src/theme.css"));

        UiData {
            params: params.clone(),
        }
        .build(cx);

        // Warm analog accent colors (amber, warm honey-gold, and coral/terracotta)
        let amber = (245, 179, 73);
        let gold = (242, 203, 97);
        let coral = (235, 140, 85);

        // Normalized value for C1 (MIDI 24) in Tune's 12..=60 range: (24 - 12) / (60 - 12) = 0.25
        let tune_c1_norm = Some((24.0 - 12.0) / (60.0 - 12.0));

        VStack::new(cx, move |cx| {
            // =========================================================================
            // TOP HARDWARE HEADER BAR
            // =========================================================================
            HStack::new(cx, |cx| {
                build_rack_screw(cx);

                HStack::new(cx, |cx| {
                    Element::new(cx).class("brand-pill");
                    Label::new(cx, "TUBE RESONATOR")
                        .class("header-title")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold);
                })
                .col_between(Pixels(10.0))
                .child_top(Stretch(1.0))
                .child_bottom(Stretch(1.0))
                .width(Auto);

                Element::new(cx).width(Stretch(1.0));

                build_rack_screw(cx);
            })
            .class("header-bar")
            .width(Stretch(1.0))
            .height(Pixels(38.0))
            .child_left(Pixels(12.0))
            .child_right(Pixels(12.0))
            .child_top(Stretch(1.0))
            .child_bottom(Stretch(1.0))
            .col_between(Pixels(12.0));

            // =========================================================================
            // MAIN TWO-SECTION SPLIT CHASSIS
            // =========================================================================
            HStack::new(cx, move |cx| {
                // ---------------------------------------------------------------------
                // LEFT SECTION: Resonator, Filter, Pitch LFO & Decay LFO
                // ---------------------------------------------------------------------
                VStack::new(cx, move |cx| {
                    // Top Row: RESONATOR (3 knobs) + FILTER (2 knobs)
                    HStack::new(cx, move |cx| {
                        // Module 1: Resonator (Tune, Decay, Radius)
                        VStack::new(cx, move |cx| {
                            HStack::new(cx, |cx| {
                                Label::new(cx, "RESONATOR")
                                    .class("module-title")
                                    .font_weight(FontWeightKeyword::Bold);
                            })
                            .class("module-header")
                            .width(Stretch(1.0))
                            .height(Pixels(26.0));

                            HStack::new(cx, move |cx| {
                                build_knob_cell(
                                    cx,
                                    UiData::params,
                                    |p| &p.tune_note,
                                    "TUNE",
                                    false,
                                    amber,
                                    false,
                                    tune_c1_norm,
                                );
                                build_knob_cell(
                                    cx,
                                    UiData::params,
                                    |p| &p.decay_ms,
                                    "DECAY",
                                    false,
                                    amber,
                                    false,
                                    None,
                                );
                                build_knob_cell(
                                    cx,
                                    UiData::params,
                                    |p| &p.damping,
                                    "DAMPING",
                                    false,
                                    amber,
                                    false,
                                    None,
                                );
                            })
                            .class("knob-row")
                            .width(Stretch(1.0))
                            .height(Stretch(1.0));
                        })
                        .class("hardware-module")
                        .width(Stretch(1.52))
                        .height(Stretch(1.0));

                        // Module 2: Filter (Filter On toggle, Freq, Width)
                        VStack::new(cx, |cx| {
                            HStack::new(cx, |cx| {
                                LedToggleSwitch::new(
                                    cx,
                                    UiData::params,
                                    |p| &p.filter_enabled,
                                    "FILTER",
                                    gold,
                                )
                                .height(Pixels(22.0))
                                .width(Pixels(82.0));
                            })
                            .class("module-header")
                            .width(Stretch(1.0))
                            .height(Pixels(26.0));

                            HStack::new(cx, |cx| {
                                build_knob_cell(
                                    cx,
                                    UiData::params,
                                    |p| &p.filter_freq_hz,
                                    "FREQ",
                                    false,
                                    gold,
                                    false,
                                    None,
                                );
                                build_knob_cell(
                                    cx,
                                    UiData::params,
                                    |p| &p.filter_bandwidth_oct,
                                    "WIDTH",
                                    false,
                                    gold,
                                    false,
                                    None,
                                );
                            })
                            .class("knob-row")
                            .width(Stretch(1.0))
                            .height(Stretch(1.0));
                        })
                        .class("hardware-module")
                        .width(Stretch(1.0))
                        .height(Stretch(1.0));
                    })
                    .col_between(Pixels(10.0))
                    .width(Stretch(1.0))
                    .height(Stretch(1.0));

                    // Bottom Row: PITCH LFO (3 knobs) + DECAY LFO (2 knobs)
                    HStack::new(cx, |cx| {
                        // Module 3: Pitch LFO (Dirty toggle, Amount, Rate, Phase)
                        VStack::new(cx, |cx| {
                            HStack::new(cx, |cx| {
                                Label::new(cx, "PITCH LFO")
                                    .class("module-title")
                                    .font_weight(FontWeightKeyword::Bold);
                                Element::new(cx)
                                    .width(Stretch(1.0))
                                    .height(Stretch(1.0))
                                    .hoverable(false);
                                LedToggleSwitch::new(
                                    cx,
                                    UiData::params,
                                    |p| &p.lfo_dirty,
                                    "DIRTY",
                                    coral,
                                )
                                .height(Pixels(22.0))
                                .width(Pixels(78.0));
                            })
                            .class("module-header")
                            .width(Stretch(1.0))
                            .height(Pixels(26.0));

                            HStack::new(cx, |cx| {
                                build_knob_cell(
                                    cx,
                                    UiData::params,
                                    |p| &p.lfo_amount,
                                    "AMOUNT",
                                    false,
                                    coral,
                                    false,
                                    None,
                                );
                                build_knob_cell(
                                    cx,
                                    UiData::params,
                                    |p| &p.lfo_rate_hz,
                                    "RATE",
                                    false,
                                    coral,
                                    false,
                                    None,
                                );
                                build_knob_cell(
                                    cx,
                                    UiData::params,
                                    |p| &p.lfo_stereo_phase_deg,
                                    "PHASE",
                                    false,
                                    coral,
                                    false,
                                    None,
                                );
                            })
                            .class("knob-row")
                            .width(Stretch(1.0))
                            .height(Stretch(1.0));
                        })
                        .class("hardware-module")
                        .width(Stretch(1.52))
                        .height(Stretch(1.0));

                        // Module 4: Decay LFO (Amount, Rate)
                        VStack::new(cx, |cx| {
                            HStack::new(cx, |cx| {
                                Label::new(cx, "DECAY LFO")
                                    .class("module-title")
                                    .font_weight(FontWeightKeyword::Bold);
                            })
                            .class("module-header")
                            .width(Stretch(1.0))
                            .height(Pixels(26.0));

                            HStack::new(cx, |cx| {
                                build_knob_cell(
                                    cx,
                                    UiData::params,
                                    |p| &p.decay_lfo_amount,
                                    "AMOUNT",
                                    false,
                                    coral,
                                    false,
                                    None,
                                );
                                build_knob_cell(
                                    cx,
                                    UiData::params,
                                    |p| &p.decay_lfo_rate_hz,
                                    "RATE",
                                    false,
                                    coral,
                                    false,
                                    None,
                                );
                            })
                            .class("knob-row")
                            .width(Stretch(1.0))
                            .height(Stretch(1.0));
                        })
                        .class("hardware-module")
                        .width(Stretch(1.0))
                        .height(Stretch(1.0));
                    })
                    .col_between(Pixels(10.0))
                    .width(Stretch(1.0))
                    .height(Stretch(1.0));
                })
                .class("section-panel")
                .width(Stretch(1.24))
                .height(Stretch(1.0))
                .row_between(Pixels(10.0));

                // ---------------------------------------------------------------------
                // RIGHT SECTION: Secondary & Output Controls (Dry/Wet rightmost)
                // ---------------------------------------------------------------------
                VStack::new(cx, |cx| {
                    // Top Row: Fine, L/R Detune, Reflect
                    VStack::new(cx, |cx| {
                        HStack::new(cx, |cx| {
                            Label::new(cx, "TIMBRE & DETUNE")
                                .class("module-title")
                                .font_weight(FontWeightKeyword::Bold);
                        })
                        .class("module-header")
                        .width(Stretch(1.0))
                        .height(Pixels(26.0));

                        HStack::new(cx, |cx| {
                            build_knob_cell(
                                cx,
                                UiData::params,
                                |p| &p.fine_cents,
                                "FINE",
                                true,
                                gold,
                                false,
                                None,
                            );
                            build_knob_cell(
                                cx,
                                UiData::params,
                                |p| &p.lr_detune,
                                "L/R DETUNE",
                                true,
                                gold,
                                false,
                                None,
                            );
                            build_knob_cell(
                                cx,
                                UiData::params,
                                |p| &p.reflect,
                                "REFLECT",
                                false,
                                amber,
                                false,
                                None,
                            );
                        })
                        .class("knob-row")
                        .width(Stretch(1.0))
                        .height(Stretch(1.0));
                    })
                    .class("hardware-module")
                    .width(Stretch(1.0))
                    .height(Stretch(1.0));

                    // Bottom Row: Stereo Width, Direct, Gain, Dry/Wet (rightmost) + DC Filter
                    VStack::new(cx, |cx| {
                        HStack::new(cx, |cx| {
                            Label::new(cx, "OUTPUT & MIX")
                                .class("module-title")
                                .font_weight(FontWeightKeyword::Bold);

                            Element::new(cx).width(Stretch(1.0));

                            LedToggleSwitch::new(
                                cx,
                                UiData::params,
                                |p| &p.dc_blocker_enabled,
                                "DC FILTER",
                                amber,
                            )
                            .height(Pixels(22.0))
                            .width(Pixels(98.0));
                        })
                        .class("module-header")
                        .width(Stretch(1.0))
                        .height(Pixels(26.0));

                        HStack::new(cx, |cx| {
                            build_knob_cell(
                                cx,
                                UiData::params,
                                |p| &p.width,
                                "STEREO WIDTH",
                                false,
                                gold,
                                false,
                                None,
                            );
                            build_knob_cell(
                                cx,
                                UiData::params,
                                |p| &p.direct,
                                "DIRECT",
                                false,
                                gold,
                                false,
                                None,
                            );
                            build_knob_cell(
                                cx,
                                UiData::params,
                                |p| &p.gain_db,
                                "GAIN",
                                true,
                                amber,
                                false,
                                None,
                            );
                            // Rightmost knob: DRY / WET
                            build_knob_cell(
                                cx,
                                UiData::params,
                                |p| &p.dry_wet,
                                "DRY / WET",
                                false,
                                amber,
                                true,
                                None,
                            );
                        })
                        .class("knob-row")
                        .width(Stretch(1.0))
                        .height(Stretch(1.0));
                    })
                    .class("hardware-module")
                    .width(Stretch(1.0))
                    .height(Stretch(1.0));
                })
                .class("section-panel")
                .width(Stretch(1.0))
                .height(Stretch(1.0))
                .row_between(Pixels(10.0));
            })
            .class("main-chassis")
            .width(Stretch(1.0))
            .height(Stretch(1.0))
            .col_between(Pixels(12.0));
        })
        .class("plugin-root")
        .width(Stretch(1.0))
        .height(Stretch(1.0));
    })
}
