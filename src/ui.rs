use crate::i18n::{t, Language};
use crate::{
    audio::{self, Engine, Shared},
    dsp::{Dsp, Params},
    hotkeys::Hotkeys,
    presets::{self, Preset},
    routing,
};
use eframe::egui::{Color32 as C, *};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const BG: C = C::from_rgb(20, 21, 23);
const PANEL: C = C::from_rgb(25, 26, 28);
const CARD: C = C::from_rgb(31, 32, 35);
const LINE: C = C::from_rgb(48, 49, 52);
const TEXT: C = C::from_rgb(236, 234, 226);
const MUTED: C = C::from_rgb(143, 145, 148);
const AMBER: C = C::from_rgb(235, 184, 111);
const GREEN: C = C::from_rgb(151, 192, 157);
const RED: C = C::from_rgb(219, 121, 109);
fn bold(n: f32) -> FontId {
    FontId::new(n, FontFamily::Name("bold".into()))
}
fn label(ui: &mut Ui, t: impl Into<String>, size: f32, color: C) {
    ui.label(RichText::new(t.into()).size(size).color(color));
}
fn title(ui: &mut Ui, t: &str, size: f32) {
    ui.label(RichText::new(t).font(bold(size)).color(TEXT));
}
fn section(ui: &mut Ui, t: &str) {
    ui.add_space(12.);
    label(ui, t, 10., MUTED);
    ui.add_space(10.);
}
fn button(ui: &mut Ui, t: &str) -> Response {
    ui.add(
        Button::new(RichText::new(t).size(13.))
            .min_size(vec2(0., 34.))
            .corner_radius(7.),
    )
}
fn primary(ui: &mut Ui, t: &str, size: Vec2) -> Response {
    ui.add(
        Button::new(RichText::new(t).font(bold(13.)).color(BG))
            .fill(AMBER)
            .stroke(Stroke::NONE)
            .min_size(size)
            .corner_radius(7.),
    )
}
fn toggle(ui: &mut Ui, v: &mut bool) -> Response {
    let (r, mut res) = ui.allocate_exact_size(vec2(34., 20.), Sense::click());
    if res.clicked() {
        *v = !*v;
        res.mark_changed();
    }
    ui.painter()
        .rect_filled(r, 10., if *v { AMBER } else { LINE });
    ui.painter().circle_filled(
        pos2(
            if *v { r.right() - 10. } else { r.left() + 10. },
            r.center().y,
        ),
        7.,
        if *v { BG } else { MUTED },
    );
    res
}
fn row_toggle(ui: &mut Ui, t: &str, v: &mut bool) {
    ui.horizontal(|ui| {
        ui.label(t);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            toggle(ui, v);
        });
    });
}
fn control(ui: &mut Ui, name: &str, v: &mut f32, min: f32, max: f32, suffix: &str) -> bool {
    let mut reset = false;
    ui.horizontal(|ui| {
        let r = ui.label(RichText::new(name).size(12.).color(TEXT));
        if r.double_clicked() {
            reset = true;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{:.1}{}", *v, suffix))
                    .monospace()
                    .size(11.)
                    .color(AMBER),
            );
        });
    });
    let changed = ui
        .add(
            Slider::new(v, min..=max)
                .show_value(false)
                .trailing_fill(true),
        )
        .changed();
    ui.add_space(5.);
    let _ = reset;
    changed
}
fn glyph(p: &Painter, r: Rect, color: C, kind: usize) {
    let center = r.center();
    let radius = r.width().min(r.height()) * 0.35;
    match kind % 6 {
        0 => {
            for i in 0..7 {
                let x = center.x + (i as f32 - 3.) * radius * 0.32;
                let h = radius * (0.25 + ((i * 7 + kind) % 9) as f32 * 0.1);
                p.line_segment(
                    [pos2(x, center.y - h), pos2(x, center.y + h)],
                    Stroke::new(2.2_f32, color),
                );
            }
        }
        1 => {
            for i in 0..3 {
                let rr = radius * (0.45 + i as f32 * 0.3);
                p.circle_stroke(
                    center,
                    rr,
                    Stroke::new(1.3_f32, color.gamma_multiply(1. - i as f32 * 0.18)),
                );
            }
            p.circle_filled(center + vec2(radius * 0.75, -radius * 0.55), 3., color);
        }
        2 => {
            for i in 0..3 {
                let y = center.y + (i as f32 - 1.) * radius * 0.5;
                let pts: Vec<_> = (0..32)
                    .map(|j| {
                        let x = j as f32 / 31.;
                        pos2(
                            center.x - radius + 2. * radius * x,
                            y + (x * std::f32::consts::TAU).sin() * radius * 0.25,
                        )
                    })
                    .collect();
                p.add(Shape::line(pts, Stroke::new(1.7_f32, color)));
            }
        }
        3 => {
            for i in 0..3 {
                let a = i as f32 * std::f32::consts::PI / 3.;
                let pts: Vec<_> = (0..65)
                    .map(|j| {
                        let t = j as f32 / 64. * std::f32::consts::TAU;
                        let x = t.cos() * radius;
                        let y = t.sin() * radius * 0.35;
                        center + vec2(x * a.cos() - y * a.sin(), x * a.sin() + y * a.cos())
                    })
                    .collect();
                p.add(Shape::line(pts, Stroke::new(1.2_f32, color)));
            }
        }
        4 => {
            for i in 0..4 {
                let x = center.x + (i as f32 - 1.5) * radius * 0.48;
                let h = radius * (0.45 + ((i + kind) % 3) as f32 * 0.24);
                p.rect_stroke(
                    Rect::from_center_size(pos2(x, center.y), vec2(radius * 0.27, h * 2.)),
                    2.,
                    Stroke::new(1.5_f32, color),
                    StrokeKind::Inside,
                );
            }
        }
        _ => {
            for i in 0..3 {
                let rr = radius * (1. - i as f32 * 0.25);
                let pts: Vec<_> = (0..=6)
                    .map(|j| {
                        let a = j as f32 * std::f32::consts::TAU / 6. - std::f32::consts::FRAC_PI_2;
                        center + vec2(a.cos(), a.sin()) * rr
                    })
                    .collect();
                p.add(Shape::line(pts, Stroke::new(1.2_f32, color)));
            }
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    language: Language,
    params: Params,
    selected: usize,
    input: String,
    output: String,
    monitor_device: String,
    monitor: bool,
    buffer_ms: u32,
    favorites: Vec<String>,
    custom: Vec<Preset>,
    hotkeys: bool,
    compact: bool,
    system_voice: bool,
}
impl Default for Settings {
    fn default() -> Self {
        let (i, o) = audio::defaults();
        let route = audio::devices()
            .1
            .into_iter()
            .find(|s| audio::is_virtual_output(s))
            .unwrap_or_else(|| o.clone());
        Self {
            language: Language::default(),
            params: presets::factory()[1].params,
            selected: 1,
            input: i,
            output: route,
            monitor_device: o,
            monitor: false,
            buffer_ms: 24,
            favorites: vec!["Velvet".into(), "Android".into(), "Broadcast".into()],
            custom: vec![],
            hotkeys: true,
            compact: false,
            system_voice: true,
        }
    }
}
pub struct Studio {
    cfg: Settings,
    all: Vec<Preset>,
    shared: Arc<Shared>,
    engine: Option<Engine>,
    route: Option<routing::Guard>,
    route_checked: Instant,
    inputs: Vec<String>,
    outputs: Vec<String>,
    page: usize,
    tab: usize,
    category: usize,
    search: String,
    only_favs: bool,
    data: PathBuf,
    toast: String,
    toast_at: Instant,
    save_at: Instant,
    dirty: bool,
    name: String,
    clips: Vec<PathBuf>,
    files: Vec<PathBuf>,
    record_at: Option<Instant>,
    in_meter: f32,
    out_meter: f32,
    wave: [f32; 512],
    hotkeys: Hotkeys,
    key_at: [Instant; 3],
    board_at: [Instant; 8],
    started: Instant,
    shot: bool,
    qa: Option<usize>,
    auto_audio: bool,
    qa_exit: bool,
}
impl Studio {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "ui".into(),
            FontData::from_static(include_bytes!("../assets/ui.ttf")).into(),
        );
        fonts.font_data.insert(
            "bold".into(),
            FontData::from_static(include_bytes!("../assets/bold.ttf")).into(),
        );
        fonts.font_data.insert(
            "mono".into(),
            FontData::from_static(include_bytes!("../assets/mono.ttf")).into(),
        );
        fonts.font_data.insert(
            "symbols".into(),
            FontData::from_static(include_bytes!("../assets/symbols.ttf")).into(),
        );
        fonts.families.insert(
            FontFamily::Proportional,
            vec!["ui".into(), "symbols".into()],
        );
        fonts.families.insert(
            FontFamily::Name("bold".into()),
            vec!["bold".into(), "ui".into(), "symbols".into()],
        );
        fonts
            .families
            .insert(FontFamily::Monospace, vec!["mono".into(), "symbols".into()]);
        cc.egui_ctx.set_fonts(fonts);
        let mut style = Style::default();
        style.visuals = Visuals::dark();
        style.visuals.panel_fill = BG;
        style.visuals.window_fill = PANEL;
        style.visuals.extreme_bg_color = C::from_rgb(17, 18, 20);
        style.visuals.faint_bg_color = CARD;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.selection.bg_fill = AMBER.gamma_multiply(0.35);
        style.visuals.selection.stroke = Stroke::new(1.0_f32, AMBER);
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
        style.visuals.widgets.inactive.bg_fill = CARD;
        style.visuals.widgets.inactive.weak_bg_fill = CARD;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);
        style.visuals.widgets.hovered.bg_fill = C::from_rgb(49, 48, 45);
        style.visuals.widgets.hovered.weak_bg_fill = C::from_rgb(49, 48, 45);
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, AMBER.gamma_multiply(0.65));
        style.visuals.widgets.active.bg_fill = AMBER;
        style.visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, AMBER);
        style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, MUTED);
        style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);
        style.spacing.item_spacing = vec2(10., 8.);
        style.spacing.button_padding = vec2(12., 7.);
        style.spacing.slider_width = 264.;
        style.spacing.interact_size.y = 26.;
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(13.));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(13.));
        cc.egui_ctx.set_style(style);
        let qa_mode = std::env::args().any(|arg| arg.starts_with("--screenshot="));
        if qa_mode {
            cc.egui_ctx
                .style_mut(|style| style.interaction.tooltip_delay = 60.);
        }
        let data = if qa_mode {
            crate::diagnostic_dir().join("qa-profile")
        } else {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join("Vesper")
        };
        let _ = std::fs::create_dir_all(data.join("Recordings"));
        let _ = std::fs::create_dir_all(data.join("Sounds"));
        let mut load_error = None;
        match routing::recover(&data) {
            Ok(true) => load_error = Some(t("Microphone restored after a crash").into()),
            Ok(false) => {}
            Err(e) => load_error = Some(e),
        }
        let mut cfg = match std::fs::read(data.join("settings.json")) {
            Ok(b) => match serde_json::from_slice::<Settings>(&b) {
                Ok(s) => s,
                Err(e) => {
                    load_error = Some(format!("Could not read settings: {e}"));
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        };
        if qa_mode {
            if let Some(language) = std::env::args().find_map(|arg| {
                arg.strip_prefix("--language=")
                    .and_then(Language::from_code)
            }) {
                cfg.language = language;
            }
        }
        crate::i18n::set_language(cfg.language);
        if qa_mode {
            cfg.hotkeys = false;
            cfg.system_voice = false;
        }
        cfg.params.sanitize();
        cfg.buffer_ms = cfg.buffer_ms.clamp(8, 100);
        for p in &mut cfg.custom {
            p.category = "Custom".into();
            if p.desc == "Твой собственный голос." {
                p.desc = "Custom voice".into();
            }
            p.params.sanitize();
        }
        let mut all = presets::factory();
        all.extend(cfg.custom.iter().cloned());
        cfg.selected = cfg.selected.min(all.len() - 1);
        let shared = Shared::new(cfg.params);
        let (inputs, outputs) = audio::devices();
        if !inputs.contains(&cfg.input) {
            cfg.input = audio::defaults().0;
        }
        if !outputs.contains(&cfg.output) {
            cfg.output = outputs
                .iter()
                .find(|s| s.to_lowercase().contains("cable input"))
                .cloned()
                .unwrap_or_else(|| audio::defaults().1);
        }
        let args: Vec<_> = std::env::args().collect();
        let qa = args
            .iter()
            .find_map(|s| s.strip_prefix("--screenshot=").and_then(|s| s.parse().ok()));
        if qa.is_some() {
            let _ = std::fs::create_dir_all(crate::diagnostic_dir());
        }
        let hotkeys = Hotkeys::new(cc.egui_ctx.clone(), cfg.hotkeys);
        let mut app = Self {
            cfg,
            all,
            shared,
            engine: None,
            route: None,
            route_checked: Instant::now(),
            inputs,
            outputs,
            page: qa.unwrap_or(0).min(4),
            tab: if qa_mode {
                args.iter()
                    .find_map(|arg| {
                        arg.strip_prefix("--qa-tab=")
                            .and_then(|value| value.parse::<usize>().ok())
                    })
                    .unwrap_or(0)
                    .min(2)
            } else {
                0
            },
            category: 0,
            search: String::new(),
            only_favs: false,
            data,
            toast: load_error.unwrap_or_default(),
            toast_at: Instant::now(),
            save_at: Instant::now(),
            dirty: false,
            name: String::new(),
            clips: vec![],
            files: vec![],
            record_at: None,
            in_meter: 0.,
            out_meter: 0.,
            wave: [0.; 512],
            hotkeys,
            key_at: [Instant::now() - Duration::from_secs(1); 3],
            board_at: [Instant::now() - Duration::from_secs(1); 8],
            started: Instant::now(),
            shot: false,
            qa,
            auto_audio: args.iter().any(|s| s == "--audio-smoke"),
            qa_exit: args.iter().any(|s| s == "--qa-exit"),
        };
        app.refresh_files();
        app
    }
    fn notify(&mut self, t: impl Into<String>) {
        self.toast = t.into();
        self.toast_at = Instant::now();
    }
    fn modified(&mut self) {
        self.dirty = true;
        self.save_at = Instant::now();
        if let Ok(mut p) = self.shared.params.lock() {
            *p = self.cfg.params;
        }
    }
    fn save(&mut self) {
        match serde_json::to_vec_pretty(&self.cfg)
            .map_err(|e| e.to_string())
            .and_then(|b| {
                let tmp = self.data.join("settings.tmp");
                std::fs::write(&tmp, b).map_err(|e| e.to_string())?;
                std::fs::rename(tmp, self.data.join("settings.json")).map_err(|e| e.to_string())
            }) {
            Ok(()) => self.dirty = false,
            Err(e) => {
                self.notify(format!("Could not save settings: {e}"));
                self.dirty = false;
            }
        }
    }
    fn select(&mut self, i: usize) {
        self.cfg.selected = i;
        self.cfg.params = self.all[i].params;
        self.modified();
    }
    fn refresh_files(&mut self) {
        self.files = scan(&self.data.join("Recordings"));
        self.files.sort_by(|a, b| b.cmp(a));
        self.clips = scan(&self.data.join("Sounds"));
    }
    fn stop_engine(&mut self) -> bool {
        let mut restored = true;
        if let Some(mut route) = self.route.take() {
            if let Err(e) = route.stop() {
                restored = false;
                self.notify(format!("Could not restore the Windows microphone: {e}"));
            }
        }
        self.engine.take();
        self.wave = [0.; 512];
        self.record_at = None;
        restored
    }
    fn start_stop(&mut self) {
        if self.qa.is_some() {
            return;
        }
        if self.engine.is_some() {
            let restored = self.stop_engine();
            self.refresh_files();
            if restored {
                self.notify(t("Microphone restored"));
            }
        } else {
            (self.inputs, self.outputs) = audio::devices();
            let cable = if self.cfg.system_voice {
                match routing::cable() {
                    Ok(c) => Some(c),
                    Err(e) => {
                        self.page = 4;
                        self.notify(e);
                        return;
                    }
                }
            } else {
                None
            };
            if let Some(c) = &cable {
                self.cfg.output = c.playback.name.clone();
                let physical = match routing::physical_inputs() {
                    Ok(v) => v,
                    Err(e) => {
                        self.notify(e);
                        return;
                    }
                };
                if !physical.iter().any(|e| e.name == self.cfg.input) {
                    let Some(input) = physical.first() else {
                        self.notify(t("Connect a physical microphone"));
                        return;
                    };
                    self.cfg.input = input.name.clone();
                }
            }
            let mon = if self.cfg.monitor {
                Some(self.cfg.monitor_device.as_str())
            } else {
                None
            };
            match Engine::start(
                &self.cfg.input,
                &self.cfg.output,
                mon,
                self.cfg.buffer_ms,
                self.shared.clone(),
            ) {
                Ok(e) => {
                    if let Some(c) = cable {
                        match routing::Guard::start(&self.data, &c.capture.id, &self.cfg.input) {
                            Ok(guard) => self.route = Some(guard),
                            Err(err) => {
                                drop(e);
                                self.notify(err);
                                return;
                            }
                        }
                    }
                    self.engine = Some(e);
                    self.modified();
                    self.notify(if self.route.is_some() {
                        t("System microphone active")
                    } else {
                        t("Local monitoring and recording")
                    });
                }
                Err(e) => self.notify(e),
            }
        }
    }
    fn record(&mut self) {
        if self.shared.recording.load(Ordering::Relaxed) {
            if let Some(e) = &self.engine {
                e.stop_record();
            }
            self.record_at = None;
        } else if let Some(e) = &self.engine {
            let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let name = format!(
                "Vesper_{}_{}.wav",
                self.all[self.cfg.selected]
                    .name
                    .chars()
                    .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    })
                    .collect::<String>(),
                time
            );
            match e.record(self.data.join("Recordings").join(name)) {
                Ok(()) => {
                    self.record_at = Some(Instant::now());
                    self.notify(t("Recording started"));
                }
                Err(e) => self.notify(e),
            }
        } else {
            self.notify(t("Start voice processing first"));
        }
    }
    fn play_sound(&mut self, kind: usize) {
        if let Some(e) = &self.engine {
            if e.play.try_send(Some(audio::tone(kind, e.rate))).is_err() {
                self.notify(t("Sound queue is busy"));
            }
        } else {
            self.notify(t("Start voice processing first"));
        }
    }
    fn play_file(&mut self, path: &Path) {
        if let Some(e) = &self.engine {
            match audio::load_wav(path, e.rate) {
                Ok(v) => {
                    let _ = e.play.try_send(Some(v));
                    self.notify(t("Playing clip"));
                }
                Err(err) => self.notify(err),
            }
        } else {
            self.notify(t("Start voice processing first"));
        }
    }
    fn hotkeys(&mut self, ctx: &Context) {
        let mut triggers = [false; 8];
        let pending = self.hotkeys.take(self.cfg.hotkeys);
        if self.cfg.hotkeys {
            for (i, key) in [Key::F6, Key::F7, Key::F8].into_iter().enumerate() {
                let event_pressed = ctx.input(|s| s.key_pressed(key));
                if (pending & (1 << i) != 0 || event_pressed)
                    && self.key_at[i].elapsed() > Duration::from_millis(180)
                {
                    self.key_at[i] = Instant::now();
                    match i {
                        0 => {
                            let v = self.shared.enabled.load(Ordering::Relaxed);
                            self.shared.enabled.store(!v, Ordering::Relaxed);
                        }
                        1 => {
                            let v = self.shared.muted.load(Ordering::Relaxed);
                            self.shared.muted.store(!v, Ordering::Relaxed);
                        }
                        _ => self.record(),
                    }
                }
            }
        }
        for (i, key) in [
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
            Key::Num8,
        ]
        .into_iter()
        .enumerate()
        {
            triggers[i] |= ctx.input(|a| a.key_pressed(key) && a.modifiers.ctrl);
        }
        for (i, trigger) in triggers.into_iter().enumerate() {
            if trigger && self.board_at[i].elapsed() > Duration::from_millis(180) {
                self.board_at[i] = Instant::now();
                self.play_sound(i);
            }
        }
    }
    fn rail(&mut self, ctx: &Context) {
        SidePanel::left("rail")
            .exact_width(184.)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(16, 24)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(24., 28.), Sense::hover());
                    for i in 0..3 {
                        let x = r.left() + 4. + i as f32 * 7.;
                        let h = if i == 1 { 22. } else { 12. };
                        ui.painter().line_segment(
                            [
                                pos2(x, r.center().y - h / 2.),
                                pos2(x, r.center().y + h / 2.),
                            ],
                            Stroke::new(3.0_f32, AMBER),
                        );
                    }
                    ui.label(RichText::new("VESPER").font(bold(20.)).color(TEXT));
                });
                ui.add_space(32.);
                for (i, name) in ["Voices", "Studio", "Soundboard", "Recordings", "Settings"]
                    .iter()
                    .enumerate()
                {
                    let (r, res) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 44.), Sense::click());
                    let selected = self.page == i;
                    if selected || res.hovered() {
                        ui.painter().rect_filled(
                            r,
                            8.,
                            if selected {
                                C::from_rgb(47, 41, 33)
                            } else {
                                CARD
                            },
                        );
                    }
                    let color = if selected { AMBER } else { MUTED };
                    nav_icon(ui.painter(), r.min + vec2(20., 22.), i, color);
                    ui.painter().text(
                        r.min + vec2(40., 22.),
                        Align2::LEFT_CENTER,
                        t(name),
                        FontId::proportional(13.),
                        if selected { TEXT } else { MUTED },
                    );
                    if res.clicked() {
                        self.page = i;
                        self.refresh_files();
                    }
                    ui.add_space(3.);
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    label(
                        ui,
                        concat!("VESPER ", env!("CARGO_PKG_VERSION")),
                        10.,
                        MUTED,
                    );
                    ui.add_space(12.);
                    self.language_picker(ui, "rail-language");
                });
            });
    }
    fn language_picker(&mut self, ui: &mut Ui, id: &str) {
        let old = self.cfg.language;
        ComboBox::from_id_salt(id)
            .selected_text(self.cfg.language.name())
            .height(320.)
            .width(ui.available_width().min(240.))
            .show_ui(ui, |ui| {
                for language in crate::i18n::Language::ALL {
                    ui.selectable_value(&mut self.cfg.language, language, language.name());
                }
            });
        if old != self.cfg.language {
            crate::i18n::set_language(self.cfg.language);
            self.toast.clear();
            self.modified();
            ui.ctx().request_repaint();
        }
    }

    fn transport(&mut self, ctx: &Context) {
        TopBottomPanel::bottom("transport")
            .exact_height(78.)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(24, 15))
                    .stroke(Stroke::new(1.0_f32, LINE)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let running = self.engine.is_some();
                    if primary(
                        ui,
                        t(if running { "Stop" } else { "Start voice" }),
                        vec2(146., 44.),
                    )
                    .clicked()
                    {
                        self.start_stop();
                    }
                    ui.add_space(10.);
                    let mut fx = self.shared.enabled.load(Ordering::Relaxed);
                    ui.vertical(|ui| {
                        label(ui, t("Effects"), 10., MUTED);
                        if toggle(ui, &mut fx).on_hover_text("F6").changed() {
                            self.shared.enabled.store(fx, Ordering::Relaxed);
                        }
                    });
                    ui.add_space(8.);
                    let muted = self.shared.muted.load(Ordering::Relaxed);
                    if ui
                        .add(
                            Button::new(
                                RichText::new(t(if muted { "Unmute" } else { "Mute" }))
                                    .color(if muted { RED } else { TEXT }),
                            )
                            .min_size(vec2(94., 40.)),
                        )
                        .on_hover_text("F7")
                        .clicked()
                    {
                        self.shared.muted.store(!muted, Ordering::Relaxed);
                    }
                    let old = self.cfg.monitor;
                    ui.vertical(|ui| {
                        label(ui, t("Monitor"), 10., MUTED);
                        toggle(ui, &mut self.cfg.monitor);
                    });
                    if old != self.cfg.monitor {
                        self.modified();
                        if running {
                            self.stop_engine();
                            self.start_stop();
                        }
                    }
                    ui.add_space(12.);
                    ui.vertical(|ui| {
                        label(ui, t("Input"), 10., MUTED);
                        meter(ui, self.in_meter, 70.);
                    });
                    ui.vertical(|ui| {
                        label(ui, t("Output"), 10., MUTED);
                        meter(ui, self.out_meter, 70.);
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let rec = self.shared.recording.load(Ordering::Relaxed);
                        let text = if let Some(at) = self.record_at {
                            format!(
                                "■  {:02}:{:02}",
                                at.elapsed().as_secs() / 60,
                                at.elapsed().as_secs() % 60
                            )
                        } else {
                            format!("●  {}", t("Record"))
                        };
                        if ui
                            .add_enabled(
                                running,
                                Button::new(RichText::new(text).color(if rec {
                                    RED
                                } else {
                                    TEXT
                                }))
                                .min_size(vec2(110., 40.)),
                            )
                            .on_hover_text("F8")
                            .clicked()
                        {
                            self.record();
                        }
                        if let Some(engine) = &self.engine {
                            label(
                                ui,
                                format!("{} Hz · {:.0} ms", engine.rate, engine.buffered_ms()),
                                10.,
                                MUTED,
                            );
                        }
                    });
                });
            });
    }

    fn inspector(&mut self, ctx: &Context) {
        SidePanel::right("inspector")
            .exact_width(314.)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(22, 25)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    label(ui, t("Voice controls"), 10., MUTED);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button(t("Reset")).clicked() {
                            self.cfg.params = self.all[self.cfg.selected].params;
                            self.modified();
                        }
                    });
                });
                ui.add_space(12.);
                title(ui, &self.all[self.cfg.selected].name, 24.);

                ui.add_space(17.);
                ui.horizontal(|ui| {
                    for (i, t) in [t("Tone"), t("Effects"), t("Dynamics")].iter().enumerate() {
                        if ui.selectable_label(self.tab == i, *t).clicked() {
                            self.tab = i;
                        }
                    }
                });
                ui.add_space(8.);
                ui.separator();
                let before = self.cfg.params;
                ScrollArea::vertical()
                    .id_salt("controls")
                    .max_height(ui.available_height() - 102.)
                    .show(ui, |ui| {
                        let width = (ui.available_width() - 8.).max(100.);
                        ui.set_width(width);
                        ui.spacing_mut().slider_width = width;
                        let p = &mut self.cfg.params;
                        match self.tab {
                            0 => {
                                section(ui, t("Pitch & movement"));
                                control(ui, t("Pitch"), &mut p.pitch, -12., 12., " st");
                                control(ui, t("Vibrato depth"), &mut p.vibrato, 0., 1., "");
                                control(ui, t("Vibrato rate"), &mut p.vibrato_hz, 0.5, 12., " Hz");
                                section(ui, t("Equalizer"));
                                control(ui, t("Bass · 160 Hz"), &mut p.bass, -12., 12., " dB");
                                control(ui, t("Body · 1.1 kHz"), &mut p.body, -12., 12., " dB");
                                control(ui, t("Air · 4.8 kHz"), &mut p.air, -12., 12., " dB");
                                control(ui, t("High cut"), &mut p.lowpass, 500., 20000., " Hz");
                                section(ui, t("Mix"));
                                control(ui, t("Dry / wet"), &mut p.mix, 0., 1., "");
                            }
                            1 => {
                                section(ui, t("Texture"));
                                control(ui, t("Drive"), &mut p.drive, 0., 1., "");
                                control(ui, t("Robot"), &mut p.robot, 0., 1., "");
                                control(ui, t("Ring frequency"), &mut p.ring_hz, 10., 600., " Hz");
                                control(ui, t("Bit depth"), &mut p.bits, 4., 16., " bit");
                                section(ui, t("Space"));
                                control(ui, t("Reverb"), &mut p.reverb, 0., 0.8, "");
                                control(ui, t("Room size"), &mut p.room, 0., 1., "");
                                control(ui, t("Delay level"), &mut p.delay, 0., 0.85, "");
                                control(ui, t("Delay time"), &mut p.delay_ms, 20., 650., " ms");
                                control(ui, t("Feedback"), &mut p.feedback, 0., 0.8, "");
                            }
                            _ => {
                                section(ui, t("Levels"));
                                control(ui, t("Input gain"), &mut p.input, -24., 24., " dB");
                                control(ui, t("Output gain"), &mut p.output, -24., 12., " dB");
                                section(ui, t("Gate & compressor"));
                                control(ui, t("Gate threshold"), &mut p.gate, -80., -15., " dB");
                                control(
                                    ui,
                                    t("Compressor threshold"),
                                    &mut p.threshold,
                                    -40.,
                                    0.,
                                    " dB",
                                );
                                control(ui, t("Ratio"), &mut p.ratio, 1., 12., ":1");
                                control(ui, t("Attack"), &mut p.attack, 1., 80., " ms");
                                control(ui, t("Release"), &mut p.release, 20., 600., " ms");
                                control(ui, t("De-esser"), &mut p.deess, 0., 1., "");
                                section(ui, t("Soundboard"));
                                control(ui, t("Clip volume"), &mut p.board, 0., 1., "");
                            }
                        }
                    });
                if before != self.cfg.params {
                    self.modified();
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    if button(ui, t("Save custom voice")).clicked() {
                        self.page = 1;
                        self.name = format!("{} Custom", self.all[self.cfg.selected].name);
                    }
                    ui.separator();
                });
            });
    }
    fn header(&mut self, ui: &mut Ui, text: &str) {
        ui.horizontal(|ui| {
            title(ui, text, 28.);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let running = self.engine.is_some();
                label(
                    ui,
                    t(if running { "Live" } else { "Ready" }),
                    11.,
                    if running { GREEN } else { MUTED },
                );
            });
        });
        ui.add_space(20.);
    }

    fn scope(&self, ui: &mut Ui, height: f32, hero: bool) {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        ui.painter().rect_filled(r, 12., PANEL);
        let left = if hero {
            r.left() + 190.
        } else {
            r.left() + 20.
        };
        let area = Rect::from_min_max(pos2(left, r.top() + 20.), r.max - vec2(20., 20.));
        ui.painter().line_segment(
            [
                pos2(area.left(), area.center().y),
                pos2(area.right(), area.center().y),
            ],
            Stroke::new(1.0_f32, LINE),
        );
        if hero {
            ui.painter().text(
                r.min + vec2(20., 26.),
                Align2::LEFT_TOP,
                &self.all[self.cfg.selected].name,
                bold(25.),
                TEXT,
            );
            ui.painter().text(
                r.min + vec2(20., 66.),
                Align2::LEFT_TOP,
                t(if self.route.is_some() {
                    "System microphone"
                } else {
                    "Local audio"
                }),
                FontId::proportional(11.),
                MUTED,
            );
        }
        let pts: Vec<_> = self
            .wave
            .iter()
            .enumerate()
            .map(|(i, v)| {
                pos2(
                    area.left() + area.width() * i as f32 / 511.,
                    area.center().y - v.clamp(-1., 1.) * area.height() * 0.48,
                )
            })
            .collect();
        ui.painter()
            .add(Shape::line(pts, Stroke::new(1.6_f32, AMBER)));
    }

    fn library(&mut self, ui: &mut Ui) {
        self.header(ui, t("Voices"));
        self.scope(ui, 106., true);
        if !self
            .outputs
            .iter()
            .any(|name| audio::is_virtual_output(name))
        {
            ui.add_space(10.);
            if ui.link(t("Connect virtual microphone")).clicked() {
                self.page = 4;
            }
        }
        ui.add_space(19.);
        ui.horizontal(|ui| {
            let favorite_width = ui
                .painter()
                .layout_no_wrap(t("★ Favorites").to_owned(), FontId::proportional(13.), TEXT)
                .size()
                .x
                + 34.;
            let search_width =
                (ui.available_width() - favorite_width - ui.spacing().item_spacing.x - 26.)
                    .max(100.);
            ui.add(
                TextEdit::singleline(&mut self.search)
                    .hint_text(t("Search voices…"))
                    .desired_width(search_width)
                    .margin(vec2(12., 9.)),
            );
            if button(
                ui,
                if self.only_favs {
                    t("★ Favorites")
                } else {
                    t("☆ Favorites")
                },
            )
            .clicked()
            {
                self.only_favs = !self.only_favs;
            }
        });
        ui.add_space(8.);
        ui.horizontal_wrapped(|ui| {
            for (i, t) in [
                t("All"),
                t("Character"),
                t("Synthetic"),
                t("Atmosphere"),
                t("Retro"),
                t("Custom"),
            ]
            .iter()
            .enumerate()
            {
                let selected = self.category == i;
                let res = ui.add(
                    Button::new(RichText::new(*t).size(11.).color(if selected {
                        BG
                    } else {
                        MUTED
                    }))
                    .fill(if selected { AMBER } else { BG })
                    .stroke(Stroke::NONE)
                    .corner_radius(20.),
                );
                if res.clicked() {
                    self.category = i;
                }
            }
        });
        ui.add_space(14.);
        let cats = [
            "",
            "Character",
            "Synthetic",
            "Atmosphere",
            "Retro",
            "Custom",
        ];
        let search = self.search.to_lowercase();
        let visible: Vec<_> = self
            .all
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                (self.category == 0 || p.category == cats[self.category])
                    && (!self.only_favs || self.cfg.favorites.contains(&p.name))
                    && (search.is_empty()
                        || format!(
                            "{} {} {} {} {}",
                            p.name,
                            p.desc,
                            p.category,
                            t(&p.desc),
                            t(&p.category)
                        )
                        .to_lowercase()
                        .contains(&search))
            })
            .map(|(i, _)| i)
            .collect();
        ScrollArea::vertical()
            .id_salt("voices")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if visible.is_empty() {
                    ui.add_space(40.);
                    title(ui, t("No voices found"), 20.);
                    label(
                        ui,
                        t("Try another search or save a custom voice in Studio."),
                        12.,
                        MUTED,
                    );
                    return;
                }
                let cols = if ui.available_width() > 610. { 3 } else { 2 };
                let w = (ui.available_width() - (cols - 1) as f32 * 12.) / cols as f32;
                for row in visible.chunks(cols) {
                    ui.horizontal(|ui| {
                        for &i in row {
                            self.preset_card(ui, i, w);
                        }
                    });
                    ui.add_space(4.);
                }
            });
    }
    fn preset_card(&mut self, ui: &mut Ui, i: usize, width: f32) {
        let p = &self.all[i];
        let selected = i == self.cfg.selected;
        let height = if self.cfg.compact { 76. } else { 104. };
        let (r, res) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        let painter = ui.painter();
        painter.rect_filled(
            r,
            10.,
            if selected {
                C::from_rgb(43, 38, 31)
            } else if res.hovered() {
                C::from_rgb(36, 37, 40)
            } else {
                CARD
            },
        );
        if selected {
            painter.rect_stroke(
                r,
                10.,
                Stroke::new(1.0_f32, AMBER.gamma_multiply(0.8)),
                StrokeKind::Inside,
            );
        }
        let color = C::from_rgb(p.color[0], p.color[1], p.color[2]);
        glyph(
            painter,
            Rect::from_center_size(pos2(r.left() + 35., r.center().y), vec2(42., 42.)),
            color,
            p.shape,
        );
        let font = if width < 220. { 16. } else { 18. };
        painter.text(
            pos2(r.left() + 68., r.center().y),
            Align2::LEFT_CENTER,
            &p.name,
            bold(font),
            TEXT,
        );
        let star = Rect::from_min_size(pos2(r.right() - 34., r.top() + 8.), vec2(28., 28.));
        let fav = self.cfg.favorites.contains(&p.name);
        painter.text(
            star.center(),
            Align2::CENTER_CENTER,
            if fav { "★" } else { "☆" },
            FontId::proportional(17.),
            if fav {
                AMBER
            } else {
                MUTED.gamma_multiply(0.5)
            },
        );
        if res.clicked() {
            if res
                .interact_pointer_pos()
                .is_some_and(|pos| star.contains(pos))
            {
                let name = p.name.clone();
                if fav {
                    self.cfg.favorites.retain(|n| *n != name);
                } else {
                    self.cfg.favorites.push(name);
                }
                self.modified();
            } else {
                self.select(i);
            }
        }
        res.on_hover_text(t(&self.all[i].desc));
    }

    fn studio(&mut self, ui: &mut Ui) {
        self.header(ui, t("Studio"));
        ScrollArea::vertical().show(ui, |ui| {
            self.scope(ui, 130., false);
            ui.add_space(22.);
            title(ui, &self.all[self.cfg.selected].name, 26.);
            ui.add_space(22.);
            Frame::new()
                .fill(PANEL)
                .corner_radius(12.)
                .inner_margin(22.)
                .show(ui, |ui| {
                    title(ui, t("Save custom voice"), 18.);
                    ui.add_space(14.);
                    ui.add(
                        TextEdit::singleline(&mut self.name)
                            .hint_text(t("Voice name"))
                            .desired_width(ui.available_width())
                            .margin(vec2(12., 10.)),
                    );
                    ui.add_space(12.);
                    if primary(ui, t("Save voice"), vec2(150., 38.)).clicked() {
                        let name = self.name.trim().chars().take(40).collect::<String>();
                        if name.is_empty() {
                            self.notify(t("Enter a voice name"));
                        } else if self.all.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
                            self.notify(t("This name already exists"));
                        } else {
                            let old = &self.all[self.cfg.selected];
                            let preset = Preset {
                                name,
                                desc: "Custom voice".into(),
                                category: "Custom".into(),
                                color: old.color,
                                shape: old.shape,
                                params: self.cfg.params,
                            };
                            self.cfg.custom.push(preset.clone());
                            self.all.push(preset);
                            self.cfg.selected = self.all.len() - 1;
                            self.modified();
                            self.notify(t("Voice saved"));
                            self.name.clear();
                        }
                    }
                });
            ui.add_space(16.);
            ui.horizontal_wrapped(|ui| {
                if button(ui, t("Export preset")).clicked() {
                    let mut preset = self.all[self.cfg.selected].clone();
                    preset.params = self.cfg.params;
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Vesper preset", &["json"])
                        .set_file_name("Vesper-preset.json")
                        .save_file()
                    {
                        match serde_json::to_vec_pretty(&preset)
                            .map_err(|e| e.to_string())
                            .and_then(|bytes| {
                                std::fs::write(path, bytes).map_err(|e| e.to_string())
                            }) {
                            Ok(()) => self.notify(t("Preset exported")),
                            Err(e) => self.notify(e),
                        }
                    }
                }
                if button(ui, t("Import preset")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Vesper preset", &["json"])
                        .pick_file()
                    {
                        self.import_preset(&path);
                    }
                }
            });
            if self.cfg.selected >= 24 {
                ui.add_space(18.);
                if ui
                    .add(Button::new(
                        RichText::new(t("Delete custom voice")).color(RED),
                    ))
                    .clicked()
                {
                    let i = self.cfg.selected;
                    self.all.remove(i);
                    self.cfg.custom.remove(i - 24);
                    self.select(1);
                    self.notify(t("Voice deleted"));
                }
            }
        });
    }
    fn import_preset(&mut self, path: &Path) {
        match std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| serde_json::from_slice::<Preset>(&bytes).map_err(|e| e.to_string()))
        {
            Ok(mut preset) => {
                preset.params.sanitize();
                preset.category = "Custom".into();
                preset.name = preset.name.trim().chars().take(40).collect();
                if preset.name.is_empty() {
                    self.notify(t("Enter a voice name"));
                    return;
                }
                let base = preset.name.clone();
                let mut suffix = 2;
                while self
                    .all
                    .iter()
                    .any(|p| p.name.eq_ignore_ascii_case(&preset.name))
                {
                    preset.name = format!("{base} {suffix}");
                    suffix += 1;
                }
                self.cfg.custom.push(preset.clone());
                self.all.push(preset);
                self.select(self.all.len() - 1);
                self.notify(t("Preset imported"));
            }
            Err(error) => self.notify(format!("{}: {error}", t("Import failed"))),
        }
    }

    fn soundboard(&mut self, ui: &mut Ui) {
        self.header(ui, t("Soundboard"));
        ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                title(ui, t("Built-in sounds"), 18.);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button(ui, t("Stop clip")).clicked() {
                        if let Some(engine) = &self.engine {
                            let _ = engine.play.try_send(None);
                        }
                    }
                });
            });
            ui.add_space(16.);
            let names = [
                "Notification",
                "Lift off",
                "Level up",
                "Alert",
                "Impact",
                "UFO",
                "Confirm",
                "Power down",
            ];
            let cols = if ui.available_width() > 800. { 4 } else { 3 };
            let width = (ui.available_width() - (cols - 1) as f32 * 12.) / cols as f32;
            for row in (0..8).collect::<Vec<_>>().chunks(cols) {
                ui.horizontal(|ui| {
                    for &i in row {
                        let (r, res) = ui.allocate_exact_size(vec2(width, 112.), Sense::click());
                        ui.painter().rect_filled(
                            r,
                            10.,
                            if res.hovered() {
                                C::from_rgb(45, 40, 33)
                            } else {
                                CARD
                            },
                        );
                        glyph(
                            ui.painter(),
                            Rect::from_min_size(r.min + vec2(12., 10.), vec2(44., 44.)),
                            AMBER,
                            i,
                        );
                        ui.painter().text(
                            r.min + vec2(16., 78.),
                            Align2::LEFT_CENTER,
                            t(names[i]),
                            bold(16.),
                            TEXT,
                        );
                        ui.painter().text(
                            r.min + vec2(width - 12., 18.),
                            Align2::RIGHT_TOP,
                            format!("Ctrl {}", i + 1),
                            FontId::monospace(9.),
                            MUTED,
                        );
                        if res.clicked() {
                            self.play_sound(i);
                        }
                    }
                });
                ui.add_space(6.);
            }
            ui.add_space(20.);
            Frame::new()
                .fill(PANEL)
                .corner_radius(12.)
                .inner_margin(22.)
                .show(ui, |ui| {
                    title(ui, t("Your sounds"), 18.);
                    ui.add_space(12.);
                    ui.horizontal_wrapped(|ui| {
                        if button(ui, t("Add WAV")).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("WAV audio", &["wav"])
                                .pick_file()
                            {
                                self.import_wav(&path);
                            }
                        }
                        if button(ui, t("Open folder")).clicked() {
                            open_folder(&self.data.join("Sounds"));
                        }
                        if button(ui, t("Refresh")).clicked() {
                            self.refresh_files();
                        }
                    });
                    ui.add_space(10.);
                    label(ui, t("Drop WAV files here · up to 60 seconds"), 11., MUTED);
                    ui.add_space(14.);
                    let before = self.cfg.params.board;
                    control(ui, t("Clip volume"), &mut self.cfg.params.board, 0., 1., "");
                    if before != self.cfg.params.board {
                        self.modified();
                    }
                });
            ui.add_space(16.);
            for path in self.clips.clone() {
                ui.horizontal(|ui| {
                    label(
                        ui,
                        path.file_stem().unwrap_or_default().to_string_lossy(),
                        13.,
                        TEXT,
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if button(ui, t("Play")).clicked() {
                            self.play_file(&path);
                        }
                    });
                });
                ui.separator();
            }
        });
    }

    fn import_wav(&mut self, path: &Path) {
        match audio::load_wav(path, 48000) {
            Ok(_) => {
                let Some(name) = path.file_name() else {
                    return;
                };
                let dest = self.data.join("Sounds").join(name);
                if dest == path {
                    self.refresh_files();
                    return;
                }
                match std::fs::copy(path, dest) {
                    Ok(_) => {
                        self.refresh_files();
                        self.notify(t("WAV added"));
                    }
                    Err(e) => self.notify(e.to_string()),
                }
            }
            Err(e) => self.notify(e),
        }
    }
    fn recordings(&mut self, ui: &mut Ui) {
        self.header(ui, t("Recordings"));
        ui.horizontal(|ui| {
            if primary(
                ui,
                if self.shared.recording.load(Ordering::Relaxed) {
                    t("Stop recording")
                } else {
                    t("Record voice")
                },
                vec2(185., 40.),
            )
            .clicked()
            {
                self.record();
            }
            if button(ui, t("Open folder")).clicked() {
                open_folder(&self.data.join("Recordings"));
            }
            if button(ui, t("Refresh")).clicked() {
                self.refresh_files();
            }
        });
        ui.add_space(22.);
        ScrollArea::vertical().show(ui, |ui| {
            if self.files.is_empty() {
                ui.add_space(60.);
                title(ui, t("No recordings yet"), 25.);
                label(ui, t("Start processing and press Record."), 13., MUTED);
            }
            let files = self.files.clone();
            for p in files {
                Frame::new()
                    .fill(PANEL)
                    .corner_radius(8.)
                    .inner_margin(16.)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(40., 44.), Sense::hover());
                            glyph(ui.painter(), r, AMBER, 0);
                            ui.vertical(|ui| {
                                title(
                                    ui,
                                    &p.file_stem().unwrap_or_default().to_string_lossy(),
                                    14.,
                                );
                                let info = hound::WavReader::open(&p)
                                    .ok()
                                    .map(|w| {
                                        format!(
                                            "{:.1} s  ·  {} Hz  ·  16 bit",
                                            w.duration() as f32 / w.spec().sample_rate as f32,
                                            w.spec().sample_rate
                                        )
                                    })
                                    .unwrap_or_else(|| t("Still recording").into());
                                label(ui, info, 11., MUTED);
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if button(ui, t("Listen")).clicked() {
                                    let _ =
                                        std::process::Command::new("explorer.exe").arg(&p).spawn();
                                }
                                if button(ui, t("Add to soundboard")).clicked() {
                                    self.import_wav(&p);
                                }
                            });
                        });
                    });
                ui.add_space(7.);
            }
        });
    }
    fn settings(&mut self, ui: &mut Ui) {
        self.header(ui, t("Settings"));
        ScrollArea::vertical().show(ui,|ui|{
                ui.set_width(ui.available_width());
            let before=(self.cfg.input.clone(),self.cfg.output.clone(),self.cfg.monitor_device.clone(),self.cfg.buffer_ms,self.cfg.hotkeys,self.cfg.compact);
            Frame::new().fill(PANEL).corner_radius(12.).inner_margin(22.).show(ui,|ui|{
                ui.set_width(ui.available_width());
                ui.horizontal(|ui|{title(ui,t("Language"),18.);ui.with_layout(Layout::right_to_left(Align::Center),|ui|{self.language_picker(ui,"settings-language");});});
            });
            ui.add_space(14.);
            Frame::new().fill(PANEL).corner_radius(12.).inner_margin(22.).show(ui,|ui|{
                ui.set_width(ui.available_width());
                title(ui,t("Audio routing"),18.);ui.add_space(14.);
                let was=self.cfg.system_voice;
                row_toggle(ui,t("Use as Windows microphone"),&mut self.cfg.system_voice);
                if was!=self.cfg.system_voice{self.modified();if self.engine.is_some(){self.stop_engine();self.start_stop();}}
                ui.add_space(10.);
                label(ui,t(if self.route.is_some(){"System microphone active"}else if self.cfg.system_voice{"Starts with voice processing"}else{"Local monitoring and recording"}),12.,if self.route.is_some(){GREEN}else{MUTED});
                ui.add_space(12.);
                let cable=self.outputs.iter().any(|name|audio::is_virtual_output(name));
                ui.horizontal_wrapped(|ui|{
                    if !cable && primary(ui,t("Install VB-CABLE"),vec2(160.,36.)).clicked(){self.install_cable();}
                    if button(ui,t("Refresh devices")).clicked(){(self.inputs,self.outputs)=audio::devices();self.notify(t("Devices refreshed"));}
                    if self.engine.is_none() && button(ui,t("Restore microphone")).clicked(){match routing::recover(&self.data){Ok(true)=>self.notify(t("Microphone restored")),Ok(false)=>self.notify(t("No recovery needed")),Err(e)=>self.notify(e)}}
                });
                ui.add_space(8.);
                ui.collapsing(t("Connection help"),|ui|{
                    label(ui,t("Select CABLE Output in your game or browser. Restart microphone capture after switching."),12.,MUTED);
                    label(ui,t("Your previous Windows microphone is restored when processing stops."),12.,MUTED);
                    ui.hyperlink_to("VB-CABLE · VB-Audio · donationware ↗","https://vb-audio.com/Services/licensing.htm");
                    label(ui,t("Driver installation requires administrator permission and may require a restart."),11.,MUTED);
                });
            });
            ui.add_space(14.);
            Frame::new().fill(PANEL).corner_radius(12.).inner_margin(22.).show(ui,|ui|{
                ui.set_width(ui.available_width());
                title(ui,t("Audio devices"),18.);ui.add_space(16.);
                label(ui,t("Input microphone"),11.,MUTED);combo(ui,"in",&mut self.cfg.input,&self.inputs);ui.add_space(12.);
                label(ui,t("Processed output"),11.,MUTED);combo(ui,"out",&mut self.cfg.output,&self.outputs);ui.add_space(12.);
                label(ui,t("Monitor headphones"),11.,MUTED);combo(ui,"monitor",&mut self.cfg.monitor_device,&self.outputs);ui.add_space(12.);
                label(ui,t("Audio buffer"),11.,MUTED);
                ComboBox::from_id_salt("buffer").selected_text(format!("{} ms",self.cfg.buffer_ms)).width(200.).show_ui(ui,|ui|{for ms in [8,16,24,40,80]{ui.selectable_value(&mut self.cfg.buffer_ms,ms,format!("{ms} ms"));}}).response.on_hover_text(t("Increase the buffer if audio crackles. Pitch adds up to 43 ms of algorithmic delay."));
                if self.engine.is_some(){ui.add_space(14.);if primary(ui,t("Apply and restart"),vec2(170.,36.)).clicked(){self.stop_engine();self.start_stop();}}
            });
            ui.add_space(14.);
            Frame::new().fill(PANEL).corner_radius(12.).inner_margin(22.).show(ui,|ui|{
                ui.set_width(ui.available_width());
                title(ui,t("Preferences"),18.);ui.add_space(14.);
                row_toggle(ui,t("Global hotkeys"),&mut self.cfg.hotkeys);
                row_toggle(ui,t("Compact voice cards"),&mut self.cfg.compact);
                ui.add_space(12.);
                ui.horizontal_wrapped(|ui|{
                    if button(ui,t("Open app folder")).clicked(){open_folder(&self.data);}
                    if button(ui,t("Save settings")).clicked(){self.save();self.notify(t("Settings saved"));}
                });
                ui.add_space(8.);
                ui.collapsing(t("Diagnostics"),|ui|{
                    label(ui,format!("{}: {} · {}: {}",t("Underruns"),self.shared.xruns.load(Ordering::Relaxed),t("Recording drops"),self.shared.record_drops.load(Ordering::Relaxed)),12.,MUTED);
                    label(ui,self.data.display().to_string(),10.,MUTED);
                    label(ui,"F6 · FX   /   F7 · Mute   /   F8 · REC",11.,MUTED);
                    label(ui,concat!("VESPER ",env!("CARGO_PKG_VERSION")," · Rust / egui / WASAPI"),10.,MUTED);
                });
            });
            let after=(self.cfg.input.clone(),self.cfg.output.clone(),self.cfg.monitor_device.clone(),self.cfg.buffer_ms,self.cfg.hotkeys,self.cfg.compact);if before!=after{self.modified();}
        });
    }

    fn install_cable(&mut self) {
        let exe = std::env::current_exe().unwrap_or_default();
        let base = exe.parent().unwrap_or(Path::new("."));
        let candidates = [
            base.join("VirtualCable/VBCABLE_Setup_x64.exe"),
            base.join("../VirtualCable/VBCABLE_Setup_x64.exe"),
        ];
        if let Some(path) = candidates.iter().find(|p| p.exists()) {
            let script = format!(
                "Start-Process -FilePath '{}' -WorkingDirectory '{}' -Verb RunAs",
                path.display().to_string().replace('\'', "''"),
                path.parent()
                    .unwrap_or(Path::new("."))
                    .display()
                    .to_string()
                    .replace('\'', "''")
            );
            let mut cmd = std::process::Command::new("powershell.exe");
            cmd.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            match cmd.spawn() {
                Ok(_) => self.notify(t("Press Install Driver, then refresh devices.")),
                Err(e) => self.notify(e.to_string()),
            }
        } else {
            self.notify(t("VB-CABLE installer missing. Extract the full package or download it from vb-cable.com."));
        }
    }
    fn qa_capture(&mut self, ctx: &Context) {
        if self.qa.is_none() {
            return;
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|e| {
                if let Event::Screenshot { image, .. } = e {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = image {
            let pixels: Vec<_> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
            let path = crate::diagnostic_dir().join(format!("preview-{}.png", self.qa.unwrap()));
            let _ = image::save_buffer(
                path,
                &pixels,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
            );
            if self.qa_exit {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
        if self.started.elapsed() > Duration::from_secs(3) && !self.shot {
            self.shot = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
        }
    }
}
impl eframe::App for Studio {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        // Input events repaint immediately. Only a visible live signal needs
        // continuous animation; shortcuts wake the UI independently.
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        let interval = if self.engine.is_some() && !minimized {
            33
        } else {
            1000
        };
        ctx.request_repaint_after(Duration::from_millis(interval));
        crate::i18n::set_language(self.cfg.language);
        self.hotkeys(ctx);
        self.qa_capture(ctx);
        if self.shared.stream_failed.swap(false, Ordering::AcqRel) && self.engine.is_some() {
            if self.stop_engine() {
                self.notify(t("Audio device disconnected. Microphone restored; reconnect and start processing again."));
            }
        }
        if self.route_checked.elapsed() > Duration::from_secs(2) {
            self.route_checked = Instant::now();
            let error = self.route.as_ref().and_then(|g| g.ensure().err());
            if let Some(e) = error {
                self.stop_engine();
                self.notify(format!("System mode stopped: {e}"));
            }
        }
        if self.auto_audio && self.started.elapsed() > Duration::from_secs(1) {
            self.auto_audio = false;
            self.start_stop();
            if let Some(e) = &self.engine {
                let _ = e.record(self.data.join("Recordings/QA-audio-check.wav"));
                let _ = e.play.try_send(Some(audio::tone(2, e.rate)));
            }
        }
        let msg = self
            .shared
            .message
            .try_lock()
            .ok()
            .and_then(|mut m| m.take());
        if let Some(msg) = msg {
            self.notify(msg);
            self.refresh_files();
        }
        self.in_meter = f32::from_bits(self.shared.input_level.load(Ordering::Relaxed))
            .max(self.in_meter * 0.85);
        self.out_meter = f32::from_bits(self.shared.output_level.load(Ordering::Relaxed))
            .max(self.out_meter * 0.85);
        if self.engine.is_none() {
            self.in_meter = 0.;
            self.out_meter = 0.;
        }
        if self.engine.is_some() && self.page <= 1 && !minimized {
            if let Ok(w) = self.shared.waveform.try_lock() {
                self.wave = *w;
            }
        }
        if self.dirty && self.save_at.elapsed() > Duration::from_millis(700) {
            self.save();
        } else if self.dirty {
            ctx.request_repaint_after(
                Duration::from_millis(700).saturating_sub(self.save_at.elapsed()),
            );
        }
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            if let Some(p) = f.path {
                if p.extension().is_some_and(|s| s.eq_ignore_ascii_case("wav")) {
                    self.import_wav(&p);
                    self.page = 2;
                } else if p
                    .extension()
                    .is_some_and(|s| s.eq_ignore_ascii_case("json"))
                {
                    self.import_preset(&p);
                    self.page = 1;
                }
            }
        }
        self.transport(ctx);
        self.rail(ctx);
        if self.page <= 1 {
            self.inspector(ctx);
        }
        CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(BG)
                    .inner_margin(Margin::symmetric(28, 25)),
            )
            .show(ctx, |ui| match self.page {
                0 => self.library(ui),
                1 => self.studio(ui),
                2 => self.soundboard(ui),
                3 => self.recordings(ui),
                _ => self.settings(ui),
            });
        if !self.toast.is_empty() && self.toast_at.elapsed() < Duration::from_secs(7) {
            Area::new(Id::new("toast"))
                .anchor(Align2::CENTER_BOTTOM, vec2(65., -99.))
                .order(Order::Foreground)
                .show(ctx, |ui| {
                    Frame::new()
                        .fill(C::from_rgb(46, 43, 36))
                        .stroke(Stroke::new(1.0_f32, AMBER.gamma_multiply(0.4)))
                        .corner_radius(8.)
                        .inner_margin(Margin::symmetric(17, 12))
                        .show(ui, |ui| {
                            ui.set_max_width(730.);
                            label(ui, &self.toast, 12., TEXT);
                        });
                });
        }
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.stop_engine();
        self.save();
    }
}
fn scan(path: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(path)
        .map(|it| {
            it.filter_map(Result::ok)
                .map(|d| d.path())
                .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("wav")))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod localization_tests {
    use super::*;
    #[test]
    fn legacy_settings_keep_audio_parameters_and_default_to_english() {
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        value.as_object_mut().unwrap().remove("language");
        value["params"]["pitch"] = serde_json::json!(-7.5);
        value["favorites"] = serde_json::json!(["Velvet", "My voice"]);
        let settings: Settings = serde_json::from_value(value).unwrap();
        assert_eq!(settings.language, Language::En);
        assert_eq!(settings.params.pitch, -7.5);
        assert_eq!(settings.favorites, ["Velvet", "My voice"]);
    }
    #[test]
    fn changing_language_preserves_preset_identity_and_audio_parameters() {
        let mut settings = Settings::default();
        let params = settings.params;
        let voices = presets::factory();
        for language in Language::ALL {
            settings.language = language;
            let restored: Settings =
                serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();
            assert_eq!(restored.language, language);
            assert_eq!(restored.params, params);
            assert_eq!(voices[restored.selected].name, "Velvet");
            assert_eq!(
                voices.iter().filter(|p| p.category == "Character").count(),
                6
            );
        }
    }
}
fn open_folder(path: &Path) {
    let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
}
fn combo(ui: &mut Ui, id: &str, value: &mut String, values: &[String]) {
    ComboBox::from_id_salt(id)
        .selected_text(if value.is_empty() {
            t("No device")
        } else {
            value.as_str()
        })
        .width(ui.available_width().min(390.))
        .show_ui(ui, |ui| {
            for v in values {
                ui.selectable_value(value, v.clone(), v);
            }
        });
}
fn meter(ui: &mut Ui, v: f32, w: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 15.), Sense::hover());
    let db = 20. * v.max(1e-5).log10();
    let n = ((db + 60.) / 60.).clamp(0., 1.);
    for i in 0..16 {
        let rr = Rect::from_min_size(r.min + vec2(i as f32 * w / 16., 3.), vec2(w / 16. - 2., 9.));
        ui.painter().rect_filled(
            rr,
            1.,
            if (i as f32) < n * 16. {
                if i > 13 {
                    RED
                } else {
                    GREEN
                }
            } else {
                LINE
            },
        );
    }
}
fn nav_icon(p: &Painter, c: Pos2, k: usize, color: C) {
    let s = Stroke::new(1.4_f32, color);
    match k {
        0 => {
            for i in 0..3 {
                let x = c.x - 6. + i as f32 * 6.;
                let h = if i == 1 { 7. } else { 4. };
                p.line_segment([pos2(x, c.y - h), pos2(x, c.y + h)], s);
            }
        }
        1 => {
            for i in 0..3 {
                let x = c.x - 6. + i as f32 * 6.;
                p.line_segment([pos2(x, c.y - 7.), pos2(x, c.y + 7.)], s);
                p.circle_filled(pos2(x, c.y - 3. + i as f32 * 3.), 2.5, color);
            }
        }
        2 => {
            for x in 0..2 {
                for y in 0..2 {
                    p.rect_stroke(
                        Rect::from_min_size(
                            c + vec2(x as f32 * 9. - 8., y as f32 * 9. - 8.),
                            vec2(6., 6.),
                        ),
                        1.,
                        s,
                        StrokeKind::Inside,
                    );
                }
            }
        }
        3 => {
            p.circle_stroke(c, 7., s);
            p.circle_filled(c, 2.4, color);
        }
        _ => {
            p.circle_stroke(c, 7., s);
            p.circle_stroke(c, 2.4, s);
            for i in 0..6 {
                let a = i as f32 * std::f32::consts::TAU / 6.;
                p.line_segment(
                    [
                        c + vec2(a.cos(), a.sin()) * 7.,
                        c + vec2(a.cos(), a.sin()) * 10.,
                    ],
                    s,
                );
            }
        }
    }
}

