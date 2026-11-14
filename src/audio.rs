use crate::dsp::{Dsp, Params};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    thread,
};

const CAP: usize = 32768;
struct Queue {
    data: Box<[AtomicU32]>,
    read: AtomicUsize,
    write: AtomicUsize,
}
impl Queue {
    fn new() -> Self {
        Self {
            data: (0..CAP).map(|_| AtomicU32::new(0)).collect(),
            read: AtomicUsize::new(0),
            write: AtomicUsize::new(0),
        }
    }
    fn push(&self, x: f32) -> bool {
        let w = self.write.load(Ordering::Relaxed);
        if w.wrapping_sub(self.read.load(Ordering::Acquire)) >= CAP {
            return false;
        }
        self.data[w % CAP].store(x.to_bits(), Ordering::Relaxed);
        self.write.store(w.wrapping_add(1), Ordering::Release);
        true
    }
    fn pop(&self) -> Option<f32> {
        let r = self.read.load(Ordering::Relaxed);
        if r == self.write.load(Ordering::Acquire) {
            return None;
        }
        let v = f32::from_bits(self.data[r % CAP].load(Ordering::Relaxed));
        self.read.store(r.wrapping_add(1), Ordering::Release);
        Some(v)
    }
    fn len(&self) -> usize {
        let read = self.read.load(Ordering::Acquire);
        self.write
            .load(Ordering::Acquire)
            .wrapping_sub(read)
            .min(CAP)
    }
}
pub struct Shared {
    pub params: Mutex<Params>,
    pub enabled: AtomicBool,
    pub muted: AtomicBool,
    pub recording: AtomicBool,
    pub stream_failed: AtomicBool,
    pub input_level: AtomicU32,
    pub output_level: AtomicU32,
    pub waveform: Mutex<[f32; 512]>,
    pub xruns: AtomicUsize,
    pub record_drops: AtomicUsize,
    pub message: Mutex<Option<String>>,
}
impl Shared {
    pub fn new(p: Params) -> Arc<Self> {
        Arc::new(Self {
            params: Mutex::new(p),
            enabled: AtomicBool::new(true),
            muted: AtomicBool::new(false),
            recording: AtomicBool::new(false),
            stream_failed: AtomicBool::new(false),
            input_level: AtomicU32::new(0),
            output_level: AtomicU32::new(0),
            waveform: Mutex::new([0.; 512]),
            xruns: AtomicUsize::new(0),
            record_drops: AtomicUsize::new(0),
            message: Mutex::new(None),
        })
    }
    pub fn message(&self, s: String) {
        if let Ok(mut m) = self.message.try_lock() {
            *m = Some(s);
        }
    }
}
pub fn devices() -> (Vec<String>, Vec<String>) {
    let host = cpal::default_host();
    let ins = host
        .input_devices()
        .map(|ds| ds.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default();
    let outs = host
        .output_devices()
        .map(|ds| ds.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default();
    (ins, outs)
}
pub fn defaults() -> (String, String) {
    let h = cpal::default_host();
    (
        h.default_input_device()
            .and_then(|d| d.name().ok())
            .unwrap_or_default(),
        h.default_output_device()
            .and_then(|d| d.name().ok())
            .unwrap_or_default(),
    )
}
pub fn is_virtual_output(name: &str) -> bool {
    let n = name.to_lowercase();
    [
        "cable input",
        "cable-in",
        "voicemeeter",
        "virtual",
        "vb-audio",
    ]
    .iter()
    .any(|s| n.contains(s))
}
fn device(name: &str, input: bool) -> Result<cpal::Device, String> {
    let h = cpal::default_host();
    let ds = if input {
        h.input_devices()
    } else {
        h.output_devices()
    }
    .map_err(|e| e.to_string())?;
    ds.into_iter()
        .find(|d| d.name().is_ok_and(|n| n == name))
        .ok_or_else(|| format!("Device unavailable: {name}. Refresh the device list."))
}

enum RecMsg {
    Start(PathBuf, u32, SyncSender<Result<(), String>>),
    Frame([f32; 1024], usize),
    Stop,
}
pub struct Engine {
    input: Option<cpal::Stream>,
    outputs: Vec<cpal::Stream>,
    rec_tx: Option<SyncSender<RecMsg>>,
    writer: Option<thread::JoinHandle<()>>,
    pub play: SyncSender<Option<Arc<Vec<f32>>>>,
    pub rate: u32,
    shared: Arc<Shared>,
    queue: Arc<Queue>,
}
impl Drop for Engine {
    fn drop(&mut self) {
        self.shared.recording.store(false, Ordering::Relaxed);
        self.input.take();
        self.outputs.clear();
        if let Some(tx) = self.rec_tx.take() {
            let _ = tx.send(RecMsg::Stop);
            drop(tx);
        }
        if let Some(t) = self.writer.take() {
            let _ = t.join();
        }
    }
}
impl Engine {
    pub fn start(
        input: &str,
        output: &str,
        monitor: Option<&str>,
        buffer_ms: u32,
        shared: Arc<Shared>,
    ) -> Result<Self, String> {
        let id = device(input, true)?;
        let ic = id
            .default_input_config()
            .map_err(|e| format!("Microphone: {e}"))?;
        let rate = ic.sample_rate().0;
        let config: cpal::StreamConfig = ic.clone().into();
        let q = Arc::new(Queue::new());
        let mq = Arc::new(Queue::new());
        let target = (rate * buffer_ms / 1000) as usize;
        let mut outputs = Vec::new();
        outputs.push(build_output(
            output,
            rate,
            q.clone(),
            target,
            shared.clone(),
            is_virtual_output(output) || monitor == Some(output),
        )?);
        let has_monitor = monitor.is_some_and(|m| m != output);
        if let Some(m) = monitor.filter(|m| *m != output) {
            outputs.push(build_output(
                m,
                rate,
                mq.clone(),
                target,
                shared.clone(),
                true,
            )?);
        }
        let (rtx, rrx) = mpsc::sync_channel(128);
        let ws = shared.clone();
        let writer = thread::spawn(move || record_worker(rrx, ws));
        let (ptx, prx) = mpsc::sync_channel(16);
        let rec = rtx.clone();
        let s = shared.clone();
        let producer = q.clone();
        let mut processor = Processor {
            dsp: Dsp::new(rate as f32),
            shared: s.clone(),
            q: producer,
            mq: if has_monitor { Some(mq) } else { None },
            rec,
            play: prx,
            clip: None,
            clip_pos: 0,
            was_recording: false,
            scope: [0.; 512],
            scope_pos: 0,
            scope_tick: 0,
            scope_publish: 0,
        };
        if let Ok(p) = shared.params.lock() {
            processor.dsp.configure(*p);
        }
        let err_s = shared.clone();
        let err = move |e| {
            err_s.stream_failed.store(true, Ordering::Release);
            err_s.message(format!("Audio stream: {e}. Stop and restart processing."));
        };
        let stream = match ic.sample_format() {
            cpal::SampleFormat::F32 => id.build_input_stream(
                &config,
                move |d: &[f32], _| processor.process(d, config.channels as usize),
                err,
                None,
            ),
            cpal::SampleFormat::I16 => id.build_input_stream(
                &config,
                move |d: &[i16], _| processor.process(d, config.channels as usize),
                err,
                None,
            ),
            cpal::SampleFormat::U16 => id.build_input_stream(
                &config,
                move |d: &[u16], _| processor.process(d, config.channels as usize),
                err,
                None,
            ),
            f => return Err(format!("Unsupported microphone format: {f:?}")),
        }
        .map_err(|e| format!("Could not open microphone: {e}"))?;
        stream.play().map_err(|e| e.to_string())?;
        for o in &outputs {
            o.play().map_err(|e| e.to_string())?;
        }
        shared.xruns.store(0, Ordering::Relaxed);
        shared.stream_failed.store(false, Ordering::Release);
        shared.record_drops.store(0, Ordering::Relaxed);
        Ok(Self {
            input: Some(stream),
            outputs,
            rec_tx: Some(rtx),
            writer: Some(writer),
            play: ptx,
            rate,
            shared,
            queue: q,
        })
    }
    pub fn record(&self, path: PathBuf) -> Result<(), String> {
        let (ack, rx) = mpsc::sync_channel(1);
        self.rec_tx
            .as_ref()
            .unwrap()
            .send(RecMsg::Start(path, self.rate, ack))
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(std::time::Duration::from_secs(2))
            .map_err(|e| format!("Recording not ready: {e}"))??;
        self.shared.recording.store(true, Ordering::Release);
        Ok(())
    }
    pub fn stop_record(&self) {
        self.shared.recording.store(false, Ordering::Release);
    }
    pub fn buffered_ms(&self) -> f32 {
        self.queue.len() as f32 / self.rate as f32 * 1000.
    }
}
trait SampleIn: Copy {
    fn value(self) -> f32;
}
impl SampleIn for f32 {
    fn value(self) -> f32 {
        self
    }
}
impl SampleIn for i16 {
    fn value(self) -> f32 {
        self as f32 / 32768.
    }
}
impl SampleIn for u16 {
    fn value(self) -> f32 {
        (self as f32 - 32768.) / 32768.
    }
}
struct Processor {
    dsp: Dsp,
    shared: Arc<Shared>,
    q: Arc<Queue>,
    mq: Option<Arc<Queue>>,
    rec: SyncSender<RecMsg>,
    play: Receiver<Option<Arc<Vec<f32>>>>,
    clip: Option<Arc<Vec<f32>>>,
    clip_pos: usize,
    was_recording: bool,
    scope: [f32; 512],
    scope_pos: usize,
    scope_tick: usize,
    scope_publish: usize,
}
impl Processor {
    fn process<T: SampleIn>(&mut self, data: &[T], channels: usize) {
        if let Ok(p) = self.shared.params.try_lock() {
            self.dsp.configure(*p);
        }
        while let Ok(clip) = self.play.try_recv() {
            self.clip = clip;
            self.clip_pos = 0;
        }
        let enabled = self.shared.enabled.load(Ordering::Relaxed);
        let muted = self.shared.muted.load(Ordering::Relaxed);
        let recording = self.shared.recording.load(Ordering::Acquire);
        if self.was_recording && !recording {
            if self.rec.try_send(RecMsg::Stop).is_ok() {
                self.was_recording = false;
            }
        } else {
            self.was_recording = recording;
        }
        let board = self.dsp.board_gain();
        let mut block = [0.; 1024];
        let mut n = 0;
        let mut in_peak = 0_f32;
        let mut out_peak = 0_f32;
        for frame in data.chunks(channels) {
            let input = frame.iter().map(|v| v.value()).sum::<f32>() / channels as f32;
            in_peak = in_peak.max(input.abs());
            let mut y = self.dsp.tick(input, enabled);
            if let Some(clip) = &self.clip {
                if let Some(v) = clip.get(self.clip_pos) {
                    y += *v * board;
                    self.clip_pos += 1;
                } else {
                    self.clip = None;
                }
            }
            y = if muted { 0. } else { y.clamp(-0.99, 0.99) };
            out_peak = out_peak.max(y.abs());
            if !self.q.push(y) {
                self.shared.xruns.fetch_add(1, Ordering::Relaxed);
            }
            if let Some(q) = &self.mq {
                q.push(y);
            }
            self.scope_tick += 1;
            if self.scope_tick >= 8 {
                self.scope_tick = 0;
                self.scope[self.scope_pos] = y;
                self.scope_pos = (self.scope_pos + 1) % 512;
            }
            if recording {
                block[n] = y;
                n += 1;
                if n == 1024 {
                    if self.rec.try_send(RecMsg::Frame(block, n)).is_err() {
                        self.shared.record_drops.fetch_add(1, Ordering::Relaxed);
                    }
                    n = 0;
                }
            }
        }
        if recording && n > 0 && self.rec.try_send(RecMsg::Frame(block, n)).is_err() {
            self.shared.record_drops.fetch_add(1, Ordering::Relaxed);
        }
        self.shared
            .input_level
            .store(in_peak.to_bits(), Ordering::Relaxed);
        self.shared
            .output_level
            .store(out_peak.to_bits(), Ordering::Relaxed);
        self.scope_publish += data.len() / channels;
        let publish_interval = (self.dsp.sr as usize / 30).max(1);
        if self.scope_publish >= publish_interval {
            self.scope_publish %= publish_interval;
            if let Ok(mut wave) = self.shared.waveform.try_lock() {
                for (i, v) in wave.iter_mut().enumerate() {
                    *v = self.scope[(self.scope_pos + i) % 512];
                }
            }
        }
    }
}
struct Resampler {
    queue: Arc<Queue>,
    phase: f64,
    a: f32,
    b: f32,
    ready: bool,
    ratio: f64,
    target: usize,
    shared: Arc<Shared>,
    starved: bool,
}
impl Resampler {
    fn tick(&mut self) -> f32 {
        if !self.ready {
            if self.queue.len() < self.target {
                return 0.;
            }
            self.a = self.queue.pop().unwrap_or(0.);
            self.b = self.queue.pop().unwrap_or(0.);
            self.phase = 0.;
            self.ready = true;
            self.starved = false;
        }
        let y = self.a + (self.b - self.a) * self.phase as f32;
        let correction =
            ((self.queue.len() as f64 - self.target as f64) / self.target.max(1) as f64 * 0.001)
                .clamp(-0.006, 0.006);
        self.phase += self.ratio * (1. + correction);
        while self.phase >= 1. {
            self.phase -= 1.;
            self.a = self.b;
            match self.queue.pop() {
                Some(v) => self.b = v,
                None => {
                    self.ready = false;
                    if !self.starved {
                        self.shared.xruns.fetch_add(1, Ordering::Relaxed);
                        self.starved = true;
                    }
                    return 0.;
                }
            }
        }
        y
    }
}
fn build_output(
    name: &str,
    source_rate: u32,
    q: Arc<Queue>,
    target: usize,
    shared: Arc<Shared>,
    audible: bool,
) -> Result<cpal::Stream, String> {
    let d = device(name, false)?;
    let supported = d.default_output_config().map_err(|e| e.to_string())?;
    let config: cpal::StreamConfig = supported.clone().into();
    let channels = config.channels as usize;
    let mut r = Resampler {
        queue: q,
        phase: 0.,
        a: 0.,
        b: 0.,
        ready: false,
        ratio: source_rate as f64 / config.sample_rate.0 as f64,
        target: target.max(128),
        shared: shared.clone(),
        starved: false,
    };
    let err = move |e| {
        shared.stream_failed.store(true, Ordering::Release);
        shared.message(format!("Output device: {e}"));
    };
    match supported.sample_format() {
        cpal::SampleFormat::F32 => d.build_output_stream(
            &config,
            move |data: &mut [f32], _| {
                for frame in data.chunks_mut(channels) {
                    let y = r.tick() * if audible { 1. } else { 0. };
                    frame.fill(y);
                }
            },
            err,
            None,
        ),
        cpal::SampleFormat::I16 => d.build_output_stream(
            &config,
            move |data: &mut [i16], _| {
                for frame in data.chunks_mut(channels) {
                    let y = (r.tick() * if audible { 32767. } else { 0. }) as i16;
                    frame.fill(y);
                }
            },
            err,
            None,
        ),
        cpal::SampleFormat::U16 => d.build_output_stream(
            &config,
            move |data: &mut [u16], _| {
                for frame in data.chunks_mut(channels) {
                    let y = ((r.tick() * if audible { 1. } else { 0. } + 1.) * 32767.5) as u16;
                    frame.fill(y);
                }
            },
            err,
            None,
        ),
        f => return Err(format!("Unsupported output format: {f:?}")),
    }
    .map_err(|e| format!("Output \"{name}\": {e}"))
}
fn record_worker(rx: Receiver<RecMsg>, s: Arc<Shared>) {
    let mut wav: Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>> = None;
    let mut current = PathBuf::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            RecMsg::Start(path, rate, ack) => {
                if let Some(w) = wav.take() {
                    let _ = w.finalize();
                }
                current = path.clone();
                match hound::WavWriter::create(
                    &path,
                    hound::WavSpec {
                        channels: 1,
                        sample_rate: rate,
                        bits_per_sample: 16,
                        sample_format: hound::SampleFormat::Int,
                    },
                ) {
                    Ok(w) => {
                        wav = Some(w);
                        let _ = ack.send(Ok(()));
                    }
                    Err(e) => {
                        s.recording.store(false, Ordering::Relaxed);
                        let _ = ack.send(Err(e.to_string()));
                        s.message(format!("Recording: {e}"));
                    }
                }
            }
            RecMsg::Frame(block, n) => {
                if let Some(w) = wav.as_mut() {
                    let mut err = None;
                    for x in &block[..n] {
                        if let Err(e) = w.write_sample((x.clamp(-1., 1.) * 32767.) as i16) {
                            err = Some(e);
                            break;
                        }
                    }
                    if let Some(e) = err {
                        s.recording.store(false, Ordering::Relaxed);
                        s.message(format!("Recording error: {e}"));
                        if let Some(w) = wav.take() {
                            let _ = w.finalize();
                        }
                    }
                }
            }
            RecMsg::Stop => {
                if let Some(w) = wav.take() {
                    match w.finalize() {
                        Ok(()) => s.message(format!("Recording saved: {}", current.display())),
                        Err(e) => s.message(format!("Could not save WAV: {e}")),
                    }
                }
            }
        }
    }
    if let Some(w) = wav {
        let _ = w.finalize();
    }
}

