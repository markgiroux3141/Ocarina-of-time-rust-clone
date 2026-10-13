//! The audio library through an output device (cpal), for the window app.
//!
//! A thread of its own plays the console: every 1/60 s of the clock it plays the AI for a VI
//! period and runs the retrace (`AudioMgr_HandleRetrace`, `AudioThread_UpdateImpl`), exactly as the
//! offline renderer does, and pushes what the AI plays (32006 Hz stereo) into a queue. The
//! device's callback drains the queue at the device's rate, resampling linearly, and nudges its
//! rate by up to half a percent to keep the queue near its target: the device's clock and the
//! system's drift apart. It starts once the queue holds its target, and if the queue ever runs
//! dry it waits for the target again; if it ever holds more than twice its target (the device
//! started late, or stalled), the oldest frames are dropped back to the target, so the sound is
//! never more than that behind the game. What the game's frames do arrive as `GameOp`s and are applied
//! before the next retrace; after each retrace the thread publishes what the game reads
//! (`AudioView`), with the queues' messages kept until the game takes them
//! (docs/adr/0025-audio-mixer.md, docs/adr/0026-the-games-audio.md).
//!
//! The game reads the view before its next frame, and only once a retrace has run after the
//! last ops it sent (`take_view` waits for it): on the console the audio thread has run the
//! game's commands by the next game frame (three retraces later), and the game reads
//! `gAudioCtx` as they left it. A view from before them loses sounds: the sound effects'
//! `Audio_PlayActiveSfx` takes a channel whose IO port 1 still reads `SEQ_IO_VAL_NONE` (the last
//! sound's end) as done, and forgets the new sound, so its stop never reaches it (an ocarina
//! note held on after its button is let go).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::context::AudioContext;
use crate::data::AudioData;
use crate::link::{AudioView, GameOp};
use crate::thread::Ai;

/// What the game sends the audio thread.
enum Msg {
    /// A game frame's `GameOp`s.
    Ops(Vec<GameOp>),
    Stop,
}

/// The frames queued for the device: the target fill (about 32 ms at 32 kHz, two retraces'
/// worth) and the most it holds before the oldest are dropped back to the target (about 64 ms:
/// the device started late or stalled). The rate nudge alone takes minutes to work off a
/// backlog, and every sound would be heard that much late: a sword swing's after the swing. The
/// queue is delay the console doesn't have (its AI plays the buffer the last retrace made), so
/// it's kept short: 64 ms put the ocarina's notes noticeably behind their buttons.
const TARGET_FILL: usize = 1024;
const MAX_FILL: usize = 2 * TARGET_FILL;

struct Shared {
    frames: VecDeque<[i16; 2]>,
    /// Playing: the queue reached its target since it last ran dry. Until then the device plays
    /// silence, so a dry queue costs one gap, not a crackle.
    primed: bool,
    /// Times the queue ran dry while playing.
    underruns: u64,
    /// Times the queue overfilled and was cut back to its target.
    dropped: u64,
}

/// What the audio thread publishes after each retrace.
struct Published {
    view: AudioView,
    /// The batches of ops (`Msg::Ops`) applied before that retrace.
    applied: u64,
}

/// The published view and its signal.
type SharedView = Arc<(Mutex<Published>, Condvar)>;

/// The longest the game waits for the audio thread's retrace (a stalled thread: carry on).
const VIEW_WAIT: Duration = Duration::from_millis(100);

/// The device's side: plays until dropped.
pub struct AudioOutput {
    tx: Sender<Msg>,
    /// What the game reads, as of the last retrace.
    view: SharedView,
    /// The batches of ops sent.
    sent: AtomicU64,
    thread: Option<std::thread::JoinHandle<()>>,
    /// The device's name and rate, for the HUD.
    pub description: String,
}

