use super::Reporter;
use super::devices::{
    DeviceKind, VIRTUAL_MIC_DESCRIPTION, VIRTUAL_MIC_NODE, device_kind, device_label,
};
use pipewire as pw;
use pw::properties::{PropertiesBox, properties};
use pw::spa;
use pw::stream::{StreamFlags, StreamListener, StreamRc, StreamState};
use pw::types::ObjectType;
use soundboard_core::{
    AudioConfig, AudioControl, BackendStatus, Bus, BusPump, CHANNELS, Command, DeviceInfo,
    DeviceList, ENGINE_RATE, mic_ring,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::JoinHandle;

const BYTES_PER_SAMPLE: usize = std::mem::size_of::<f32>();
const STRIDE: usize = CHANNELS * BYTES_PER_SAMPLE;
const NODE_LATENCY: &str = "256/48000";
const SCRATCH_FRAMES: usize = 8192;

enum Msg {
    Configure(AudioConfig),
    Terminate,
}

pub struct PipeWireAudio {
    tx: pw::channel::Sender<Msg>,
    thread: Option<JoinHandle<()>>,
}

impl AudioControl for PipeWireAudio {
    fn configure(&mut self, cfg: &AudioConfig) {
        let _ = self.tx.send(Msg::Configure(cfg.clone()));
    }
}

impl Drop for PipeWireAudio {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Terminate);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn start(pump: Arc<BusPump>, report: Reporter) -> Result<PipeWireAudio, String> {
    let (tx, rx) = pw::channel::channel();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let thread = std::thread::Builder::new()
        .name("pipewire".into())
        .spawn(move || {
            if let Err(e) = run(pump, report.clone(), rx, &ready_tx) {
                tracing::error!("PipeWire backend stopped: {e}");
                report(Command::BackendStatus(BackendStatus::Error(e.clone())));
                let _ = ready_tx.send(Err(e));
            }
        })
        .map_err(|e| e.to_string())?;
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(PipeWireAudio {
            tx,
            thread: Some(thread),
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("the PipeWire thread exited unexpectedly".into()),
    }
}

fn format_param() -> Vec<u8> {
    let mut info = spa::param::audio::AudioInfoRaw::new();
    info.set_format(spa::param::audio::AudioFormat::F32LE);
    info.set_rate(ENGINE_RATE);
    info.set_channels(CHANNELS as u32);
    let mut position = [0; spa::param::audio::MAX_CHANNELS];
    position[0] = spa::sys::SPA_AUDIO_CHANNEL_FL;
    position[1] = spa::sys::SPA_AUDIO_CHANNEL_FR;
    info.set_position(position);
    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(spa::pod::Object {
            type_: spa::sys::SPA_TYPE_OBJECT_Format,
            id: spa::sys::SPA_PARAM_EnumFormat,
            properties: info.into(),
        }),
    )
    .expect("serializing a fixed audio format cannot fail")
    .0
    .into_inner()
}

fn connect(
    stream: &StreamRc,
    direction: spa::utils::Direction,
    flags: StreamFlags,
) -> Result<(), String> {
    let bytes = format_param();
    let pod = spa::pod::Pod::from_bytes(&bytes).ok_or("invalid audio format pod")?;
    stream
        .connect(direction, None, flags, &mut [pod])
        .map_err(|e| e.to_string())
}

/// A stream that plays one mixer bus. The listener is declared first so it
/// is unregistered before the stream is destroyed.
struct OutputStream {
    _listener: StreamListener<Vec<f32>>,
    _stream: StreamRc,
}

fn output_stream(
    core: &pw::core::CoreRc,
    name: &str,
    props: PropertiesBox,
    pump: Arc<BusPump>,
    bus: Bus,
    flags: StreamFlags,
    report: Option<Reporter>,
) -> Result<OutputStream, String> {
    let stream = StreamRc::new(core.clone(), name, props).map_err(|e| e.to_string())?;
    let listener = stream
        .add_local_listener_with_user_data(Vec::<f32>::with_capacity(SCRATCH_FRAMES * CHANNELS))
        .state_changed(move |_, _, _, new| {
            if let (Some(report), StreamState::Error(e)) = (&report, new) {
                report(Command::BackendStatus(BackendStatus::Error(e)));
            }
        })
        .process(move |stream, scratch| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let requested = buffer.requested() as usize;
            let datas = buffer.datas_mut();
            let Some(data) = datas.first_mut() else {
                return;
            };
            let frames = match data.data() {
                Some(bytes) => {
                    let capacity = bytes.len() / STRIDE;
                    let frames = if requested == 0 {
                        capacity
                    } else {
                        requested.min(capacity)
                    };
                    scratch.resize(frames * CHANNELS, 0.0);
                    pump.pull(bus, scratch);
                    for (dst, sample) in bytes
                        .as_chunks_mut::<BYTES_PER_SAMPLE>()
                        .0
                        .iter_mut()
                        .zip(scratch.iter())
                    {
                        dst.copy_from_slice(&sample.to_le_bytes());
                    }
                    frames
                }
                None => 0,
            };
            let chunk = data.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.stride_mut() = STRIDE as i32;
            *chunk.size_mut() = (frames * STRIDE) as u32;
        })
        .register()
        .map_err(|e| e.to_string())?;
    connect(&stream, spa::utils::Direction::Output, flags)?;
    Ok(OutputStream {
        _listener: listener,
        _stream: stream,
    })
}

