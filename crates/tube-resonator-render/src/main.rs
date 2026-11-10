//! Command-line WAV renderer for `tube-resonator-dsp`.
//!
//! Usage:
//!   cargo run -p tube-resonator-render -- <in.wav> <out.wav> [--no-lfo] [--no-filter] [--no-dc] [--tune <hz>] [--fine <ct>] [--filter-freq <hz>] [--lfo-phase-rad <rad>]

use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use tube_resonator_dsp::{TubeResonator, TubeResonatorParams};

fn read_wav_f32_stereo(path: &Path) -> Result<(u32, Vec<f32>, Vec<f32>), String> {
    let mut f = BufReader::new(File::open(path).map_err(|e| e.to_string())?);
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).map_err(|e| e.to_string())?;

    if buf.len() < 44 || &buf[0..4] != b"RIFF" || &buf[8..12] != b"WAVE" {
        return Err("Not a valid RIFF WAVE file".into());
    }

    let mut pos = 12;
    let mut audio_format = 0u16;
    let mut num_channels = 0u16;
    let mut sample_rate = 0u32;
    let mut bits_per_sample = 0u16;
    let mut data_slice: Option<&[u8]> = None;

    while pos + 8 <= buf.len() {
        let chunk_id = &buf[pos..pos + 4];
        let chunk_size = u32::from_le_bytes(buf[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let chunk_start = pos + 8;
        let chunk_end = (chunk_start + chunk_size).min(buf.len());

        if chunk_id == b"fmt " && chunk_size >= 16 {
            audio_format = u16::from_le_bytes(buf[chunk_start..chunk_start + 2].try_into().unwrap());
            num_channels =
                u16::from_le_bytes(buf[chunk_start + 2..chunk_start + 4].try_into().unwrap());
            sample_rate =
                u32::from_le_bytes(buf[chunk_start + 4..chunk_start + 8].try_into().unwrap());
            bits_per_sample =
                u16::from_le_bytes(buf[chunk_start + 14..chunk_start + 16].try_into().unwrap());
        } else if chunk_id == b"data" {
            data_slice = Some(&buf[chunk_start..chunk_end]);
            break;
        }
        pos = chunk_start + chunk_size + (chunk_size & 1);
    }

    let data = data_slice.ok_or("Missing data chunk")?;
    if audio_format != 3 || bits_per_sample != 32 {
        return Err(format!(
            "Expected 32-bit IEEE float WAV (format=3, bits=32), got format={audio_format}, bits={bits_per_sample}"
        ));
    }

    let ch = num_channels as usize;
    if ch == 0 {
        return Err("Invalid channel count 0".into());
    }
    let frames = data.len() / (4 * ch);
    let mut left = Vec::with_capacity(frames);
    let mut right = Vec::with_capacity(frames);

    for i in 0..frames {
        let base = i * 4 * ch;
        let l = f32::from_le_bytes(data[base..base + 4].try_into().unwrap());
        let r = if ch >= 2 {
            f32::from_le_bytes(data[base + 4..base + 8].try_into().unwrap())
        } else {
            l
        };
        left.push(l);
        right.push(r);
    }

    Ok((sample_rate, left, right))
}

fn write_wav_f32_stereo(
    path: &Path,
    sample_rate: u32,
    left: &[f32],
    right: &[f32],
) -> Result<(), String> {
    let frames = left.len().min(right.len());
    let data_bytes = (frames * 8) as u32;
    let fmt_size = 16u32;
    let riff_size = 4 + (8 + fmt_size) + (8 + data_bytes);

    let mut w = BufWriter::new(File::create(path).map_err(|e| e.to_string())?);
    w.write_all(b"RIFF").map_err(|e| e.to_string())?;
    w.write_all(&riff_size.to_le_bytes()).map_err(|e| e.to_string())?;
    w.write_all(b"WAVE").map_err(|e| e.to_string())?;

    w.write_all(b"fmt ").map_err(|e| e.to_string())?;
    w.write_all(&fmt_size.to_le_bytes()).map_err(|e| e.to_string())?;
    w.write_all(&3u16.to_le_bytes()).map_err(|e| e.to_string())?; // IEEE float
    w.write_all(&2u16.to_le_bytes()).map_err(|e| e.to_string())?; // stereo
    w.write_all(&sample_rate.to_le_bytes()).map_err(|e| e.to_string())?;
    w.write_all(&(sample_rate * 8).to_le_bytes()).map_err(|e| e.to_string())?;
    w.write_all(&8u16.to_le_bytes()).map_err(|e| e.to_string())?;
    w.write_all(&32u16.to_le_bytes()).map_err(|e| e.to_string())?;

    w.write_all(b"data").map_err(|e| e.to_string())?;
    w.write_all(&data_bytes.to_le_bytes()).map_err(|e| e.to_string())?;

    for i in 0..frames {
        w.write_all(&left[i].to_le_bytes()).map_err(|e| e.to_string())?;
        w.write_all(&right[i].to_le_bytes()).map_err(|e| e.to_string())?;
    }
    w.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <in.wav> <out.wav> [--no-lfo] [--no-filter] [--no-dc] [--no-limiter] [--tune <hz>] [--fine <ct>] [--filter-freq <hz>] [--filter-bw <oct>] [--lfo-phase-rad <rad>]",
            args[0]
        );
        std::process::exit(1);
    }

    let in_path = Path::new(&args[1]);
    let out_path = Path::new(&args[2]);
    let mut params = TubeResonatorParams::default();
    let mut lfo_phase_rad = 0.0_f64;

    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--no-lfo" => params.lfo_enabled = false,
            "--no-filter" => params.filter_enabled = false,
            "--no-dc" => params.dc_blocker_enabled = false,
            "--no-limiter" => params.safety_limiter = false,
            "--tune" => {
                i += 1;
                params.tune_hz = args[i]
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| e.to_string())?;
                params.fine_cents = 0.0;
            }
            "--fine" => {
                i += 1;
                params.fine_cents = args[i]
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| e.to_string())?;
            }
            "--filter-freq" => {
                i += 1;
                params.filter_freq_hz = args[i]
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| e.to_string())?;
            }
            "--filter-bw" => {
                i += 1;
                params.filter_bandwidth_oct = args[i]
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| e.to_string())?;
            }
            "--lfo-phase-rad" => {
                i += 1;
                lfo_phase_rad = args[i]
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| e.to_string())?;
            }
            other => return Err(format!("Unknown flag: {other}")),
        }
        i += 1;
    }

    let (sr, mut left, mut right) = read_wav_f32_stereo(in_path)?;
    let mut dsp = TubeResonator::new(sr as f64);
    dsp.set_lfo_phase_rad(lfo_phase_rad);
    dsp.process_block(&mut left, &mut right, &params);
    write_wav_f32_stereo(out_path, sr, &left, &right)?;

    println!(
        "Rendered {} frames at {} Hz -> {}",
        left.len(),
        sr,
        out_path.display()
    );
    Ok(())
}
