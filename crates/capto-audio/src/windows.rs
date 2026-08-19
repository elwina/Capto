use crate::{AudioDeviceInfo, AudioDeviceKind, AudioError, AudioLevels, PcmInputSpec, Result};
use std::collections::{HashSet, VecDeque};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use wasapi::{initialize_mta, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

const SAMPLE_RATE: u32 = 48_000;
const CHANNELS: u16 = 2;
const BYTES_PER_FRAME: usize = CHANNELS as usize * std::mem::size_of::<f32>();
const CHUNK_FRAMES: usize = 480; // 10 ms at 48 kHz
const CHUNK_BYTES: usize = CHUNK_FRAMES * BYTES_PER_FRAME;

#[derive(Debug, Clone, Copy)]
enum EndpointDirection {
    Capture,
    RenderLoopback,
}

impl EndpointDirection {
    fn kind(self) -> AudioDeviceKind {
        match self {
            Self::Capture => AudioDeviceKind::Input,
            Self::RenderLoopback => AudioDeviceKind::Loopback,
        }
    }
}

struct PreparedStream {
    listener: Option<TcpListener>,
    device_id: String,
    direction: EndpointDirection,
}

/// Owns native WASAPI capture threads and their localhost PCM transports.
///
/// FFmpeg connects as a TCP client and consumes `f32le`, 48 kHz stereo. TCP
/// keeps stdin free for FFmpeg's `q` command and gives each source independent
/// backpressure without filesystem or named-pipe cleanup.
pub struct NativeAudioSession {
    prepared: Vec<PreparedStream>,
    inputs: Vec<PcmInputSpec>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    handles: Vec<JoinHandle<()>>,
    levels: Arc<LevelMeters>,
}

/// Short-lived WASAPI monitor used by the UI before recording starts.
pub struct AudioMeterSession {
    stop: Arc<AtomicBool>,
    handles: Vec<JoinHandle<()>>,
    levels: Arc<LevelMeters>,
}

#[derive(Default)]
struct LevelMeters {
    microphone: AtomicU32,
    system: AtomicU32,
}

impl LevelMeters {
    fn update(&self, direction: EndpointDirection, peak: f32) {
        let slot = match direction {
            EndpointDirection::Capture => &self.microphone,
            EndpointDirection::RenderLoopback => &self.system,
        };
        let next = peak.clamp(0.0, 1.0);
        let mut current = slot.load(Ordering::Relaxed);
        while next > f32::from_bits(current) {
            match slot.compare_exchange_weak(
                current,
                next.to_bits(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    fn take(&self) -> AudioLevels {
        AudioLevels {
            microphone: f32::from_bits(self.microphone.swap(0, Ordering::Relaxed)),
            system: f32::from_bits(self.system.swap(0, Ordering::Relaxed)),
        }
    }
}

impl NativeAudioSession {
    pub fn prepare(mic: Option<&str>, loopback: Option<&str>) -> Result<Option<Self>> {
        let mut requested = Vec::new();
        if let Some(id) = mic.filter(|id| !id.is_empty()) {
            requested.push((id, EndpointDirection::Capture));
        }
        if let Some(id) = loopback.filter(|id| !id.is_empty()) {
            requested.push((id, EndpointDirection::RenderLoopback));
        }
        if requested.is_empty() {
            return Ok(None);
        }

        let mut prepared = Vec::with_capacity(requested.len());
        let mut inputs = Vec::with_capacity(requested.len());
        for (stored_id, expected) in requested {
            let (direction, device_id) = parse_device_id(stored_id)?;
            if direction.kind() != expected.kind() {
                return Err(AudioError::Device(format!(
                    "audio endpoint has the wrong direction: {stored_id}"
                )));
            }

            let listener = TcpListener::bind(("127.0.0.1", 0))
                .map_err(|e| AudioError::Transport(e.to_string()))?;
            listener
                .set_nonblocking(true)
                .map_err(|e| AudioError::Transport(e.to_string()))?;
            let port = listener
                .local_addr()
                .map_err(|e| AudioError::Transport(e.to_string()))?
                .port();
            let spec = PcmInputSpec {
                kind: direction.kind(),
                url: format!("tcp://127.0.0.1:{port}"),
                sample_rate: SAMPLE_RATE,
                channels: CHANNELS,
            };
            inputs.push(spec.clone());
            prepared.push(PreparedStream {
                listener: Some(listener),
                device_id,
                direction,
            });
        }

        Ok(Some(Self {
            prepared,
            inputs,
            stop: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            handles: Vec::new(),
            levels: Arc::new(LevelMeters::default()),
        }))
    }

    pub fn inputs(&self) -> &[PcmInputSpec] {
        &self.inputs
    }

    /// Soft-pause: stop writing PCM so encode timeline skips paused time.
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Release);
    }

    /// Start every capture thread and wait until WASAPI is initialized.
    pub fn start(&mut self) -> Result<()> {
        self.stop.store(false, Ordering::Release);
        let mut ready_receivers = Vec::new();

        for prepared in &mut self.prepared {
            let listener = prepared
                .listener
                .take()
                .ok_or_else(|| AudioError::Transport("audio session already started".into()))?;
            let device_id = prepared.device_id.clone();
            let direction = prepared.direction;
            let stop = Arc::clone(&self.stop);
            let paused = Arc::clone(&self.paused);
            let levels = Arc::clone(&self.levels);
            let (ready_tx, ready_rx) = mpsc::sync_channel(1);
            ready_receivers.push(ready_rx);
            let name = match direction {
                EndpointDirection::Capture => "capto-wasapi-mic",
                EndpointDirection::RenderLoopback => "capto-wasapi-loopback",
            };
            let handle = thread::Builder::new()
                .name(name.into())
                .spawn(move || {
                    let error_tx = ready_tx.clone();
                    if let Err(error) = capture_thread(
                        listener, &device_id, direction, &stop, &paused, &levels, ready_tx,
                    ) {
                        let _ = error_tx.try_send(Err(error.clone()));
                        tracing::error!(%error, ?direction, "WASAPI capture stopped");
                    }
                })
                .map_err(|e| AudioError::Transport(e.to_string()))?;
            self.handles.push(handle);
        }

        for ready in ready_receivers {
            match ready.recv_timeout(Duration::from_secs(5)) {
                Ok(Ok(())) => {}
                Ok(Err(message)) => {
                    self.stop();
                    return Err(AudioError::Device(message));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    self.stop();
                    return Err(AudioError::Transport(
                        "timed out initializing WASAPI".into(),
                    ));
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    self.stop();
                    return Err(AudioError::Transport(
                        "WASAPI capture thread exited during initialization".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }

    pub fn levels(&self) -> AudioLevels {
        self.levels.take()
    }
}

impl AudioMeterSession {
    pub fn start(mic: Option<&str>, loopback: Option<&str>) -> Result<Option<Self>> {
        let mut requested = Vec::new();
        if let Some(id) = mic.filter(|id| !id.is_empty()) {
            requested.push((id.to_owned(), EndpointDirection::Capture));
        }
        if let Some(id) = loopback.filter(|id| !id.is_empty()) {
            requested.push((id.to_owned(), EndpointDirection::RenderLoopback));
        }
        if requested.is_empty() {
            return Ok(None);
        }

        let stop = Arc::new(AtomicBool::new(false));
        let levels = Arc::new(LevelMeters::default());
        let mut handles = Vec::with_capacity(requested.len());
        let mut ready_receivers = Vec::with_capacity(requested.len());
        for (stored_id, expected) in requested {
            let (direction, device_id) = parse_device_id(&stored_id)?;
            if direction.kind() != expected.kind() {
                return Err(AudioError::Device(format!(
                    "audio endpoint has the wrong direction: {stored_id}"
                )));
            }
            let stop2 = Arc::clone(&stop);
            let levels2 = Arc::clone(&levels);
            let (ready_tx, ready_rx) = mpsc::sync_channel(1);
            ready_receivers.push(ready_rx);
            let handle = thread::Builder::new()
                .name("capto-audio-meter".into())
                .spawn(move || {
                    let _ = meter_thread(&device_id, direction, &stop2, &levels2, ready_tx);
                })
                .map_err(|e| AudioError::Transport(e.to_string()))?;
            handles.push(handle);
        }
        for ready in ready_receivers {
            match ready.recv_timeout(Duration::from_secs(5)) {
                Ok(Ok(())) => {}
                Ok(Err(message)) => return Err(AudioError::Device(message)),
                Err(_) => {
                    return Err(AudioError::Transport(
                        "timed out starting audio meter".into(),
                    ))
                }
            }
        }
        Ok(Some(Self {
            stop,
            handles,
            levels,
        }))
    }

    pub fn levels(&self) -> AudioLevels {
        self.levels.take()
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Release);
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}

impl Drop for AudioMeterSession {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}

impl Drop for NativeAudioSession {
    fn drop(&mut self) {
        self.stop();
    }
}

fn meter_thread(
    device_id: &str,
    direction: EndpointDirection,
    stop: &AtomicBool,
    levels: &LevelMeters,
    ready: mpsc::SyncSender<std::result::Result<(), String>>,
) -> std::result::Result<(), String> {
    initialize_mta()
        .ok()
        .map_err(|e| format!("COM initialization failed: {e}"))?;
    let enumerator =
        DeviceEnumerator::new().map_err(|e| format!("create device enumerator: {e}"))?;
    let collection_direction = match direction {
        EndpointDirection::Capture => Direction::Capture,
        EndpointDirection::RenderLoopback => Direction::Render,
    };
    let collection = enumerator
        .get_device_collection(&collection_direction)
        .map_err(|e| format!("enumerate audio endpoints: {e}"))?;
    let mut matched = None;
    for item in &collection {
        let candidate = item.map_err(|e| format!("read audio endpoint: {e}"))?;
        if candidate.get_id().ok().as_deref() == Some(device_id) {
            matched = Some(candidate);
            break;
        }
    }
    let device = matched.ok_or_else(|| format!("audio endpoint disappeared: {device_id}"))?;
    let mut audio_client = device
        .get_iaudioclient()
        .map_err(|e| format!("activate audio client: {e}"))?;
    let desired_format = WaveFormat::new(
        32,
        32,
        &SampleType::Float,
        SAMPLE_RATE as usize,
        CHANNELS as usize,
        None,
    );
    let (_, minimum_period) = audio_client
        .get_device_period()
        .map_err(|e| format!("query device period: {e}"))?;
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: minimum_period,
    };
    audio_client
        .initialize_client(&desired_format, &Direction::Capture, &mode)
        .map_err(|e| format!("initialize 48 kHz stereo capture: {e}"))?;
    let event = audio_client
        .set_get_eventhandle()
        .map_err(|e| format!("create capture event: {e}"))?;
    let capture = audio_client
        .get_audiocaptureclient()
        .map_err(|e| format!("get capture client: {e}"))?;
    audio_client
        .start_stream()
        .map_err(|e| format!("start capture stream: {e}"))?;
    let _ = ready.send(Ok(()));
    let mut packet = Vec::new();
    while !stop.load(Ordering::Acquire) {
        let _ = event.wait_for_event(20);
        loop {
            let frames = capture
                .get_next_packet_size()
                .map_err(|e| e.to_string())?
                .unwrap_or(0) as usize;
            if frames == 0 {
                break;
            }
            packet.resize(frames * BYTES_PER_FRAME, 0);
            let (read, info) = capture
                .read_from_device(&mut packet)
                .map_err(|e| e.to_string())?;
            if !info.flags.silent {
                let mut peak = 0.0f32;
                for sample in packet[..read as usize * BYTES_PER_FRAME].chunks_exact(4) {
                    peak = peak.max(
                        f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]).abs(),
                    );
                }
                levels.update(direction, peak);
            }
        }
    }
    let _ = audio_client.stop_stream();
    Ok(())
}

fn capture_thread(
    listener: TcpListener,
    device_id: &str,
    direction: EndpointDirection,
    stop: &AtomicBool,
    paused: &AtomicBool,
    levels: &LevelMeters,
    ready: mpsc::SyncSender<std::result::Result<(), String>>,
) -> std::result::Result<(), String> {
    initialize_mta()
        .ok()
        .map_err(|e| format!("COM initialization failed: {e}"))?;
    let enumerator =
        DeviceEnumerator::new().map_err(|e| format!("create device enumerator: {e}"))?;
    // `wasapi::DeviceEnumerator::get_device` in 0.23 constructs a PCWSTR from
    // a temporary HSTRING and can return E_NOTFOUND. Enumerating and matching
    // the stable endpoint ID avoids that lifetime bug.
    let collection_direction = match direction {
        EndpointDirection::Capture => Direction::Capture,
        EndpointDirection::RenderLoopback => Direction::Render,
    };
    let collection = enumerator
        .get_device_collection(&collection_direction)
        .map_err(|e| format!("enumerate audio endpoints: {e}"))?;
    let mut matched = None;
    for item in &collection {
        let candidate = item.map_err(|e| format!("read audio endpoint: {e}"))?;
        if candidate.get_id().ok().as_deref() == Some(device_id) {
            matched = Some(candidate);
            break;
        }
    }
    let device = matched.ok_or_else(|| format!("audio endpoint disappeared: {device_id}"))?;
    let mut audio_client = device
        .get_iaudioclient()
        .map_err(|e| format!("activate audio client: {e}"))?;
    let desired_format = WaveFormat::new(
        32,
        32,
        &SampleType::Float,
        SAMPLE_RATE as usize,
        CHANNELS as usize,
        None,
    );
    let (_, minimum_period) = audio_client
        .get_device_period()
        .map_err(|e| format!("query device period: {e}"))?;
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: minimum_period,
    };
    // A render endpoint initialized for capture makes the wasapi crate set
    // AUDCLNT_STREAMFLAGS_LOOPBACK.
    audio_client
        .initialize_client(&desired_format, &Direction::Capture, &mode)
        .map_err(|e| format!("initialize 48 kHz stereo capture: {e}"))?;
    let event = audio_client
        .set_get_eventhandle()
        .map_err(|e| format!("create capture event: {e}"))?;
    let capture = audio_client
        .get_audiocaptureclient()
        .map_err(|e| format!("get capture client: {e}"))?;
    audio_client
        .start_stream()
        .map_err(|e| format!("start capture stream: {e}"))?;
    let _ = ready.send(Ok(()));

    let mut stream = accept_ffmpeg(&listener, stop)?;
    // Windows inherits non-blocking from the listener; FFmpeg briefly not
    // reading (video init) then makes write_all fail with WSAEWOULDBLOCK
    // (10035) and we used to abort — killing mic/loopback for the whole take.
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    stream.set_nodelay(true).map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_millis(2_000)))
        .map_err(|e| e.to_string())?;
    let mut queue = VecDeque::<u8>::with_capacity(CHUNK_BYTES * 20);
    let mut packet = Vec::<u8>::new();
    let mut next_tick = Instant::now();
    let mut peak = 0.0f32;
    let mut frames_sent: u64 = 0;

    while !stop.load(Ordering::Acquire) {
        let _ = event.wait_for_event(5);
        loop {
            let frames = capture
                .get_next_packet_size()
                .map_err(|e| e.to_string())?
                .unwrap_or(0) as usize;
            if frames == 0 {
                break;
            }
            packet.resize(frames * BYTES_PER_FRAME, 0);
            let (read, info) = capture
                .read_from_device(&mut packet)
                .map_err(|e| e.to_string())?;
            let byte_count = read as usize * BYTES_PER_FRAME;
            if info.flags.silent {
                queue.extend(std::iter::repeat_n(0, byte_count));
            } else {
                let mut packet_peak = 0.0f32;
                for sample in packet[..byte_count].chunks_exact(4) {
                    let value = f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]);
                    peak = peak.max(value.abs());
                    packet_peak = packet_peak.max(value.abs());
                }
                levels.update(direction, packet_peak);
                queue.extend(&packet[..byte_count]);
            }
        }

        let now = Instant::now();
        if paused.load(Ordering::Acquire) {
            // Drop captured audio while paused; do not write so FFmpeg's
            // sample clock (and video CFR) skip this wall time together.
            queue.clear();
            next_tick = now + Duration::from_millis(10);
            continue;
        }
        while now >= next_tick && !stop.load(Ordering::Acquire) {
            // Keep at most 200 ms queued. If capture falls behind, old samples
            // are less useful than staying synchronized with the video clock.
            while queue.len() > CHUNK_BYTES * 20 {
                let _ = queue.pop_front();
            }
            let mut chunk = [0u8; CHUNK_BYTES];
            for byte in &mut chunk {
                *byte = queue.pop_front().unwrap_or(0);
            }
            if let Err(error) = write_all_pcm(&mut stream, &chunk, stop) {
                if !stop.load(Ordering::Acquire) {
                    return Err(error);
                }
                break;
            }
            frames_sent += CHUNK_FRAMES as u64;
            if frames_sent == CHUNK_FRAMES as u64 * 100 {
                tracing::debug!(?direction, peak, "WASAPI PCM flowing to FFmpeg");
                peak = 0.0;
            } else if frames_sent % (CHUNK_FRAMES as u64 * 500) == 0 {
                tracing::debug!(?direction, peak, frames_sent, "WASAPI PCM still flowing");
                peak = 0.0;
            }
            next_tick += Duration::from_millis(10);
            if now.saturating_duration_since(next_tick) > Duration::from_millis(100) {
                next_tick = now + Duration::from_millis(10);
            }
        }
    }

    let _ = audio_client.stop_stream();
    Ok(())
}