struct CaptureStream {
    _listener: StreamListener<rtrb::Producer<f32>>,
    _stream: StreamRc,
}

fn capture_stream(
    core: &pw::core::CoreRc,
    target: Option<&str>,
    report: &Reporter,
) -> Result<CaptureStream, String> {
    let mut props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Communication",
        *pw::keys::NODE_NAME => "soundboard_passthrough",
        *pw::keys::NODE_DESCRIPTION => "Soundboard mic passthrough",
        *pw::keys::NODE_LATENCY => NODE_LATENCY,
    };
    if let Some(target) = target {
        props.insert(*pw::keys::TARGET_OBJECT, target);
    }
    let stream =
        StreamRc::new(core.clone(), "soundboard-passthrough", props).map_err(|e| e.to_string())?;
    let (producer, mic) = mic_ring(ENGINE_RATE);
    let listener = stream
        .add_local_listener_with_user_data(producer)
        .process(|stream, producer| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            let Some(data) = datas.first_mut() else {
                return;
            };
            let offset = data.chunk().offset() as usize;
            let size = data.chunk().size() as usize;
            if let Some(bytes) = data.data() {
                let end = (offset + size).min(bytes.len());
                let start = offset.min(end);
                for frame in bytes[start..end].as_chunks::<STRIDE>().0 {
                    if producer.slots() < CHANNELS {
                        break;
                    }
                    for sample in frame.as_chunks::<BYTES_PER_SAMPLE>().0 {
                        let _ = producer.push(f32::from_le_bytes(*sample));
                    }
                }
            }
        })
        .register()
        .map_err(|e| e.to_string())?;
    connect(
        &stream,
        spa::utils::Direction::Input,
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
    )?;
    report(Command::MicReady(Box::new(mic)));
    Ok(CaptureStream {
        _listener: listener,
        _stream: stream,
    })
}

#[derive(Default)]
struct Devices {
    nodes: BTreeMap<u32, (DeviceKind, DeviceInfo)>,
}

impl Devices {
    fn list(&self) -> DeviceList {
        let mut list = DeviceList::default();
        for (kind, info) in self.nodes.values() {
            match kind {
                DeviceKind::Output => list.outputs.push(info.clone()),
                DeviceKind::Input => list.inputs.push(info.clone()),
            }
        }
        list
    }
}

struct Graph {
    core: pw::core::CoreRc,
    pump: Arc<BusPump>,
    report: Reporter,
    config: Option<AudioConfig>,
    monitor: Option<OutputStream>,
    capture: Option<CaptureStream>,
}

impl Graph {
    fn monitor_props(target: Option<&str>) -> PropertiesBox {
        let mut props = properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Playback",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::NODE_NAME => "soundboard_monitor",
            *pw::keys::NODE_DESCRIPTION => "Soundboard",
            *pw::keys::NODE_LATENCY => NODE_LATENCY,
        };
        if let Some(target) = target {
            props.insert(*pw::keys::TARGET_OBJECT, target);
        }
        props
    }

    fn apply(&mut self, cfg: AudioConfig) {
        let old = self.config.take();
        let monitor_changed = old
            .as_ref()
            .is_none_or(|o| o.monitor_device != cfg.monitor_device);
        let capture_changed = old
            .as_ref()
            .is_none_or(|o| o.passthrough != cfg.passthrough || o.mic_device != cfg.mic_device);

        if monitor_changed {
            self.monitor = None;
            match output_stream(
                &self.core,
                "soundboard-monitor",
                Self::monitor_props(cfg.monitor_device.as_deref()),
                self.pump.clone(),
                Bus::Monitor,
                StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
                None,
            ) {
                Ok(stream) => self.monitor = Some(stream),
                Err(e) => (self.report)(Command::Notice(format!(
                    "Could not open the monitor output: {e}"
                ))),
            }
        }
        if capture_changed {
            if self.capture.take().is_some() {
                (self.report)(Command::MicClosed);
            }
            if cfg.passthrough {
                match capture_stream(&self.core, cfg.mic_device.as_deref(), &self.report) {
                    Ok(stream) => self.capture = Some(stream),
                    Err(e) => (self.report)(Command::Notice(format!(
                        "Could not open the microphone: {e}"
                    ))),
                }
            }
        }
        self.config = Some(cfg);
    }
}

