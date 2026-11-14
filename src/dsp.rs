use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Params {
    pub pitch: f32,
    pub bass: f32,
    pub body: f32,
    pub air: f32,
    pub drive: f32,
    pub robot: f32,
    pub ring_hz: f32,
    pub bits: f32,
    pub vibrato: f32,
    pub vibrato_hz: f32,
    pub delay: f32,
    pub delay_ms: f32,
    pub feedback: f32,
    pub reverb: f32,
    pub room: f32,
    pub lowpass: f32,
    pub gate: f32,
    pub threshold: f32,
    pub ratio: f32,
    pub attack: f32,
    pub release: f32,
    pub deess: f32,
    pub input: f32,
    pub output: f32,
    pub mix: f32,
    pub board: f32,
}
impl Default for Params {
    fn default() -> Self {
        Self {
            pitch: 0.,
            bass: 0.,
            body: 0.,
            air: 0.,
            drive: 0.,
            robot: 0.,
            ring_hz: 65.,
            bits: 16.,
            vibrato: 0.,
            vibrato_hz: 5.,
            delay: 0.,
            delay_ms: 220.,
            feedback: 0.3,
            reverb: 0.,
            room: 0.5,
            lowpass: 18000.,
            gate: -52.,
            threshold: -18.,
            ratio: 2.5,
            attack: 8.,
            release: 120.,
            deess: 0.15,
            input: 0.,
            output: 0.,
            mix: 1.,
            board: 0.55,
        }
    }
}
impl Params {
    pub fn sanitize(&mut self) {
        macro_rules! clamp { ($($f:ident: $a:expr, $b:expr);* $(;)?) => { $(if !self.$f.is_finite() { self.$f = Self::default().$f; } self.$f = self.$f.clamp($a, $b);)* }; }
        clamp!(pitch: -12.,12.; bass: -12.,12.; body: -12.,12.; air: -12.,12.; drive: 0.,1.; robot: 0.,1.; ring_hz: 10.,600.; bits: 4.,16.; vibrato: 0.,1.; vibrato_hz: 0.5,12.; delay: 0.,0.85; delay_ms: 20.,650.; feedback: 0.,0.8; reverb: 0.,0.8; room: 0.,1.; lowpass: 500.,20000.; gate: -80.,-15.; threshold: -40.,0.; ratio: 1.,12.; attack: 1.,80.; release: 20.,600.; deess: 0.,1.; input: -24.,24.; output: -24.,12.; mix: 0.,1.; board: 0.,1.);
    }
}
pub fn gain(db: f32) -> f32 {
    10_f32.powf(db / 20.)
}

#[derive(Clone, Copy, Default)]
struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
    z: [f32; 2],
}
impl Biquad {
    fn configure(&mut self, hz: f32, db: f32, kind: u8, sr: f32) {
        let w = TAU * hz.min(sr * 0.45) / sr;
        let c = w.cos();
        let s = w.sin();
        let a = 10_f32.powf(db / 40.);
        let alpha = s / 1.4142135;
        let beta = 2. * a.sqrt() * alpha;
        let (b0, b1, b2, a0, a1, a2) = match kind {
            0 => (
                a * ((a + 1.) - (a - 1.) * c + beta),
                2. * a * ((a - 1.) - (a + 1.) * c),
                a * ((a + 1.) - (a - 1.) * c - beta),
                (a + 1.) + (a - 1.) * c + beta,
                -2. * ((a - 1.) + (a + 1.) * c),
                (a + 1.) + (a - 1.) * c - beta,
            ),
            1 => (
                1. + alpha * a,
                -2. * c,
                1. - alpha * a,
                1. + alpha / a,
                -2. * c,
                1. - alpha / a,
            ),
            _ => (
                a * ((a + 1.) + (a - 1.) * c + beta),
                -2. * a * ((a - 1.) + (a + 1.) * c),
                a * ((a + 1.) + (a - 1.) * c - beta),
                (a + 1.) - (a - 1.) * c + beta,
                2. * ((a - 1.) - (a + 1.) * c),
                (a + 1.) - (a - 1.) * c - beta,
            ),
        };
        self.b = [b0 / a0, b1 / a0, b2 / a0];
        self.a = [a1 / a0, a2 / a0];
    }
    fn tick(&mut self, x: f32) -> f32 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}