fn accept_ffmpeg(
    listener: &TcpListener,
    stop: &AtomicBool,
) -> std::result::Result<TcpStream, String> {
    while !stop.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) => return Ok(stream),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("audio capture cancelled before FFmpeg connected".into())
}

fn write_all_pcm(
    stream: &mut TcpStream,
    chunk: &[u8],
    stop: &AtomicBool,
) -> std::result::Result<(), String> {
    let mut offset = 0;
    while offset < chunk.len() {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        match stream.write(&chunk[offset..]) {
            Ok(0) => return Err("FFmpeg audio transport closed".into()),
            Ok(n) => offset += n,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                ) =>
            {
                // Blocking sockets should rarely hit this; keep waiting so a
                // brief FFmpeg stall does not kill the mic mid-take.
                thread::sleep(Duration::from_millis(1));
            }
            Err(error) => {
                return Err(format!("FFmpeg audio transport closed: {error}"));
            }
        }
    }
    Ok(())
}

fn parse_device_id(stored: &str) -> Result<(EndpointDirection, String)> {
    let mut parts = stored.splitn(3, ':');
    if parts.next() != Some("wasapi") {
        return Err(AudioError::Device(format!(
            "legacy audio device is no longer valid: {stored}"
        )));
    }
    let direction = match parts.next() {
        Some("capture") => EndpointDirection::Capture,
        Some("render") => EndpointDirection::RenderLoopback,
        _ => {
            return Err(AudioError::Device(format!(
                "invalid WASAPI device id: {stored}"
            )))
        }
    };
    let id = parts
        .next()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| AudioError::Device(format!("invalid WASAPI device id: {stored}")))?;
    Ok((direction, id.to_string()))
}

