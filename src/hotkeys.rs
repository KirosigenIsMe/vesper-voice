use eframe::egui::Context;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

/// Global shortcuts must wake an idle/minimized window without repainting it
/// continuously. This small worker only polls three keys; UI work stays on the
/// main thread. Dropping the app stops and joins the worker.
pub struct Hotkeys {
    enabled: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    pending: Arc<AtomicU8>,
    worker: Option<JoinHandle<()>>,
}
impl Hotkeys {
    pub fn new(ctx: Context, enabled: bool) -> Self {
        let active = Arc::new(AtomicBool::new(enabled));
        let stop = Arc::new(AtomicBool::new(false));
        let pending = Arc::new(AtomicU8::new(0));
        let (a, s, p) = (active.clone(), stop.clone(), pending.clone());
        let worker = thread::Builder::new()
            .name("vesper-keys".into())
            .stack_size(128 * 1024)
            .spawn(move || {
                #[cfg(windows)]
                #[link(name = "user32")]
                unsafe extern "system" {
                    fn GetAsyncKeyState(vkey: i32) -> i16;
                }
                let mut previous = 0_u8;
                let mut initialized = false;
                while !s.load(Ordering::Acquire) {
                    let mut down = 0_u8;
                    let mut taps = 0_u8;
                    #[cfg(windows)]
                    for i in 0..3 {
                        let state = unsafe { GetAsyncKeyState(0x75 + i) };
                        if state < 0 {
                            down |= 1 << i;
                        }
                        // Catch taps shorter than the polling interval as well
                        // as physical held-key edges. Discard pre-launch taps.
                        if state & 1 != 0 {
                            taps |= 1 << i;
                        }
                    }
                    let pressed = (down & !previous) | taps;
                    previous = down;
                    if initialized && pressed != 0 && a.load(Ordering::Relaxed) {
                        p.fetch_or(pressed, Ordering::Release);
                        ctx.request_repaint();
                    }
                    initialized = true;
                    thread::sleep(Duration::from_millis(16));
                }
            })
            .expect("global shortcut worker");
        Self {
            enabled: active,
            stop,
            pending,
            worker: Some(worker),
        }
    }
    pub fn take(&self, enabled: bool) -> u8 {
        self.enabled.store(enabled, Ordering::Relaxed);
        self.pending.swap(0, Ordering::AcqRel)
    }
}
impl Drop for Hotkeys {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