struct Comb {
    buf: Vec<f32>,
    pos: usize,
    damp: f32,
}
impl Comb {
    fn new(n: usize) -> Self {
        Self {
            buf: vec![0.; n],
            pos: 0,
            damp: 0.,
        }
    }
    fn tick(&mut self, x: f32, feedback: f32) -> f32 {
        let y = self.buf[self.pos];
        self.damp = y * 0.75 + self.damp * 0.25;
        self.buf[self.pos] = x + self.damp * feedback;
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
        y
    }
}
pub struct Dsp {
    pub sr: f32,
    p: Params,
    eq: [Biquad; 3],
    pitch_buf: Vec<f32>,
    write: usize,
    phase: f32,
    echo: Vec<f32>,
    echo_pos: usize,
    combs: [Comb; 4],
    osc: f32,
    vib: f32,
    dc_x: f32,
    dc_y: f32,
    envelope: f32,
    gate_gain: f32,
    comp_gain: f32,
    lp: f32,
    de_lp: f32,
    in_gain: f32,
    out_gain: f32,
    attack: f32,
    release: f32,
    lp_alpha: f32,
    gate_level: f32,
    comp_level: f32,
    pitch_ratio: f32,
    configured: bool,
    vibrato_step: f32,
    ring_step: f32,
    pitch_span: f32,
    pitch_step: f32,
    comp_exponent: f32,
    drive: f32,
    drive_norm: f32,
    bit_levels: f32,
    echo_delay: usize,
    room_feedback: f32,
}
impl Dsp {
    pub fn new(sr: f32) -> Self {
        let mut s = Self {
            sr,
            p: Params::default(),
            eq: [Biquad::default(); 3],
            pitch_buf: vec![0.; (sr * 0.15) as usize],
            write: 0,
            phase: 0.,
            echo: vec![0.; (sr * 0.7) as usize + 2],
            echo_pos: 0,
            combs: [1557, 1617, 1491, 1422].map(|n| Comb::new((n as f32 * sr / 44100.) as usize)),
            osc: 0.,
            vib: 0.,
            dc_x: 0.,
            dc_y: 0.,
            envelope: 0.,
            gate_gain: 0.,
            comp_gain: 1.,
            lp: 0.,
            de_lp: 0.,
            in_gain: 1.,
            out_gain: 1.,
            attack: 0.,
            release: 0.,
            lp_alpha: 0.,
            gate_level: 0.,
            comp_level: 0.,
            pitch_ratio: 1.,
            configured: false,
            vibrato_step: 0.,
            ring_step: 0.,
            pitch_span: sr * 0.042,
            pitch_step: 0.,
            comp_exponent: 0.,
            drive: 1.,
            drive_norm: 1.,
            bit_levels: 1.,
            echo_delay: 0,
            room_feedback: 0.,
        };
        s.configure(Params::default());
        s
    }
    pub fn configure(&mut self, mut p: Params) {
        // The audio callback submits controls every block. Rebuild coefficients
        // only when they change; the first default configuration must still run.
        if self.configured && p == self.p {
            return;
        }
        p.sanitize();
        if self.configured && p == self.p {
            return;
        }
        self.configured = true;
        self.p = p;
        self.eq[0].configure(160., p.bass, 0, self.sr);
        self.eq[1].configure(1100., p.body, 1, self.sr);
        self.eq[2].configure(4800., p.air, 2, self.sr);
        self.in_gain = gain(p.input);
        self.out_gain = gain(p.output);
        self.attack = (-1. / (self.sr * p.attack * 0.001)).exp();
        self.release = (-1. / (self.sr * p.release * 0.001)).exp();
        self.lp_alpha = 1. - (-TAU * p.lowpass.min(self.sr * 0.45) / self.sr).exp();
        self.gate_level = gain(p.gate);
        self.comp_level = gain(p.threshold);
        self.pitch_ratio = 2_f32.powf(p.pitch / 12.);
        self.vibrato_step = TAU * p.vibrato_hz / self.sr;
        self.ring_step = TAU * p.ring_hz / self.sr;
        self.pitch_step = (1. - self.pitch_ratio) / self.pitch_span;
        self.comp_exponent = 1. / p.ratio - 1.;
        self.drive = 1. + p.drive * 18.;
        self.drive_norm = self.drive.sqrt();
        self.bit_levels = 2_f32.powf(p.bits - 1.);
        self.echo_delay = ((self.sr * p.delay_ms * 0.001) as usize).min(self.echo.len() - 1);
        self.room_feedback = 0.55 + p.room * 0.35;
    }
    fn read_pitch(&self, delay: f32) -> f32 {
        let len = self.pitch_buf.len();
        let position = self.write as f32 - delay;
        let pos = if position >= 0. && position < len as f32 {
            position
        } else {
            position.rem_euclid(len as f32)
        };
        let i = pos as usize;
        let f = pos - i as f32;
        // A tiny negative position can round up to `len` in f32 rem_euclid.
        let i = if i == len { 0 } else { i };
        let next = if i + 1 == len { 0 } else { i + 1 };
        self.pitch_buf[i] * (1. - f) + self.pitch_buf[next] * f
    }
    pub fn board_gain(&self) -> f32 {
        self.p.board
    }
    pub fn tick(&mut self, input: f32, enabled: bool) -> f32 {
        let p = self.p;
        let raw = if input.is_finite() {
            input.clamp(-8., 8.)
        } else {
            0.
        } * self.in_gain;
        let hp = raw - self.dc_x + 0.992 * self.dc_y;
        self.dc_x = raw;
        self.dc_y = hp;
        let amp = hp.abs();
        let env_a = if amp > self.envelope {
            self.attack
        } else {
            self.release
        };
        self.envelope = env_a * self.envelope + (1. - env_a) * amp;
        let target = if self.envelope > self.gate_level {
            1.
        } else {
            0.
        };
        let ga = if target > self.gate_gain {
            0.006
        } else {
            0.0006
        };
        self.gate_gain += (target - self.gate_gain) * ga;
        let mut x = hp * self.gate_gain;
        if !enabled {
            return (x * self.out_gain).clamp(-0.99, 0.99);
        }
        let dry = x;
        self.pitch_buf[self.write] = x;
        self.vib += self.vibrato_step;
        if self.vib >= TAU {
            self.vib -= TAU;
        }
        let span = self.pitch_span;
        self.phase += self.pitch_step;
        if self.phase < 0. {
            self.phase += 1.;
        } else if self.phase >= 1. {
            self.phase -= 1.;
        }
        if p.pitch.abs() > 0.01 || p.vibrato > 0.001 {
            let mut ph2 = self.phase + 0.5;
            if ph2 >= 1. {
                ph2 -= 1.;
            }
            let v = self.vib.sin() * p.vibrato * self.sr * 0.0015;
            let w = (PI * self.phase).sin().powi(2);
            x = self.read_pitch(64. + self.phase * span + v) * w
                + self.read_pitch(64. + ph2 * span + v) * (1. - w);
        }
        self.write += 1;
        if self.write == self.pitch_buf.len() {
            self.write = 0;
        }
        for eq in &mut self.eq {
            x = eq.tick(x);
        }
        let target_comp = if self.envelope > self.comp_level {
            (self.envelope / self.comp_level).powf(self.comp_exponent)
        } else {
            1.
        };
        self.comp_gain = env_a * self.comp_gain + (1. - env_a) * target_comp;
        x *= self.comp_gain;
        self.de_lp += (x - self.de_lp) * 0.45;
        let hi = x - self.de_lp;
        x -= hi * (hi.abs() * 12.).min(1.) * p.deess;
        self.osc += self.ring_step;
        if self.osc >= TAU {
            self.osc -= TAU;
        }
        if p.robot != 0. {
            x *= 1. - p.robot + p.robot * self.osc.sin();
        }
        if p.drive > 0.001 {
            x = (x * self.drive).tanh() / self.drive_norm;
        }
        if p.bits < 15.9 {
            let levels = self.bit_levels;
            x = (x * levels).round() / levels;
        }
        self.lp += (x - self.lp) * self.lp_alpha;
        x = self.lp;
        let mut read = self.echo_pos + self.echo.len() - self.echo_delay;
        if read >= self.echo.len() {
            read -= self.echo.len();
        }
        let echo = self.echo[read];
        self.echo[self.echo_pos] = x + echo * p.feedback;
        self.echo_pos += 1;
        if self.echo_pos == self.echo.len() {
            self.echo_pos = 0;
        }
        x += echo * p.delay;
        let mut rv = 0.;
        for c in &mut self.combs {
            rv += c.tick(x * 0.18, self.room_feedback);
        }
        x += rv * p.reverb;
        let y = (dry * (1. - p.mix) + x * p.mix) * self.out_gain;
        if !y.is_finite() {
            return 0.;
        }
        let a = y.abs();
        if a > 0.9 {
            y.signum() * (0.9 + 0.09 * ((a - 0.9) / 0.09).tanh())
        } else {
            y
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_presets_stable_and_bounded() {
        for pr in crate::presets::factory() {
            let mut d = Dsp::new(48000.);
            d.configure(pr.params);
            for i in 0..48000 {
                let y = d.tick((TAU * 220. * i as f32 / 48000.).sin() * 0.4, true);
                assert!(y.is_finite() && y.abs() <= 0.991, "{}", pr.name);
            }
        }
    }
    #[test]
    fn pitch_changes_frequency() {
        for semi in [-7., 7.] {
            let mut d = Dsp::new(48000.);
            let mut p = Params::default();
            p.pitch = semi;
            p.gate = -80.;
            p.ratio = 1.;
            p.deess = 0.;
            d.configure(p);
            let v: Vec<_> = (0..96000)
                .map(|i| d.tick((TAU * 220. * i as f32 / 48000.).sin() * 0.2, true))
                .skip(48000)
                .collect();
            let crosses = v.windows(2).filter(|w| w[0] <= 0. && w[1] > 0.).count() as f32;
            let expected = 220. * 2_f32.powf(semi / 12.);
            assert!(
                (crosses - expected).abs() / expected < 0.09,
                "{crosses} != {expected}"
            );
        }
    }
    #[test]
    fn rejects_invalid_settings() {
        let mut p = Params::default();
        p.pitch = f32::NAN;
        p.feedback = 20.;
        p.sanitize();
        assert_eq!(p.pitch, 0.);
        assert_eq!(p.feedback, 0.8);
    }
    #[test]
    fn pitch_interpolation_wraps_rounded_buffer_endpoint() {
        let mut d = Dsp::new(48000.);
        d.pitch_buf[0] = 0.25;
        d.write = 0;
        // rem_euclid rounds this tiny negative position to exactly 7200.0.
        assert_eq!(d.read_pitch(0.0001), 0.25);
    }
}