pub fn list_devices() -> Result<Vec<AudioDeviceInfo>> {
    // Keep COM initialization and enumeration on one short-lived OS thread.
    thread::spawn(list_devices_inner)
        .join()
        .map_err(|_| AudioError::Device("WASAPI device enumeration panicked".into()))?
}

fn list_devices_inner() -> Result<Vec<AudioDeviceInfo>> {
    initialize_mta()
        .ok()
        .map_err(|e| AudioError::Device(e.to_string()))?;
    let enumerator = DeviceEnumerator::new().map_err(|e| AudioError::Device(e.to_string()))?;
    let default_capture = enumerator
        .get_default_device(&Direction::Capture)
        .ok()
        .and_then(|device| device.get_id().ok());
    let default_render = enumerator
        .get_default_device(&Direction::Render)
        .ok()
        .and_then(|device| device.get_id().ok());
    let mut devices = Vec::new();
    let mut seen = HashSet::new();

    for (direction, prefix, kind, default_id) in [
        (
            Direction::Capture,
            "capture",
            AudioDeviceKind::Input,
            default_capture.as_deref(),
        ),
        (
            Direction::Render,
            "render",
            AudioDeviceKind::Loopback,
            default_render.as_deref(),
        ),
    ] {
        let collection = enumerator
            .get_device_collection(&direction)
            .map_err(|e| AudioError::Device(e.to_string()))?;
        for item in &collection {
            let device = item.map_err(|e| AudioError::Device(e.to_string()))?;
            let id = device
                .get_id()
                .map_err(|e| AudioError::Device(e.to_string()))?;
            if !seen.insert((kind, id.clone())) {
                continue;
            }
            let name = device
                .get_friendlyname()
                .unwrap_or_else(|_| "Unknown audio device".into());
            devices.push(AudioDeviceInfo {
                id: format!("wasapi:{prefix}:{id}"),
                name,
                kind,
                is_default: default_id == Some(id.as_str()),
            });
        }
    }

    if devices.is_empty() {
        Err(AudioError::NoHost)
    } else {
        Ok(devices)
    }
}