fn run(
    pump: Arc<BusPump>,
    report: Reporter,
    rx: pw::channel::Receiver<Msg>,
    ready: &std::sync::mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    pw::init();
    let connect_err = |e: pw::Error| format!("Could not connect to PipeWire: {e}");
    let mainloop = pw::main_loop::MainLoopRc::new(None).map_err(connect_err)?;
    let context = pw::context::ContextRc::new(&mainloop, None).map_err(connect_err)?;
    let core = context.connect_rc(None).map_err(connect_err)?;
    let registry = core.get_registry_rc().map_err(connect_err)?;

    let _core_listener = core
        .add_listener_local()
        .error({
            let report = report.clone();
            let mainloop = mainloop.clone();
            move |id, _seq, _res, message| {
                if id == pw::core::PW_ID_CORE {
                    report(Command::BackendStatus(BackendStatus::Error(format!(
                        "Lost the PipeWire connection: {message}"
                    ))));
                    mainloop.quit();
                }
            }
        })
        .register();

    let devices = Rc::new(RefCell::new(Devices::default()));
    let _registry_listener = registry
        .add_listener_local()
        .global({
            let devices = devices.clone();
            let report = report.clone();
            move |obj| {
                if obj.type_ != ObjectType::Node {
                    return;
                }
                let Some(props) = obj.props else { return };
                let (Some(class), Some(name)) = (
                    props.get(*pw::keys::MEDIA_CLASS),
                    props.get(*pw::keys::NODE_NAME),
                ) else {
                    return;
                };
                let Some(kind) = device_kind(class, name) else {
                    return;
                };
                let info = DeviceInfo {
                    id: name.to_string(),
                    name: device_label(
                        props.get(*pw::keys::NODE_DESCRIPTION),
                        props.get(*pw::keys::NODE_NICK),
                        name,
                    ),
                };
                let mut devices = devices.borrow_mut();
                devices.nodes.insert(obj.id, (kind, info));
                report(Command::Devices(devices.list()));
            }
        })
        .global_remove({
            let devices = devices.clone();
            let report = report.clone();
            move |id| {
                let mut devices = devices.borrow_mut();
                if devices.nodes.remove(&id).is_some() {
                    report(Command::Devices(devices.list()));
                }
            }
        })
        .register();

    let mic_props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CLASS => "Audio/Source/Virtual",
        *pw::keys::NODE_NAME => VIRTUAL_MIC_NODE,
        *pw::keys::NODE_DESCRIPTION => VIRTUAL_MIC_DESCRIPTION,
        *pw::keys::NODE_LATENCY => NODE_LATENCY,
        *pw::keys::PRIORITY_SESSION => "0",
        *pw::keys::PRIORITY_DRIVER => "0",
    };
    let _mic = output_stream(
        &core,
        "soundboard-mic",
        mic_props,
        pump.clone(),
        Bus::Mic,
        StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
        Some(report.clone()),
    )?;

    let graph = Rc::new(RefCell::new(Graph {
        core: core.clone(),
        pump,
        report: report.clone(),
        config: None,
        monitor: None,
        capture: None,
    }));
    let _receiver = rx.attach(mainloop.loop_(), {
        let graph = graph.clone();
        let mainloop = mainloop.clone();
        move |msg| match msg {
            Msg::Configure(cfg) => graph.borrow_mut().apply(cfg),
            Msg::Terminate => mainloop.quit(),
        }
    });

    let _ = ready.send(Ok(()));
    report(Command::BackendStatus(BackendStatus::Ok));
    mainloop.run();

    let mut graph = graph.borrow_mut();
    graph.monitor = None;
    if graph.capture.take().is_some() {
        report(Command::MicClosed);
    }
    Ok(())
}
