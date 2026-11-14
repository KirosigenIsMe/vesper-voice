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
    import: String,
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
        let data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Vesper");
        let _ = std::fs::create_dir_all(data.join("Recordings"));
        let _ = std::fs::create_dir_all(data.join("Sounds"));
        let mut load_error = None;
        match routing::recover(&data) {
            Ok(true) => {
                load_error =
                    Some("Прежний микрофон восстановлен после аварийного завершения Vesper".into())
            }
            Ok(false) => {}
            Err(e) => load_error = Some(e),
        }
        let mut cfg = match std::fs::read(data.join("settings.json")) {
            Ok(b) => match serde_json::from_slice::<Settings>(&b) {
                Ok(s) => s,
                Err(e) => {
                    load_error = Some(format!("Не удалось прочитать настройки: {e}"));
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        };
        cfg.params.sanitize();
        cfg.buffer_ms = cfg.buffer_ms.clamp(8, 100);
        for p in &mut cfg.custom {
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
            tab: 0,
            category: 0,
            search: String::new(),
            only_favs: false,
            data,
            toast: load_error.unwrap_or_default(),
            toast_at: Instant::now(),
            save_at: Instant::now(),
            dirty: false,
            name: String::new(),
            import: String::new(),
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
                self.notify(format!("Настройки не сохранены: {e}"));
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
                self.notify(format!("Не удалось восстановить микрофон Windows: {e}"));
            }
        }
        self.engine.take();
        self.wave = [0.; 512];
        self.record_at = None;
        restored
    }
    fn start_stop(&mut self) {
        if self.engine.is_some() {
            let restored = self.stop_engine();
            self.refresh_files();
            if restored {
                self.notify("Аудиодвижок остановлен. Прежний микрофон Windows восстановлен.");
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
                        self.notify("Подключи физический микрофон");
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
                    self.notify(if self.route.is_some(){"Vesper назначен микрофоном Windows и приложений связи. Перезапусти запись на сайте."}else{"Включён локальный режим: прослушивание и запись. Вход Windows не изменён."});
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
                    self.notify("Идёт запись обработанного голоса");
                }
                Err(e) => self.notify(e),
            }
        } else {
            self.notify("Сначала запусти аудиодвижок");
        }
    }
    fn play_sound(&mut self, kind: usize) {
        if let Some(e) = &self.engine {
            if e.play.try_send(Some(audio::tone(kind, e.rate))).is_err() {
                self.notify("Очередь звуков занята");
            }
        } else {
            self.notify("Сначала запусти аудиодвижок");
        }
    }
    fn play_file(&mut self, path: &Path) {
        if let Some(e) = &self.engine {
            match audio::load_wav(path, e.rate) {
                Ok(v) => {
                    let _ = e.play.try_send(Some(v));
                    self.notify("Клип отправлен на аудиовыход");
                }
                Err(err) => self.notify(err),
            }
        } else {
            self.notify("Сначала запусти аудиодвижок");
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
            .exact_width(194.)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(20, 24)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(26., 28.), Sense::hover());
                    for i in 0..3 {
                        let x = r.left() + 4. + i as f32 * 8.;
                        let h = if i == 1 { 22. } else { 13. };
                        ui.painter().line_segment(
                            [
                                pos2(x, r.center().y - h / 2.),
                                pos2(x, r.center().y + h / 2.),
                            ],
                            Stroke::new(3.0_f32, AMBER),
                        );
                    }
                    ui.label(RichText::new("VESPER").font(bold(21.)).color(TEXT));
                });
                label(ui, "VOICE STUDIO  /  01", 9., MUTED);
                ui.add_space(24.);
                label(ui, "РАБОЧЕЕ ПРОСТРАНСТВО", 9., MUTED);
                ui.add_space(12.);
                for (i, (s, icon)) in [
                    ("Голоса", 0),
                    ("Студия", 1),
                    ("Саундборд", 2),
                    ("Записи", 3),
                    ("Настройки", 4),
                ]
                .iter()
                .enumerate()
                {
                    let (r, res) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 36.), Sense::click());
                    let selected = self.page == i;
                    if selected || res.hovered() {
                        ui.painter().rect_filled(
                            r,
                            7.,
                            if selected {
                                C::from_rgb(48, 43, 36)
                            } else {
                                CARD
                            },
                        );
                    }
                    if selected {
                        ui.painter().rect_filled(
                            Rect::from_min_size(r.min + vec2(0., 11.), vec2(2., 21.)),
                            1.,
                            AMBER,
                        );
                    }
                    let color = if selected { AMBER } else { MUTED };
                    nav_icon(ui.painter(), r.min + vec2(19., 18.), *icon, color);
                    ui.painter().text(
                        r.min + vec2(40., 18.),
                        Align2::LEFT_CENTER,
                        *s,
                        FontId::proportional(13.),
                        if selected { TEXT } else { MUTED },
                    );
                    if res.clicked() {
                        self.page = i;
                        self.refresh_files();
                    }
                }
                ui.add_space(18.);
                label(ui, "ТВОЯ КОЛЛЕКЦИЯ", 9., MUTED);
                ui.add_space(8.);
                if ui
                    .add(
                        Button::new(
                            RichText::new(format!(
                                "☆   Избранное          {}",
                                self.cfg.favorites.len()
                            ))
                            .color(if self.only_favs {
                                AMBER
                            } else {
                                MUTED
                            }),
                        )
                        .frame(false),
                    )
                    .clicked()
                {
                    self.only_favs = !self.only_favs;
                    self.page = 0;
                }
                if ui
                    .add(
                        Button::new(
                            RichText::new(format!(
                                "+   Мои голоса           {}",
                                self.cfg.custom.len()
                            ))
                            .color(MUTED),
                        )
                        .frame(false),
                    )
                    .clicked()
                {
                    self.category = 5;
                    self.page = 0;
                    self.only_favs = false;
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    label(ui, "Локально. Без аккаунта.", 10., MUTED);
                    ui.add_space(4.);
                    ui.horizontal(|ui| {
                        ui.painter()
                            .circle_filled(ui.cursor().min + vec2(3., 7.), 3., GREEN);
                        ui.add_space(12.);
                        label(ui, "RUST  /  NATIVE AUDIO", 9., MUTED);
                    });
                    ui.add_space(12.);
                    label(ui, "F8  Запись", 10., MUTED);
                    label(ui, "F7  Микрофон", 10., MUTED);
                    label(ui, "F6  Эффекты", 10., AMBER);
                    ui.add_space(4.);
                });
            });
    }
    fn transport(&mut self, ctx: &Context) {
        TopBottomPanel::bottom("transport")
            .exact_height(86.)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(24, 17))
                    .stroke(Stroke::new(1.0_f32, LINE)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let running = self.engine.is_some();
                    if primary(
                        ui,
                        if running {
                            "■   Остановить"
                        } else {
                            "▶   Включить голос"
                        },
                        vec2(166., 44.),
                    )
                    .clicked()
                    {
                        self.start_stop();
                    }
                    ui.add_space(12.);
                    let mut fx = self.shared.enabled.load(Ordering::Relaxed);
                    ui.vertical(|ui| {
                        label(ui, "ЭФФЕКТЫ", 9., MUTED);
                        if toggle(ui, &mut fx).changed() {
                            self.shared.enabled.store(fx, Ordering::Relaxed);
                        }
                    });
                    ui.add_space(12.);
                    let muted = self.shared.muted.load(Ordering::Relaxed);
                    if ui
                        .add(
                            Button::new(
                                RichText::new(if muted {
                                    "Микрофон выкл."
                                } else {
                                    "Микрофон вкл."
                                })
                                .color(if muted {
                                    RED
                                } else {
                                    TEXT
                                }),
                            )
                            .min_size(vec2(128., 40.)),
                        )
                        .clicked()
                    {
                        self.shared.muted.store(!muted, Ordering::Relaxed);
                    }
                    ui.add_space(8.);
                    let old = self.cfg.monitor;
                    ui.vertical(|ui| {
                        label(ui, "СЛЫШАТЬ СЕБЯ", 9., MUTED);
                        toggle(ui, &mut self.cfg.monitor);
                    });
                    if old != self.cfg.monitor {
                        self.modified();
                        if running {
                            self.stop_engine();
                            self.start_stop();
                        }
                    }
                    ui.add_space(16.);
                    ui.vertical(|ui| {
                        label(ui, "ВХОД", 9., MUTED);
                        meter(ui, self.in_meter, 92.);
                    });
                    ui.vertical(|ui| {
                        label(ui, "ВЫХОД", 9., MUTED);
                        meter(ui, self.out_meter, 92.);
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let rec = self.shared.recording.load(Ordering::Relaxed);
                        let txt = if let Some(t) = self.record_at {
                            format!(
                                "■  {:02}:{:02}",
                                t.elapsed().as_secs() / 60,
                                t.elapsed().as_secs() % 60
                            )
                        } else {
                            "●  Запись".into()
                        };
                        if ui
                            .add(
                                Button::new(RichText::new(txt).color(if rec { RED } else { TEXT }))
                                    .min_size(vec2(112., 40.)),
                            )
                            .clicked()
                        {
                            self.record();
                        }
                        ui.vertical(|ui| {
                            ui.add_space(3.);
                            label(
                                ui,
                                if running {
                                    "ДВИЖОК РАБОТАЕТ"
                                } else {
                                    "ОЖИДАНИЕ ЗАПУСКА"
                                },
                                9.,
                                if running { GREEN } else { MUTED },
                            );
                            let text = if let Some(e) = &self.engine {
                                format!("{} Hz · буфер {:.0} ms", e.rate, e.buffered_ms())
                            } else {
                                "WASAPI · локальная обработка".into()
                            };
                            label(ui, text, 10., MUTED);
                        });
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
                    label(ui, "ПАРАМЕТРЫ ГОЛОСА", 10., MUTED);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Сброс").clicked() {
                            self.cfg.params = self.all[self.cfg.selected].params;
                            self.modified();
                        }
                    });
                });
                ui.add_space(12.);
                title(ui, &self.all[self.cfg.selected].name, 24.);
                label(
                    ui,
                    if self.cfg.params != self.all[self.cfg.selected].params {
                        "Изменённый пресет"
                    } else {
                        "Готовый пресет"
                    },
                    11.,
                    AMBER,
                );
                ui.add_space(17.);
                ui.horizontal(|ui| {
                    for (i, t) in ["Тембр", "Эффекты", "Динамика"].iter().enumerate()
                    {
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
                                section(ui, "ВЫСОТА И ДВИЖЕНИЕ");
                                control(ui, "Высота тона", &mut p.pitch, -12., 12., " st");
                                control(ui, "Глубина вибрато", &mut p.vibrato, 0., 1., "");
                                control(ui, "Скорость вибрато", &mut p.vibrato_hz, 0.5, 12., " Hz");
                                section(ui, "ТРЁХПОЛОСНЫЙ ЭКВАЛАЙЗЕР");
                                control(ui, "Низкие · 160 Hz", &mut p.bass, -12., 12., " dB");
                                control(ui, "Середина · 1.1 kHz", &mut p.body, -12., 12., " dB");
                                control(ui, "Воздух · 4.8 kHz", &mut p.air, -12., 12., " dB");
                                control(ui, "Срез высоких", &mut p.lowpass, 500., 20000., " Hz");
                                section(ui, "БАЛАНС");
                                control(ui, "Сухой / обработанный", &mut p.mix, 0., 1., "");
                            }
                            1 => {
                                section(ui, "ТЕКСТУРА");
                                control(ui, "Насыщение", &mut p.drive, 0., 1., "");
                                control(ui, "Роботизация", &mut p.robot, 0., 1., "");
                                control(ui, "Частота модулятора", &mut p.ring_hz, 10., 600., " Hz");
                                control(ui, "Битовая глубина", &mut p.bits, 4., 16., " bit");
                                section(ui, "ПРОСТРАНСТВО");
                                control(ui, "Реверберация", &mut p.reverb, 0., 0.8, "");
                                control(ui, "Размер комнаты", &mut p.room, 0., 1., "");
                                control(ui, "Уровень эха", &mut p.delay, 0., 0.85, "");
                                control(ui, "Время эха", &mut p.delay_ms, 20., 650., " ms");
                                control(ui, "Обратная связь", &mut p.feedback, 0., 0.8, "");
                            }
                            _ => {
                                section(ui, "УРОВЕНЬ СИГНАЛА");
                                control(ui, "Усиление входа", &mut p.input, -24., 24., " dB");
                                control(ui, "Уровень выхода", &mut p.output, -24., 12., " dB");
                                section(ui, "ГЕЙТ И КОМПРЕССОР");
                                control(ui, "Порог гейта", &mut p.gate, -80., -15., " dB");
                                control(ui, "Порог компрессора", &mut p.threshold, -40., 0., " dB");
                                control(ui, "Степень сжатия", &mut p.ratio, 1., 12., ":1");
                                control(ui, "Атака", &mut p.attack, 1., 80., " ms");
                                control(ui, "Восстановление", &mut p.release, 20., 600., " ms");
                                control(ui, "Деэссер", &mut p.deess, 0., 1., "");
                                section(ui, "САУНДБОРД");
                                control(ui, "Громкость клипов", &mut p.board, 0., 1., "");
                            }
                        }
                    });
                if before != self.cfg.params {
                    self.modified();
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    if button(ui, "+   Сохранить свой голос").clicked() {
                        self.page = 1;
                        self.name = format!("{} Custom", self.all[self.cfg.selected].name);
                    }
                    label(ui, "Настройки сохраняются автоматически", 10., MUTED);
                    ui.separator();
                });
            });
    }
    fn header(&mut self, ui: &mut Ui, t: &str, sub: &str) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                title(ui, t, 27.);
                label(ui, sub, 12., MUTED);
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let r =
                    ui.add(
                        Button::new(
                            RichText::new(if self.engine.is_some() {
                                "●  LIVE"
                            } else {
                                "○  STANDBY"
                            })
                            .monospace()
                            .size(10.)
                            .color(if self.engine.is_some() { GREEN } else { MUTED }),
                        )
                        .fill(PANEL)
                        .corner_radius(20.),
                    );
                r.on_hover_text("Микрофон включается кнопкой внизу");
            });
        });
        ui.add_space(22.);
    }
    fn scope(&self, ui: &mut Ui, height: f32, hero: bool) {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        ui.painter().rect_filled(r, 10., PANEL);
        let p = ui.painter();
        let left = if hero {
            r.left() + 225.
        } else {
            r.left() + 16.
        };
        let area = Rect::from_min_max(pos2(left, r.top() + 20.), r.max - vec2(20., 25.));
        for j in 0..5 {
            let y = area.top() + area.height() * j as f32 / 4.;
            p.line_segment(
                [pos2(area.left(), y), pos2(area.right(), y)],
                Stroke::new(0.6_f32, LINE.gamma_multiply(0.5)),
            );
        }
        for j in 0..10 {
            let x = area.left() + area.width() * j as f32 / 9.;
            p.line_segment(
                [pos2(x, area.top()), pos2(x, area.bottom())],
                Stroke::new(0.6_f32, LINE.gamma_multiply(0.5)),
            );
        }
        if hero {
            p.text(
                r.min + vec2(20., 22.),
                Align2::LEFT_TOP,
                "СЕЙЧАС В СТУДИИ",
                FontId::proportional(9.),
                MUTED,
            );
            p.text(
                r.min + vec2(20., 48.),
                Align2::LEFT_TOP,
                &self.all[self.cfg.selected].name,
                bold(30.),
                TEXT,
            );
            p.text(
                r.min + vec2(20., 94.),
                Align2::LEFT_TOP,
                if self.engine.is_some() {
                    "Живой сигнал"
                } else {
                    "Готов к твоему голосу"
                },
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
        p.add(Shape::line(pts, Stroke::new(1.5_f32, AMBER)));
        p.text(
            pos2(area.right(), r.bottom() - 12.),
            Align2::RIGHT_BOTTOM,
            if self.engine.is_some() {
                "OUTPUT WAVEFORM"
            } else {
                "МИКРОФОН ОТКЛЮЧЁН"
            },
            FontId::monospace(8.),
            MUTED,
        );
    }
    fn library(&mut self, ui: &mut Ui) {
        self.header(ui, "Голоса", "Знакомый голос. Совсем другой характер.");
        self.scope(ui, 134., true);
        if self.route.is_some() {
            ui.add_space(8.);
            ui.horizontal(|ui| {
                label(
                    ui,
                    "●  Голос Vesper — основной микрофон Windows",
                    10.,
                    GREEN,
                );
                if ui
                    .add(
                        Button::new(RichText::new("Настройки ›").size(10.).color(AMBER))
                            .frame(false),
                    )
                    .clicked()
                {
                    self.page = 4;
                }
            });
        } else if !self.outputs.iter().any(|s| audio::is_virtual_output(s)) {
            ui.add_space(8.);
            ui.horizontal(|ui| {
                label(
                    ui,
                    "Для голоса в Discord и играх нужен VB-CABLE.",
                    10.,
                    MUTED,
                );
                if ui
                    .add(
                        Button::new(RichText::new("Подключить ›").size(10.).color(AMBER))
                            .frame(false),
                    )
                    .clicked()
                {
                    self.page = 4;
                }
            });
        }
        ui.add_space(19.);
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut self.search)
                    .hint_text("Поиск голоса…")
                    .desired_width((ui.available_width() - 125.).max(150.))
                    .margin(vec2(12., 9.)),
            );
            if button(
                ui,
                if self.only_favs {
                    "★ Избранное"
                } else {
                    "☆ Избранное"
                },
            )
            .clicked()
            {
                self.only_favs = !self.only_favs;
            }
        });
        ui.add_space(8.);
        ui.horizontal_wrapped(|ui| {
            for (i, t) in ["Все", "Характер", "Синтез", "Атмосфера", "Ретро", "Мои"]
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
        let cats = ["", "Характер", "Синтез", "Атмосфера", "Ретро", "Мои"];
        let search = self.search.to_lowercase();
        let visible: Vec<_> = self
            .all
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                (self.category == 0 || p.category == cats[self.category])
                    && (!self.only_favs || self.cfg.favorites.contains(&p.name))
                    && (search.is_empty()
                        || format!("{} {} {}", p.name, p.desc, p.category)
                            .to_lowercase()
                            .contains(&search))
            })
            .map(|(i, _)| i)
            .collect();
        ui.horizontal(|ui| {
            label(ui, format!("{} ГОЛОСОВ", visible.len()), 9., MUTED);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                label(ui, "Выбери пресет · настрой под себя", 10., MUTED);
            });
        });
        ui.add_space(5.);
        ScrollArea::vertical()
            .id_salt("voices")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if visible.is_empty() {
                    ui.add_space(40.);
                    title(ui, "Пока ничего нет", 20.);
                    label(
                        ui,
                        "Измени поиск или сохрани свой пресет в Студии.",
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
    fn preset_card(&mut self, ui: &mut Ui, i: usize, w: f32) {
        let p = &self.all[i];
        let selected = i == self.cfg.selected;
        let color = C::from_rgb(p.color[0], p.color[1], p.color[2]);
        let h = if self.cfg.compact { 130. } else { 156. };
        let (r, res) = ui.allocate_exact_size(vec2(w, h), Sense::click());
        let painter = ui.painter();
        painter.rect_filled(
            r,
            9.,
            if selected {
                C::from_rgb(42, 38, 32)
            } else if res.hovered() {
                C::from_rgb(37, 38, 41)
            } else {
                CARD
            },
        );
        painter.rect_stroke(
            r,
            9.,
            Stroke::new(
                1.0_f32,
                if selected {
                    AMBER.gamma_multiply(0.8)
                } else {
                    LINE.gamma_multiply(0.7)
                },
            ),
            StrokeKind::Inside,
        );
        glyph(
            painter,
            Rect::from_min_size(r.min + vec2(12., 10.), vec2(53., 53.)),
            color,
            p.shape,
        );
        painter.text(
            r.min + vec2(16., if self.cfg.compact { 72. } else { 84. }),
            Align2::LEFT_TOP,
            &p.name,
            bold(19.),
            TEXT,
        );
        let galley = painter.layout(p.desc.clone(), FontId::proportional(10.5), MUTED, w - 30.);
        painter.galley(
            r.min + vec2(16., if self.cfg.compact { 100. } else { 113. }),
            galley,
            MUTED,
        );
        painter.text(
            r.min + vec2(76., 34.),
            Align2::LEFT_CENTER,
            p.category.to_uppercase(),
            FontId::proportional(8.),
            MUTED,
        );
        let star = Rect::from_min_size(pos2(r.right() - 34., r.top() + 10.), vec2(26., 28.));
        let fav = self.cfg.favorites.contains(&p.name);
        painter.text(
            star.center(),
            Align2::CENTER_CENTER,
            if fav { "★" } else { "☆" },
            FontId::proportional(19.),
            if fav {
                AMBER
            } else {
                MUTED.gamma_multiply(0.6)
            },
        );
        if selected {
            painter.circle_filled(pos2(r.right() - 18., r.bottom() - 16.), 3., AMBER);
        }
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
        res.on_hover_text("Выбрать голос. Параметры появятся справа.");
    }
    fn studio(&mut self, ui: &mut Ui) {
        self.header(
            ui,
            "Лаборатория голоса",
            "Собери собственное звучание из 26 параметров.",
        );
        ScrollArea::vertical().show(ui,|ui|{
            self.scope(ui,190.,false);ui.add_space(22.);title(ui,&self.all[self.cfg.selected].name,34.);label(ui,&self.all[self.cfg.selected].desc,13.,MUTED);ui.add_space(22.);
            Frame::new().fill(PANEL).corner_radius(10.).inner_margin(22.).show(ui,|ui|{label(ui,"ЦЕПОЧКА ОБРАБОТКИ",10.,MUTED);ui.add_space(15.);ui.horizontal_wrapped(|ui|{for (i,s)in ["Гейт","Pitch","EQ","Компрессор","Текстура","Пространство","Лимитер"].iter().enumerate(){if i>0{label(ui,"›",13.,MUTED);}ui.add(Button::new(RichText::new(*s).size(11.).color(AMBER)).fill(CARD).stroke(Stroke::new(1.0_f32,LINE)).corner_radius(5.));}});ui.add_space(12.);label(ui,"Меняй тембр, эффекты и динамику в панели справа.",12.,MUTED);});
            ui.add_space(24.);title(ui,"Сохрани свой характер",20.);label(ui,"Все параметры останутся в твоей личной библиотеке.",12.,MUTED);ui.add_space(12.);ui.horizontal(|ui|{ui.add(TextEdit::singleline(&mut self.name).hint_text("Название голоса").desired_width(230.).margin(vec2(12.,10.)));if primary(ui,"Сохранить голос",vec2(135.,38.)).clicked(){let n=self.name.trim().chars().take(40).collect::<String>();if n.is_empty(){self.notify("Введи название голоса");}else if self.all.iter().any(|p|p.name.eq_ignore_ascii_case(&n)){self.notify("Такое имя уже есть. Выбери другое.");}else{let old=&self.all[self.cfg.selected];let p=Preset{name:n,desc:"Твой собственный голос.".into(),category:"Мои".into(),color:old.color,shape:old.shape,params:self.cfg.params};self.cfg.custom.push(p.clone());self.all.push(p);self.cfg.selected=self.all.len()-1;self.modified();self.notify("Новый голос сохранён в разделе «Мои»");self.name.clear();}}});
            ui.add_space(18.);ui.horizontal(|ui|{if button(ui,"Экспорт пресета").clicked(){let pr=&self.all[self.cfg.selected];let mut pr=pr.clone();pr.params=self.cfg.params;let path=self.data.join(format!("Preset_{}.json",SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()));match serde_json::to_vec_pretty(&pr).map_err(|e|e.to_string()).and_then(|b|std::fs::write(&path,b).map_err(|e|e.to_string())){Ok(())=>{open_folder(&self.data);self.notify(format!("Пресет сохранён: {}",path.display()));},Err(e)=>self.notify(e)}}if button(ui,"Импорт JSON").clicked(){if let Some(p)=rfd::FileDialog::new().add_filter("Vesper preset",&["json"]).pick_file(){self.import=p.display().to_string();}match std::fs::read(self.import.trim().trim_matches('"')).map_err(|e|e.to_string()).and_then(|b|serde_json::from_slice::<Preset>(&b).map_err(|e|e.to_string())){Ok(mut p)=>{p.params.sanitize();p.category="Мои".into();p.name=p.name.chars().take(40).collect();if p.name.trim().is_empty(){self.notify("В пресете нет имени");}else{if self.all.iter().any(|x|x.name==p.name){p.name=format!("{} {}",p.name,self.all.len());}self.cfg.custom.push(p.clone());self.all.push(p);self.select(self.all.len()-1);self.notify("Пресет импортирован");}},Err(e)=>self.notify(format!("Импорт: {e}"))}}});ui.add(TextEdit::singleline(&mut self.import).hint_text("Путь к .json для импорта / перетащи файл в окно").desired_width(f32::INFINITY));
            if self.cfg.selected>=24{ui.add_space(18.);if ui.add(Button::new(RichText::new("Удалить этот личный пресет").color(RED))).clicked(){let i=self.cfg.selected;self.all.remove(i);self.cfg.custom.remove(i-24);self.select(1);self.notify("Личный пресет удалён");}}
            ui.add_space(22.);label(ui,"Pitch меняет высоту и тембр вместе. Это DSP-эффекты, без нейросетевой имитации конкретных людей.",11.,MUTED);
        });
    }
    fn soundboard(&mut self, ui: &mut Ui) {
        self.header(ui, "Саундборд", "Звуки в твоём эфире — одним нажатием.");
        ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                label(ui, "8 ВСТРОЕННЫХ ЗВУКОВ", 10., MUTED);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button(ui, "■  Остановить клип").clicked() {
                        if let Some(e) = &self.engine { let _ = e.play.try_send(None); }
                    }
                });
            });
            ui.add_space(14.);
            let names = [("Notification", "Мягкий сигнал"), ("Lift off", "Электронный подъём"),
                ("Level up", "Маленькая победа"), ("Alert", "Пульсирующий сигнал"),
                ("Impact", "Низкий удар"), ("UFO", "Неземная частота"),
                ("Confirm", "Короткое подтверждение"), ("Power down", "Плавное выключение")];
            let cols=if ui.available_width()>800. {4} else {3};
            let w = (ui.available_width() - (cols-1) as f32*10.) / cols as f32;
            for row in (0..8).collect::<Vec<_>>().chunks(cols) {
                ui.horizontal(|ui| {
                    for &i in row {
                        let (r, res) = ui.allocate_exact_size(vec2(w, 138.), Sense::click());
                        ui.painter().rect_filled(r, 9., if res.hovered() { C::from_rgb(45, 40, 33) } else { CARD });
                        glyph(ui.painter(), Rect::from_min_size(r.min + vec2(10., 8.), vec2(55., 55.)), AMBER, i);
                        ui.painter().text(r.min + vec2(16., 77.), Align2::LEFT_TOP, names[i].0, bold(17.), TEXT);
                        ui.painter().text(r.min + vec2(16., 105.), Align2::LEFT_TOP, names[i].1, FontId::proportional(11.), MUTED);
                        ui.painter().text(r.max - vec2(13., 116.), Align2::RIGHT_TOP, format!("CTRL {}", i + 1), FontId::monospace(9.), MUTED);
                        if res.clicked() { self.play_sound(i); }
                    }
                });
                ui.add_space(7.);
            }
            ui.add_space(20.);
            Frame::new().fill(PANEL).corner_radius(10.).inner_margin(20.).show(ui, |ui| {
                title(ui, "Твои звуки", 20.);
                label(ui, "Перетащи WAV в окно или выбери файл. До 60 секунд на клип.", 12., MUTED);
                ui.add_space(12.);
                ui.horizontal_wrapped(|ui| {
                    if button(ui, "+  Выбрать WAV").clicked() {
                        if let Some(p) = rfd::FileDialog::new().add_filter("WAV audio", &["wav"]).pick_file() { self.import_wav(&p); }
                    }
                    if button(ui, "Открыть папку звуков").clicked() { open_folder(&self.data.join("Sounds")); }
                    if button(ui, "Обновить").clicked() { self.refresh_files(); }
                });
                ui.add_space(6.);
                label(ui, "PCM / Float WAV · mono и stereo · до 60 секунд", 10., MUTED);
            });
            ui.add_space(18.);
            let clips = self.clips.clone();
            if clips.is_empty() { label(ui, "Личные клипы появятся здесь.", 12., MUTED); }
            for p in clips {
                ui.horizontal(|ui| {
                    label(ui, p.file_stem().unwrap_or_default().to_string_lossy(), 13., TEXT);
                    if button(ui, "▶  В эфир").clicked() { self.play_file(&p); }
                });
                ui.separator();
            }
            ui.add_space(20.);
            let before = self.cfg.params.board;
            control(ui, "Громкость саундборда", &mut self.cfg.params.board, 0., 1., "");
            if before != self.cfg.params.board { self.modified(); }
            label(ui, "Клипы смешиваются с голосом после эффектов. F7 отключает весь выход, включая клипы.", 11., MUTED);
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
                        self.notify("WAV добавлен в саундборд");
                    }
                    Err(e) => self.notify(e.to_string()),
                }
            }
            Err(e) => self.notify(e),
        }
    }
    fn recordings(&mut self, ui: &mut Ui) {
        self.header(ui, "Записи", "Твои голоса, сохранённые в WAV. Без облака.");
        ui.horizontal(|ui| {
            if primary(
                ui,
                if self.shared.recording.load(Ordering::Relaxed) {
                    "■   Завершить запись"
                } else {
                    "●   Записать голос"
                },
                vec2(185., 40.),
            )
            .clicked()
            {
                self.record();
            }
            if button(ui, "Открыть папку").clicked() {
                open_folder(&self.data.join("Recordings"));
            }
            if button(ui, "Обновить список").clicked() {
                self.refresh_files();
            }
        });
        ui.add_space(22.);
        ScrollArea::vertical().show(ui, |ui| {
            if self.files.is_empty() {
                ui.add_space(60.);
                title(ui, "Здесь будет твой первый дубль", 25.);
                label(
                    ui,
                    "Включи голос, выбери пресет и нажми «Записать голос».",
                    13.,
                    MUTED,
                );
                ui.add_space(14.);
                label(
                    ui,
                    "Запись сохраняется в PCM WAV, mono, 16 bit, с частотой микрофона.",
                    12.,
                    MUTED,
                );
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
                                            "{:.1} сек  ·  {} Hz  ·  16 bit",
                                            w.duration() as f32 / w.spec().sample_rate as f32,
                                            w.spec().sample_rate
                                        )
                                    })
                                    .unwrap_or_else(|| "Файл ещё записывается".into());
                                label(ui, info, 11., MUTED);
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if button(ui, "▶  Прослушать").clicked() {
                                    let _ =
                                        std::process::Command::new("explorer.exe").arg(&p).spawn();
                                }
                                if button(ui, "В саундборд").clicked() {
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
        self.header(
            ui,
            "Настройки",
            "Устройства, маршрутизация и поведение приложения.",
        );
        ScrollArea::vertical().show(ui,|ui|{
            let before=(self.cfg.input.clone(),self.cfg.output.clone(),self.cfg.monitor_device.clone(),self.cfg.buffer_ms,self.cfg.hotkeys,self.cfg.compact);
            Frame::new().fill(PANEL).corner_radius(10.).inner_margin(22.).show(ui,|ui|{
                title(ui,"Голос Vesper в системе",20.);ui.add_space(12.);
                let was=self.cfg.system_voice;
                row_toggle(ui,"Назначать Vesper основным микрофоном Windows",&mut self.cfg.system_voice);
                if was!=self.cfg.system_voice{self.modified();if self.engine.is_some(){self.stop_engine();self.start_stop();}}
                ui.add_space(8.);
                label(ui,if self.route.is_some(){"●  Активен · основной микрофон и микрофон связи переключены"}else if self.cfg.system_voice{"○  Готов к включению · требуется установленный VB-CABLE"}else{"○  Локальный режим · системный вход Windows не меняется"},12.,if self.route.is_some(){GREEN}else{AMBER});
                ui.add_space(8.);
                label(ui,"При включении обработка идёт в CABLE Output. При остановке возвращается прежний микрофон. Если сайт выбрал конкретный физический микрофон, переключи его один раз на «По умолчанию» или CABLE Output и заново запусти запись.",11.,MUTED);
                ui.add_space(10.);ui.horizontal_wrapped(|ui|{
                    if !self.outputs.iter().any(|s|s.to_lowercase().contains("cable input")){if primary(ui,"Установить VB-CABLE",vec2(175.,36.)).clicked(){self.install_cable();}}
                    else if self.engine.is_none()&&primary(ui,"Включить голос в системе",vec2(205.,36.)).clicked(){self.cfg.system_voice=true;self.start_stop();}
                    if button(ui,"Обновить устройства").clicked(){(self.inputs,self.outputs)=audio::devices();self.notify("Устройства обновлены");}
                });
                if self.engine.is_none(){ui.add_space(10.);if button(ui,"Восстановить прежний вход Windows").clicked(){match routing::recover(&self.data){Ok(true)=>self.notify("Прежний микрофон Windows восстановлен"),Ok(false)=>self.notify("Нет незавершённого переключения микрофона"),Err(e)=>self.notify(e)}}}
            });ui.add_space(18.);
            ui.columns(2,|cols|{
                Frame::new().fill(PANEL).corner_radius(10.).inner_margin(22.).show(&mut cols[0],|ui|{title(ui,"Аудиоустройства",20.);ui.add_space(14.);label(ui,"МИКРОФОН · ВХОД",10.,MUTED);combo(ui,"in",&mut self.cfg.input,&self.inputs);ui.add_space(10.);label(ui,"ОБРАБОТАННЫЙ ГОЛОС · ВЫХОД",10.,MUTED);combo(ui,"out",&mut self.cfg.output,&self.outputs);ui.add_space(10.);label(ui,"ПРОСЛУШИВАНИЕ · НАУШНИКИ",10.,MUTED);combo(ui,"monitor",&mut self.cfg.monitor_device,&self.outputs);ui.add_space(14.);label(ui,"БУФЕР ОБРАБОТКИ",10.,MUTED);ComboBox::from_id_salt("buffer").selected_text(format!("{} ms",self.cfg.buffer_ms)).width(200.).show_ui(ui,|ui|{for (ms,t)in [(8,"8 ms · минимальный"),(16,"16 ms · быстрый"),(24,"24 ms · сбалансированный"),(40,"40 ms · стабильный"),(80,"80 ms · совместимость")]{ui.selectable_value(&mut self.cfg.buffer_ms,ms,t);}});ui.add_space(9.);label(ui,"Это размер очереди, полная задержка зависит от устройства и выбранных эффектов. Физический выход звучит только при включённом прослушивании.",11.,MUTED);ui.add_space(15.);if button(ui,"Обновить устройства").clicked(){(self.inputs,self.outputs)=audio::devices();self.notify("Список аудиоустройств обновлён");}if self.engine.is_some(){ui.add_space(10.);if primary(ui,"Применить и перезапустить",vec2(235.,36.)).clicked(){self.stop_engine();self.start_stop();}}});
                Frame::new().fill(PANEL).corner_radius(10.).inner_margin(22.).show(&mut cols[1],|ui|{title(ui,"Виртуальный микрофон",20.);ui.add_space(12.);let cable=self.outputs.iter().any(|s|s.to_lowercase().contains("cable input"));label(ui,if cable{"●  VB-CABLE обнаружен"}else{"○  VB-CABLE ещё не установлен"},12.,if cable{GREEN}else{AMBER});ui.add_space(16.);for(n,t)in [("01","Установи VB-CABLE один раз и обнови устройства."),("02","Включи системный режим и нажми «Включить голос»."),("03","На сайте: микрофон «По умолчанию» либо CABLE Output. Перезапусти запись."),("04","При остановке Vesper вернёт прежний микрофон Windows.")]{ui.horizontal_top(|ui|{label(ui,n,11.,AMBER);ui.label(RichText::new(t).size(12.).color(TEXT));});ui.add_space(8.);}ui.add_space(8.);if button(ui,"Установить VB-CABLE").clicked(){self.install_cable();}ui.hyperlink_to("Официальный сайт VB-Audio ↗","https://vb-audio.com/Cable/");label(ui,"Установка драйвера требует прав администратора. После установки может понадобиться перезагрузка Windows.",11.,MUTED);});
            });ui.add_space(18.);
            Frame::new().fill(PANEL).corner_radius(10.).inner_margin(22.).show(ui,|ui|{title(ui,"Поведение",20.);ui.add_space(12.);row_toggle(ui,"Глобальные клавиши F6 / F7 / F8",&mut self.cfg.hotkeys);row_toggle(ui,"Компактные карточки голосов",&mut self.cfg.compact);ui.add_space(8.);label(ui,"Ctrl + 1…8 запускают встроенные звуки, когда окно Vesper активно.",11.,MUTED);ui.add_space(12.);ui.horizontal(|ui|{if button(ui,"Папка приложения").clicked(){open_folder(&self.data);}if button(ui,"Сохранить настройки").clicked(){self.save();self.notify("Настройки сохранены");}});label(ui,self.data.display().to_string(),10.,MUTED);});
            ui.add_space(18.);Frame::new().fill(PANEL).corner_radius(10.).inner_margin(22.).show(ui,|ui|{title(ui,"Состояние движка",18.);ui.add_space(9.);label(ui,format!("Сбои очереди: {}  ·  Пропущенные блоки записи: {}",self.shared.xruns.load(Ordering::Relaxed),self.shared.record_drops.load(Ordering::Relaxed)),12.,MUTED);label(ui,"Если звук прерывается, увеличь буфер. Pitch добавляет до 43 ms собственной задержки.",11.,MUTED);label(ui,"VESPER 1.2.0  ·  Rust / egui / WASAPI  ·  24 голоса / 26 параметров",10.,MUTED);});
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
                Ok(_) => self.notify(
                    "Открылся установщик VB-CABLE. Нажми Install Driver, затем обнови устройства.",
                ),
                Err(e) => self.notify(e.to_string()),
            }
        } else {
            self.notify("Установщик не найден рядом с exe. Распакуй полный комплект или скачай VB-CABLE с официального сайта.");
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
        self.hotkeys(ctx);
        self.qa_capture(ctx);
        if self.shared.stream_failed.swap(false, Ordering::AcqRel) && self.engine.is_some() {
            if self.stop_engine() {
                self.notify("Аудиоустройство отключилось. Прежний системный микрофон восстановлен; подключи устройство и включи голос снова.");
            }
        }
        if self.route_checked.elapsed() > Duration::from_secs(2) {
            self.route_checked = Instant::now();
            let error = self.route.as_ref().and_then(|g| g.ensure().err());
            if let Some(e) = error {
                self.stop_engine();
                self.notify(format!("Системный режим остановлен: {e}"));
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
                    self.import = p.display().to_string();
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
fn open_folder(path: &Path) {
    let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
}
fn combo(ui: &mut Ui, id: &str, value: &mut String, values: &[String]) {
    ComboBox::from_id_salt(id)
        .selected_text(if value.is_empty() {
            "Нет устройства"
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
