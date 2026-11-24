//! Short retro square-wave chirp. Rendered to a WAV and handed to whatever player the OS has,
//! which avoids linking an audio library.
use std::io::Write;
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn wav(notes: &[f32], step: f32) -> Vec<u8> {
    const RATE: u32 = 22_050;
    let per = (step * RATE as f32) as usize;
    let mut pcm: Vec<i16> = vec![];
    for &freq in notes {
        for i in 0..per {
            let t = i as f32 / RATE as f32;
            // 10 ms attack, then a decay to silence over the note.
            let env = (t / 0.01).min(1.0) * (1.0 - i as f32 / per as f32).powi(2);
            let square = if (t * freq).fract() < 0.5 { 1.0 } else { -1.0 };
            pcm.push((square * env * 0.12 * i16::MAX as f32) as i16);
        }
    }
    let data = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend(b"RIFF");
    out.extend((36 + data).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend([1u8, 0, 1, 0]);
    out.extend(RATE.to_le_bytes());
    out.extend((RATE * 2).to_le_bytes());
    out.extend([2u8, 0, 16, 0]);
    out.extend(b"data");
    out.extend(data.to_le_bytes());
    out.extend(pcm.iter().flat_map(|s| s.to_le_bytes()));
    out
}

/// `notes` in Hz. Fire and forget; silently does nothing if no player is installed.
pub fn chirp(notes: &[f32]) {
    let bytes = wav(notes, 0.09);
    std::thread::spawn(move || {
        let file = std::env::temp_dir().join(format!("tcdev-{}-{}.wav", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed)));
        if std::fs::File::create(&file).and_then(|mut f| f.write_all(&bytes)).is_err() {
            return;
        }
        let path = file.to_string_lossy().to_string();
        let players: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
            &[("afplay", &[])]
        } else if cfg!(target_os = "windows") {
            &[("powershell", &["-NoProfile", "-Command"])]
        } else {
            &[("paplay", &[]), ("pw-play", &[]), ("aplay", &["-q"])]
        };
        for (program, args) in players {
            let mut cmd = crate::platform::quiet(program);
            cmd.args(*args);
            if cfg!(target_os = "windows") {
                cmd.arg(format!("(New-Object Media.SoundPlayer '{path}').PlaySync()"));
            } else {
                cmd.arg(&path);
            }
            if cmd.stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok() {
                break;
            }
        }
        let _ = std::fs::remove_file(file);
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn wav_has_a_valid_header() {
        let w = super::wav(&[660.0, 990.0], 0.09);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(w[40..44].try_into().unwrap()) as usize, w.len() - 44);
    }
}
