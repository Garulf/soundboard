use super::Reporter;
use super::cable::{find_cable, push_stereo, write_frames};
use super::retry::should_rebuild;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use soundboard_core::{
    AudioConfig, AudioControl, BackendStatus, Bus, BusPump, CHANNELS, Command, DeviceInfo,
    DeviceList, OutputAdapter, mic_ring,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(500);
const DEVICE_REFRESH: Duration = Duration::from_secs(5);
const SCRATCH_FRAMES: usize = 16_384;

enum Msg {
    Configure(AudioConfig),
    Terminate,
}

pub struct CpalAudio {
    tx: Sender<Msg>,
    thread: Option<JoinHandle<()>>,
}

impl AudioControl for CpalAudio {
    fn configure(&mut self, cfg: &AudioConfig) {
        let _ = self.tx.send(Msg::Configure(cfg.clone()));
    }
}

impl Drop for CpalAudio {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Terminate);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn start(pump: Arc<BusPump>, report: Reporter) -> Result<CpalAudio, String> {
    let (tx, rx) = channel();
    let thread = std::thread::Builder::new()
        .name("audio-devices".into())
        .spawn(move || Backend::new(pump, report).run(rx))
        .map_err(|e| e.to_string())?;
    Ok(CpalAudio {
        tx,
        thread: Some(thread),
    })
}

fn device_id(device: &Device) -> Option<String> {
    device.id().ok().map(|id| id.to_string())
}

fn device_info(device: &Device) -> Option<DeviceInfo> {
    Some(DeviceInfo {
        id: device_id(device)?,
        name: device.to_string(),
    })
}

fn f32_config(device: &Device, output: bool) -> Result<StreamConfig, String> {
    let default = if output {
        device.default_output_config()
    } else {
        device.default_input_config()
    }
    .map_err(|e| e.to_string())?;
    if default.sample_format() == SampleFormat::F32 {
        return Ok(default.config());
    }
    let rate = default.sample_rate();
    let ranges: Vec<_> = if output {
        device
            .supported_output_configs()
            .map_err(|e| e.to_string())?
            .collect()
    } else {
        device
            .supported_input_configs()
            .map_err(|e| e.to_string())?
            .collect()
    };
    ranges
        .into_iter()
        .find(|r| {
            r.sample_format() == SampleFormat::F32
                && r.min_sample_rate() <= rate
                && rate <= r.max_sample_rate()
        })
        .map(|r| r.with_sample_rate(rate).config())
        .ok_or_else(|| format!("{device} does not support 32-bit float audio"))
}

fn output_stream(
    device: &Device,
    pump: Arc<BusPump>,
    bus: Bus,
    failed: Arc<AtomicBool>,
) -> Result<Stream, String> {
    let config = f32_config(device, true)?;
    let channels = config.channels as usize;
    let mut output = OutputAdapter::new(bus, config.sample_rate);
    let mut stereo = Vec::with_capacity(SCRATCH_FRAMES * CHANNELS);
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| {
                let frames = data.len() / channels.max(1);
                stereo.resize(frames * CHANNELS, 0.0);
                output.pull(&pump, &mut stereo);
                write_frames(data, channels, &stereo);
            },
            move |e| {
                tracing::warn!("output stream error: {e}");
                failed.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

fn capture_stream(
    device: &Device,
    report: &Reporter,
    failed: Arc<AtomicBool>,
) -> Result<Stream, String> {
    let config = f32_config(device, false)?;
    let channels = config.channels as usize;
    let (mut producer, mic) = mic_ring(config.sample_rate);
    let stream = device
        .build_input_stream(
            config,
            move |data: &[f32], _| {
                push_stereo(data, channels, |s| {
                    let _ = producer.push(s);
                });
            },
            move |e| {
                tracing::warn!("capture stream error: {e}");
                failed.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    report(Command::MicReady(Box::new(mic)));
    Ok(stream)
}

#[derive(Default)]
struct Streams {
    monitor: Option<Stream>,
    cable: Option<Stream>,
    capture: Option<Stream>,
}

struct Backend {
    host: cpal::Host,
    pump: Arc<BusPump>,
    report: Reporter,
    config: Option<AudioConfig>,
    streams: Streams,
    failed: Arc<AtomicBool>,
    devices: DeviceList,
    last_refresh: Instant,
    last_attempt: Instant,
    incomplete: bool,
}

impl Backend {
    fn new(pump: Arc<BusPump>, report: Reporter) -> Self {
        Self {
            host: cpal::default_host(),
            pump,
            report,
            config: None,
            streams: Streams::default(),
            failed: Arc::new(AtomicBool::new(false)),
            devices: DeviceList::default(),
            last_refresh: Instant::now(),
            last_attempt: Instant::now(),
            incomplete: false,
        }
    }

    fn run(mut self, rx: Receiver<Msg>) {
        self.refresh_devices();
        loop {
            match rx.recv_timeout(POLL) {
                Ok(Msg::Configure(cfg)) => {
                    self.config = Some(cfg);
                    self.rebuild();
                }
                Ok(Msg::Terminate) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {}
            }
            let devices_changed =
                self.last_refresh.elapsed() >= DEVICE_REFRESH && self.refresh_devices();
            if should_rebuild(
                self.failed.load(Ordering::Relaxed),
                self.incomplete,
                self.last_attempt.elapsed(),
                devices_changed,
            ) {
                self.rebuild();
            }
        }
        self.close();
    }

    fn outputs(&self) -> Vec<Device> {
        self.host
            .output_devices()
            .map(|d| d.collect())
            .unwrap_or_default()
    }

    fn inputs(&self) -> Vec<Device> {
        self.host
            .input_devices()
            .map(|d| d.collect())
            .unwrap_or_default()
    }

    /// Re-enumerates devices and reports whether the list changed.
    fn refresh_devices(&mut self) -> bool {
        self.last_refresh = Instant::now();
        let devices = DeviceList {
            outputs: self.outputs().iter().filter_map(device_info).collect(),
            inputs: self.inputs().iter().filter_map(device_info).collect(),
        };
        if devices == self.devices {
            return false;
        }
        self.devices = devices.clone();
        (self.report)(Command::Devices(devices));
        true
    }

    fn close(&mut self) {
        self.streams.monitor = None;
        self.streams.cable = None;
        if self.streams.capture.take().is_some() {
            (self.report)(Command::MicClosed);
        }
    }

    fn pick(devices: Vec<Device>, id: Option<&str>, fallback: Option<Device>) -> Option<Device> {
        match id {
            Some(id) => devices
                .into_iter()
                .find(|d| device_id(d).as_deref() == Some(id))
                .or(fallback),
            None => fallback,
        }
    }

    fn rebuild(&mut self) {
        let Some(cfg) = self.config.clone() else {
            return;
        };
        self.last_attempt = Instant::now();
        self.failed.store(false, Ordering::Relaxed);
        self.close();
        self.refresh_devices();
        let mut problems = Vec::new();

        let monitor = Self::pick(
            self.outputs(),
            cfg.monitor_device.as_deref(),
            self.host.default_output_device(),
        );
        match monitor
            .map(|d| output_stream(&d, self.pump.clone(), Bus::Monitor, self.failed.clone()))
        {
            Some(Ok(stream)) => self.streams.monitor = Some(stream),
            Some(Err(e)) => problems.push(format!("monitor output: {e}")),
            None => problems.push("no output device found".into()),
        }

        let outputs = self.outputs();
        let cable = match cfg.cable_device.as_deref() {
            Some(id) => outputs
                .into_iter()
                .find(|d| device_id(d).as_deref() == Some(id)),
            None => {
                let names: Vec<String> = outputs.iter().map(|d| d.to_string()).collect();
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                find_cable(&refs).and_then(|i| outputs.into_iter().nth(i))
            }
        };
        let cable_found = cable.is_some();
        if let Some(device) = cable {
            match output_stream(&device, self.pump.clone(), Bus::Mic, self.failed.clone()) {
                Ok(stream) => self.streams.cable = Some(stream),
                Err(e) => problems.push(format!("virtual cable: {e}")),
            }
        }

        if cfg.passthrough {
            let mic = Self::pick(
                self.inputs(),
                cfg.mic_device.as_deref(),
                self.host.default_input_device(),
            );
            match mic.map(|d| capture_stream(&d, &self.report, self.failed.clone())) {
                Some(Ok(stream)) => self.streams.capture = Some(stream),
                Some(Err(e)) => problems.push(format!("microphone: {e}")),
                None => problems.push("no microphone found".into()),
            }
        }

        self.incomplete = !problems.is_empty() || !cable_found;
        let status = if !problems.is_empty() {
            BackendStatus::Error(problems.join("; "))
        } else if !cable_found {
            BackendStatus::CableMissing
        } else {
            BackendStatus::Ok
        };
        (self.report)(Command::BackendStatus(status));
    }
}
