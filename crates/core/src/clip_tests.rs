use super::*;
use std::f32::consts::TAU;
use std::path::PathBuf;

pub(crate) fn write_wav(path: &Path, rate: u32, channels: u16, frames: &[f32]) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for &s in frames {
        writer
            .write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .unwrap();
    }
    writer.finalize().unwrap();
}

pub(crate) fn sine(rate: u32, channels: u16, seconds: f32, amp: f32) -> Vec<f32> {
    let frames = (rate as f32 * seconds) as usize;
    (0..frames)
        .flat_map(|i| {
            let v = amp * (TAU * 1000.0 * i as f32 / rate as f32).sin();
            std::iter::repeat_n(v, channels as usize)
        })
        .collect()
}

fn temp_wav(rate: u32, channels: u16, samples: &[f32]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("clip.wav");
    write_wav(&path, rate, channels, samples);
    (dir, path)
}

#[test]
fn mono_44k_is_upmixed_and_resampled_to_engine_rate() {
    let (_dir, path) = temp_wav(44_100, 1, &sine(44_100, 1, 1.0, 0.5));
    let clip = decode_file(&path).unwrap();
    let frames = clip.frames() as i64;
    assert!((frames - ENGINE_RATE as i64).abs() < 200, "frames {frames}");
    assert_eq!(clip.samples.len() % 2, 0);
    let mid = clip.frames() / 2 * 2;
    assert!((clip.samples[mid] - clip.samples[mid + 1]).abs() < 1e-6);
}

#[test]
fn stereo_48k_is_kept_sample_exact() {
    let mut samples = sine(48_000, 2, 0.25, 0.5);
    for frame in samples.chunks_mut(2) {
        frame[1] = 0.0;
    }
    let (_dir, path) = temp_wav(48_000, 2, &samples);
    let clip = decode_file(&path).unwrap();
    assert_eq!(clip.frames(), 12_000);
    assert!(clip.samples.iter().skip(1).step_by(2).all(|&s| s == 0.0));
    assert!(clip.samples.iter().step_by(2).any(|&s| s.abs() > 0.4));
}

#[test]
fn non_audio_file_is_a_decode_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.wav");
    std::fs::write(&path, "just some text, not audio").unwrap();
    assert!(decode_file(&path).is_err());
}

#[test]
fn empty_wav_is_rejected_as_empty() {
    let (_dir, path) = temp_wav(48_000, 2, &[]);
    assert!(matches!(decode_file(&path), Err(DecodeError::Empty)));
}

#[test]
fn clips_over_the_length_limit_are_rejected() {
    let (_dir, path) = temp_wav(48_000, 1, &sine(48_000, 1, 2.0, 0.5));
    let err = decode_with_limit(&path, 1).unwrap_err();
    assert!(matches!(err, DecodeError::TooLong { .. }), "{err:?}");
}

#[test]
fn peaks_cover_the_clip_in_buckets() {
    let mut samples = vec![0.0; 2 * 100];
    samples[150] = 0.8;
    let clip = Clip { samples };
    let peaks = clip.peaks(4);
    assert_eq!(peaks.len(), 4);
    assert_eq!(peaks[3], 0.8);
    assert_eq!(peaks[0], 0.0);
}