pub fn self_test(dir: &Path) -> eframe::Result<()> {
    let _ = std::fs::create_dir_all(&dir);
    let mut report = format!(
        "VESPER {} — offline DSP verification\n",
        env!("CARGO_PKG_VERSION")
    );
    let all = presets::factory();
    let start = Instant::now();
    for pr in all {
        let mut dsp = Dsp::new(48000.);
        dsp.configure(pr.params);
        let path = dir.join(format!("test-{}.wav", pr.name));
        let mut wav = hound::WavWriter::create(
            path,
            hound::WavSpec {
                channels: 1,
                sample_rate: 48000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        let mut peak = 0_f32;
        let mut energy = 0_f64;
        for i in 0..48000 {
            let t = i as f32 / 48000.;
            let x = ((std::f32::consts::TAU * 170. * t).sin() * 0.18
                + (std::f32::consts::TAU * 340. * t).sin() * 0.08)
                * (1. - t * 0.5);
            let y = dsp.tick(x, true);
            assert!(y.is_finite() && y.abs() < 1.);
            peak = peak.max(y.abs());
            energy += (y * y) as f64;
            wav.write_sample((y * 32767.) as i16).unwrap();
        }
        wav.finalize().unwrap();
        assert!(energy > 0.01, "silent preset {}", pr.name);
        report.push_str(&format!(
            "PASS {:12} peak={:.4} rms={:.4}\n",
            pr.name,
            peak,
            (energy / 48000.).sqrt()
        ));
    }
    report.push_str(&format!(
        "24 x 1 second rendered in {:.3}s\n",
        start.elapsed().as_secs_f64()
    ));
    let (ins, outs) = audio::devices();
    report.push_str(&format!("Inputs: {ins:?}\nOutputs: {outs:?}\n"));
    std::fs::write(dir.join("verification.txt"), report).unwrap();
    Ok(())
}