impl AudioOutput {
    /// Opens the default output device and starts the audio thread on `data`.
    pub fn start(data: AudioData) -> Result<AudioOutput> {
        let (tx, rx) = channel();
        let (ready_tx, ready_rx) = channel::<Result<String>>();
        let view: SharedView = Arc::new((Mutex::new(Published { view: AudioView::default(), applied: 0 }), Condvar::new()));
        let thread_view = view.clone();
        let thread = std::thread::Builder::new()
            .name("audio".into())
            .spawn(move || {
                if let Err(e) = audio_thread(data, rx, &ready_tx, thread_view) {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .context("starting the audio thread")?;
        let description = ready_rx.recv().context("the audio thread stopped")??;
        Ok(AudioOutput { tx, view, sent: AtomicU64::new(0), thread: Some(thread), description })
    }

    /// A game frame's `GameOp`s, applied before the next retrace.
    pub fn send_ops(&self, ops: Vec<GameOp>) {
        if !ops.is_empty() {
            self.sent.fetch_add(1, Ordering::SeqCst);
            let _ = self.tx.send(Msg::Ops(ops));
        }
    }

    /// Queues commands for the next retrace (`AudioThread_QueueCmd*`, then
    /// `AudioThread_ScheduleProcessCmds`).
    pub fn send(&self, cmds: Vec<(u32, u32)>) {
        if !cmds.is_empty() {
            let mut ops: Vec<GameOp> = cmds.into_iter().map(|(a, d)| GameOp::Cmd(a, d)).collect();
            ops.push(GameOp::Schedule);
            self.send_ops(ops);
        }
    }

    /// What the game reads now: the state as of the last retrace, and the messages posted since
    /// the last call. Waits (at most `VIEW_WAIT`) for a retrace after the last ops sent.
    pub fn take_view(&self) -> AudioView {
        let sent = self.sent.load(Ordering::SeqCst);
        let (lock, signal) = &*self.view;
        let (mut p, wait) = signal.wait_timeout_while(lock.lock().unwrap(), VIEW_WAIT, |p| p.applied < sent).unwrap();
        if wait.timed_out() {
            log::debug!("audio output: no retrace after the game's ops in {VIEW_WAIT:?}");
        }
        let out = p.view.clone();
        p.view.reset_msgs.clear();
        p.view.external_load_msgs.clear();
        out
    }

    /// Starts `seq_id` on `player` (`0x82`, no fade).
    pub fn start_sequence(&self, player: u8, seq_id: u8) {
        self.send(vec![(0x8200_0000 | ((player as u32) << 16) | ((seq_id as u32) << 8), 0)]);
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn audio_thread(data: AudioData, rx: Receiver<Msg>, ready: &Sender<Result<String>>, view: SharedView) -> Result<()> {
    let mut ctx = AudioContext::new(&data);
    drop(data);
    // What the game reads before the first retrace: AudioLoad_Init's spec 0.
    view.0.lock().unwrap().view.update(ctx.view());
    let mut applied: u64 = 0;
    let mut ai = Ai::new();
    let ai_rate = ctx.audio_buffer_parameters.ai_sampling_frequency as f64;
    let refresh = ctx.refresh_rate.max(1) as f64;

    let shared = Arc::new(Mutex::new(Shared { frames: VecDeque::with_capacity(MAX_FILL + 1024), primed: false, underruns: 0, dropped: 0 }));
    let host = cpal::default_host();
    let device = host.default_output_device().context("no audio output device")?;
    let supported = device.default_output_config().context("the output device's configuration")?;
    let config: cpal::StreamConfig = supported.clone().into();
    let channels = config.channels as usize;
    let device_rate = config.sample_rate as f64;
    let name = device.id().map(|i| i.to_string()).unwrap_or_else(|_| "default".into());
    let description = format!("{name}, {device_rate} Hz, {channels} ch");

    let fmt = supported.sample_format();
    let stream = match fmt {
        cpal::SampleFormat::F32 => build::<f32>(&device, config, shared.clone(), ai_rate / device_rate, channels)?,
        cpal::SampleFormat::I16 => build::<i16>(&device, config, shared.clone(), ai_rate / device_rate, channels)?,
        cpal::SampleFormat::U16 => build::<u16>(&device, config, shared.clone(), ai_rate / device_rate, channels)?,
        cpal::SampleFormat::I32 => build::<i32>(&device, config, shared.clone(), ai_rate / device_rate, channels)?,
        other => anyhow::bail!("the output device's sample format {other} isn't supported"),
    };
    stream.play().context("starting the audio stream")?;
    let _ = ready.send(Ok(description));

    let start = Instant::now();
    let period = Duration::from_secs_f64(1.0 / refresh);
    let mut retraces: u64 = 0;
    let mut out = Vec::new();
    loop {
        // Commands from the game, as its frames end.
        loop {
            match rx.try_recv() {
                Ok(Msg::Ops(ops)) => {
                    ctx.apply(&ops);
                    applied += 1;
                }
                Ok(Msg::Stop) | Err(std::sync::mpsc::TryRecvError::Disconnected) => return Ok(()),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
            }
        }
        // The VI period: the AI plays, then the retrace.
        out.clear();
        ai.play(ai_rate / refresh, &mut out);
        let remaining = ai.remaining_in_current();
        if let Some(buf) = ctx.audio_thread_update(remaining) {
            ai.set_next_buffer(buf);
        }
        let v = ctx.view();
        {
            let mut p = view.0.lock().unwrap();
            p.view.update(v);
            p.applied = applied;
        }
        view.1.notify_all();
        {
            let mut s = shared.lock().unwrap();
            for f in out.chunks(2) {
                s.frames.push_back([f[0], f[1]]);
            }
            if s.frames.len() > MAX_FILL {
                let excess = s.frames.len() - TARGET_FILL;
                s.frames.drain(..excess);
                s.dropped += 1;
            }
        }
        retraces += 1;
        if retraces % 300 == 0 {
            let s = shared.lock().unwrap();
            let playing: Vec<String> = ctx.seq_players.iter().map(|p| if p.enabled { format!("{:02X}", p.seq_id) } else { "--".into() }).collect();
            log::info!("audio output: {retraces} retraces, {} frames queued, ran dry {} times, cut back {} times; players {}", s.frames.len(), s.underruns, s.dropped, playing.join(" "));
        }
        let next = start + period.mul_f64(retraces as f64);
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        } else if now - next > Duration::from_millis(250) {
            // Fell far behind (a breakpoint, a stall): carry on from now.
            retraces = ((now - start).as_secs_f64() * refresh) as u64;
        }
    }
}

/// The output stream for samples of type `T`: drains the queue at the device's rate.
fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(device: &cpal::Device, config: cpal::StreamConfig, shared: Arc<Mutex<Shared>>, step: f64, channels: usize) -> Result<cpal::Stream> {
    let mut pos = 0.0f64; // between frames[0] and frames[1]
    let stream = device.build_output_stream(
        config,
        move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut s = shared.lock().unwrap();
            let fill = s.frames.len();
            // Keep the queue near its target: up to 0.5 % faster or slower.
            let adjust = ((fill as f64 - TARGET_FILL as f64) / TARGET_FILL as f64 * 0.005).clamp(-0.005, 0.005);
            let step = step * (1.0 + adjust);
            if !s.primed && s.frames.len() >= TARGET_FILL {
                s.primed = true;
            }
            for frame in out.chunks_mut(channels) {
                if !s.primed || s.frames.len() < 2 {
                    if s.primed {
                        s.underruns += 1;
                        s.primed = false;
                    }
                    frame.fill(T::from_sample(0.0f32));
                    continue;
                }
                let (a, b) = (s.frames[0], s.frames[1]);
                let f = pos as f32;
                let l = (a[0] as f32 * (1.0 - f) + b[0] as f32 * f) / 32768.0;
                let r = (a[1] as f32 * (1.0 - f) + b[1] as f32 * f) / 32768.0;
                if channels == 1 {
                    frame[0] = T::from_sample((l + r) * 0.5);
                } else {
                    frame[0] = T::from_sample(l);
                    frame[1] = T::from_sample(r);
                    for x in frame.iter_mut().skip(2) {
                        *x = T::from_sample(0.0f32);
                    }
                }
                pos += step;
                while pos >= 1.0 && s.frames.len() >= 2 {
                    s.frames.pop_front();
                    pos -= 1.0;
                }
            }
        },
        |e| log::warn!("audio output: {e}"),
        None,
    )?;
    Ok(stream)
}
