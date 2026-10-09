use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};
use std::fs::File;
use std::path::Path;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;

pub const ENGINE_RATE: u32 = 48_000;
pub const CHANNELS: usize = 2;
pub const MAX_CLIP_SECONDS: u64 = 600;

/// Decoded audio: interleaved stereo f32 at [`ENGINE_RATE`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Clip {
    pub samples: Vec<f32>,
}

impl Clip {
    pub fn frames(&self) -> usize {
        self.samples.len() / CHANNELS
    }

    pub fn duration_ms(&self) -> u64 {
        self.frames() as u64 * 1000 / ENGINE_RATE as u64
    }

    /// Absolute peak per bucket, for drawing a waveform.
    pub fn peaks(&self, buckets: usize) -> Vec<f32> {
        let frames = self.frames();
        if buckets == 0 || frames == 0 {
            return vec![0.0; buckets];
        }
        (0..buckets)
            .map(|b| {
                let start = b * frames / buckets;
                let end = ((b + 1) * frames / buckets).max(start + 1).min(frames);
                self.samples[start * CHANNELS..end * CHANNELS]
                    .iter()
                    .fold(0.0f32, |peak, s| peak.max(s.abs()))
            })
            .collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("could not open file: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported or unreadable audio: {0}")]
    Unsupported(String),
    #[error("the file contains no audio")]
    Empty,
    #[error("clip is longer than {limit_seconds} seconds")]
    TooLong { limit_seconds: u64 },
    #[error("resampling failed: {0}")]
    Resample(String),
}

pub fn decode_file(path: &Path) -> Result<Clip, DecodeError> {
    decode_with_limit(path, MAX_CLIP_SECONDS)
}

pub fn decode_with_limit(path: &Path, limit_seconds: u64) -> Result<Clip, DecodeError> {
    let (samples, channels, rate) = decode_native(path, limit_seconds)?;
    let stereo = to_stereo(&samples, channels);
    if stereo.is_empty() {
        return Err(DecodeError::Empty);
    }
    let samples = if rate == ENGINE_RATE {
        stereo
    } else {
        resample(&stereo, rate, ENGINE_RATE)?
    };
    Ok(Clip { samples })
}

fn unsupported(e: SymphoniaError) -> DecodeError {
    DecodeError::Unsupported(e.to_string())
}

fn decode_native(path: &Path, limit_seconds: u64) -> Result<(Vec<f32>, usize, u32), DecodeError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(unsupported)?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| DecodeError::Unsupported("no audio track".into()))?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| DecodeError::Unsupported("no audio codec parameters".into()))?
        .clone();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(unsupported)?;

    let mut out = Vec::new();
    let mut scratch: Vec<f32> = Vec::new();
    let mut spec = None;
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(SymphoniaError::ResetRequired) => break,
            Err(e) => return Err(unsupported(e)),
        };
        if packet.track_id != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(msg)) => {
                tracing::debug!("skipping undecodable packet: {msg}");
                continue;
            }
            Err(e) => return Err(unsupported(e)),
        };
        let (rate, channels) = *spec.get_or_insert_with(|| {
            let s = decoded.spec();
            (s.rate(), s.channels().count().max(1))
        });
        decoded.copy_to_vec_interleaved(&mut scratch);
        out.extend_from_slice(&scratch);
        if (out.len() / channels) as u64 > limit_seconds * rate as u64 {
            return Err(DecodeError::TooLong { limit_seconds });
        }
    }
    let (rate, channels) = spec.ok_or(DecodeError::Empty)?;
    Ok((out, channels, rate))
}

fn to_stereo(samples: &[f32], channels: usize) -> Vec<f32> {
    match channels {
        1 => samples.iter().flat_map(|&s| [s, s]).collect(),
        2 => samples.to_vec(),
        n => samples
            .chunks_exact(n)
            .flat_map(|frame| {
                let rest = frame[2..].iter().sum::<f32>() / (n - 2) as f32 * 0.5;
                [frame[0] + rest, frame[1] + rest]
            })
            .collect(),
    }
}

fn resample(stereo: &[f32], from: u32, to: u32) -> Result<Vec<f32>, DecodeError> {
    let err = |e: &dyn std::fmt::Display| DecodeError::Resample(e.to_string());
    let ratio = to as f64 / from as f64;
    let params = SincInterpolationParameters::new(128, WindowFunction::Blackman2)
        .oversampling_factor(256)
        .interpolation(SincInterpolationType::Quadratic);
    let mut resampler =
        Async::<f32>::new_sinc(ratio, 1.0, &params, 1024, CHANNELS, FixedAsync::Input)
            .map_err(|e| err(&e))?;
    let frames_in = stereo.len() / CHANNELS;
    let frames_out_capacity = resampler.process_all_needed_output_len(frames_in);
    let mut out = vec![0.0f32; frames_out_capacity * CHANNELS];
    let input = InterleavedSlice::new(stereo, CHANNELS, frames_in).map_err(|e| err(&e))?;
    let mut output =
        InterleavedSlice::new_mut(&mut out, CHANNELS, frames_out_capacity).map_err(|e| err(&e))?;
    let (_, frames_out) = resampler
        .process_all_into_buffer(&input, &mut output, frames_in, None)
        .map_err(|e| err(&e))?;
    out.truncate(frames_out * CHANNELS);
    Ok(out)
}

#[cfg(test)]
#[path = "clip_tests.rs"]
pub(crate) mod tests;