pub fn load_wav(path: &Path, rate: u32) -> Result<Arc<Vec<f32>>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| format!("WAV: {e}"))?;
    let spec = reader.spec();
    if spec.channels == 0 || spec.sample_rate == 0 {
        return Err("Invalid WAV file".into());
    }
    if reader.duration() > spec.sample_rate * 60 {
        return Err("Clips must be 60 seconds or shorter".into());
    }
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|x| x.map(|v| if v.is_finite() { v.clamp(-1., 1.) } else { 0. }))
            .collect::<Result<_, _>>(),
        hound::SampleFormat::Int => {
            let scale = 2_f32.powi(spec.bits_per_sample as i32 - 1);
            reader
                .samples::<i32>()
                .map(|x| x.map(|v| v as f32 / scale))
                .collect::<Result<_, _>>()
        }
    }
    .map_err(|e| e.to_string())?;
    let mono: Vec<f32> = samples
        .chunks(spec.channels as usize)
        .map(|v| v.iter().sum::<f32>() / v.len() as f32)
        .collect();
    if mono.is_empty() {
        return Err("WAV file is empty".into());
    }
    let len = (mono.len() as f64 * rate as f64 / spec.sample_rate as f64) as usize;
    let out = (0..len)
        .map(|i| {
            let p = i as f64 * spec.sample_rate as f64 / rate as f64;
            let a = p as usize;
            let f = (p - a as f64) as f32;
            mono[a.min(mono.len() - 1)] * (1. - f) + mono[(a + 1).min(mono.len() - 1)] * f
        })
        .collect();
    Ok(Arc::new(out))
}
pub fn tone(kind: usize, rate: u32) -> Arc<Vec<f32>> {
    use std::f32::consts::TAU;
    let duration = match kind {
        0 => 0.65,
        1 => 1.1,
        2 => 0.9,
        3 => 1.4,
        4 => 0.65,
        5 => 1.2,
        6 => 0.5,
        _ => 1.5,
    };
    Arc::new(
        (0..(duration * rate as f32) as usize)
            .map(|i| {
                let t = i as f32 / rate as f32;
                let env = (t * 80.).min(1.) * (1. - t / duration).max(0.).powi(2);
                let f = match kind {
                    0 => {
                        if t < 0.14 {
                            660.
                        } else {
                            880.
                        }
                    }
                    1 => 120. + 900. * t * t,
                    2 => {
                        let notes = [523.25, 659.25, 783.99, 1046.5];
                        notes[((t * 6.) as usize).min(3)]
                    }
                    3 => {
                        if (t * 8.) as usize % 2 == 0 {
                            440.
                        } else {
                            554.37
                        }
                    }
                    4 => 80. + 600. * (-t * 15.).exp(),
                    5 => 220. + 180. * (t * 7.).sin(),
                    6 => {
                        if t < 0.12 {
                            1200.
                        } else {
                            700.
                        }
                    }
                    _ => 880. - 600. * t / duration,
                };
                (TAU * f * t).sin() * env * 0.45
            })
            .collect(),
    )
}
pub fn smoke_test(dir: &Path) {
    let result = (|| -> Result<String, String> {
        let (input, output) = defaults();
        let shared = Shared::new(Params::default());
        let engine = Engine::start(&input, &output, None, 24, shared.clone())?;
        let rate = engine.rate;
        let path = dir.join("qa-audio.wav");
        engine.record(path.clone())?;
        engine
            .play
            .try_send(Some(tone(2, rate)))
            .map_err(|e| e.to_string())?;
        let mut peak = 0_f32;
        for _ in 0..20 {
            thread::sleep(std::time::Duration::from_millis(100));
            peak = peak.max(f32::from_bits(shared.output_level.load(Ordering::Relaxed)));
        }
        let queue = engine.buffered_ms();
        let xruns = shared.xruns.load(Ordering::Relaxed);
        let drops = shared.record_drops.load(Ordering::Relaxed);
        drop(engine);
        let wav = hound::WavReader::open(&path).map_err(|e| e.to_string())?;
        let duration = wav.duration();
        if duration < rate || peak < 0.01 {
            return Err(format!(
                "Audio pipeline produced too little signal: samples={duration},peak={peak}"
            ));
        }
        Ok(format!("PASS Windows WASAPI input → DSP + soundboard → queues + WAV\nInput: {input}\nOutput: {output} (physical monitor muted)\nRate: {rate} Hz\nSamples recorded: {duration}\nPeak: {peak:.4}\nQueue: {queue:.1} ms\nXruns: {xruns}\nRecording drops: {drops}\n"))
    })();
    let text = match result {
        Ok(s) => s,
        Err(e) => format!("FAIL {e}\n"),
    };
    let _ = std::fs::write(dir.join("audio-verification.txt"), text);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_finalizes_wav() {
        let path =
            std::env::temp_dir().join(format!("vesper-record-test-{}.wav", std::process::id()));
        let (tx, rx) = mpsc::sync_channel(8);
        let shared = Shared::new(Params::default());
        let worker = thread::spawn(move || record_worker(rx, shared));
        let (ack, ready) = mpsc::sync_channel(1);
        tx.send(RecMsg::Start(path.clone(), 48000, ack)).unwrap();
        ready.recv().unwrap().unwrap();
        tx.send(RecMsg::Frame([0.25; 1024], 1024)).unwrap();
        tx.send(RecMsg::Stop).unwrap();
        drop(tx);
        worker.join().unwrap();
        let mut reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.duration(), 1024);
        assert_eq!(reader.spec().sample_rate, 48000);
        assert_eq!(reader.samples::<i16>().next().unwrap().unwrap(), 8191);
        drop(reader);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn queue_transfers_between_threads() {
        let q = Arc::new(Queue::new());
        let producer = q.clone();
        let thread = thread::spawn(move || {
            for i in 0..100000 {
                while !producer.push(i as f32) {
                    std::hint::spin_loop();
                }
            }
        });
        for i in 0..100000 {
            loop {
                if let Some(v) = q.pop() {
                    assert_eq!(v, i as f32);
                    break;
                }
                std::hint::spin_loop();
            }
        }
        thread.join().unwrap();
    }
    #[test]
    fn queue_order_and_full() {
        let q = Queue::new();
        for i in 0..CAP {
            assert!(q.push(i as f32));
        }
        assert!(!q.push(0.));
        for i in 0..CAP {
            assert_eq!(q.pop(), Some(i as f32));
        }
        assert_eq!(q.pop(), None);
    }
    #[test]
    fn resample_wav() {
        let p = std::env::temp_dir().join(format!("vesper-test-{}.wav", std::process::id()));
        let mut w = hound::WavWriter::create(
            &p,
            hound::WavSpec {
                channels: 2,
                sample_rate: 44100,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..4410 {
            w.write_sample(1000_i16).unwrap();
            w.write_sample(1000_i16).unwrap();
        }
        w.finalize().unwrap();
        let v = load_wav(&p, 48000).unwrap();
        assert_eq!(v.len(), 4800);
        assert!((v[10] - 1000. / 32768.).abs() < 1e-5);
        std::fs::remove_file(p).unwrap();
    }
}
