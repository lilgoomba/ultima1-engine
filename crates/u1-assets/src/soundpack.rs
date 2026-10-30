//! The optional enhanced sound mode (SPEC §19, "Enhanced sound"): one WAV
//! file per game event in an asset pack's `sounds/` folder. Events with no
//! file keep the original PC-speaker effect.

use crate::sound::Sfx;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

/// How effects are played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SoundMode {
    /// The original's PC-speaker effects (the default).
    #[default]
    Original,
    /// The sound pack's sounds, the original effects where it has none.
    Enhanced,
    /// Nothing is heard; the game still waits as long as the original.
    Mute,
}

impl SoundMode {
    pub fn parse(s: &str) -> Option<SoundMode> {
        match s {
            "original" | "beeps" => Some(SoundMode::Original),
            "enhanced" => Some(SoundMode::Enhanced),
            "mute" | "off" => Some(SoundMode::Mute),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            SoundMode::Original => "original",
            SoundMode::Enhanced => "enhanced",
            SoundMode::Mute => "mute",
        }
    }
}

/// Decoded sounds, mono 16-bit at the output rate.
#[derive(Debug, Default, Clone)]
pub struct SoundPack {
    pub rate: u32,
    pub clips: BTreeMap<Sfx, Arc<Vec<i16>>>,
    pub warnings: Vec<String>,
}

impl SoundPack {
    /// Loads `dir/<event>.wav` for every event, resampled to `rate`.
    pub fn load(dir: &Path, rate: u32) -> SoundPack {
        let mut p = SoundPack { rate, ..Default::default() };
        for s in Sfx::ALL {
            let f = dir.join(format!("{}.wav", s.name()));
            let Ok(bytes) = std::fs::read(&f) else { continue };
            match decode_wav(&bytes) {
                Ok((r, data)) => {
                    p.clips.insert(s, Arc::new(resample(&data, r, rate)));
                }
                Err(e) => p.warnings.push(format!("{}: {e}", f.display())),
            }
        }
        p
    }

    pub fn get(&self, s: Sfx) -> Option<&Arc<Vec<i16>>> {
        self.clips.get(&s)
    }
}

/// A review reel: for every event in turn, its original effect (if it
/// has one), a short gap, the pack's sound (if any), a longer gap. Returns
/// the samples at the pack's rate and one line per event.
pub fn demo_reel(p: &SoundPack) -> (Vec<i16>, Vec<String>) {
    let rate = p.rate.max(8000);
    let gap = |ms: u32| vec![0i16; (rate * ms / 1000) as usize];
    let mut out = gap(300);
    let mut lines = Vec::new();
    for (i, s) in Sfx::ALL.iter().enumerate() {
        let at = out.len() as f64 / f64::from(rate);
        let beep = s.beep().map(|n| crate::sound::render(&crate::sound::effect(n), rate, crate::sound::CPU_HZ, 6000));
        if let Some(b) = &beep {
            out.extend_from_slice(b);
            out.extend(gap(400));
        }
        let enh = p.get(*s);
        if let Some(c) = enh {
            out.extend_from_slice(c);
        }
        out.extend(gap(900));
        let what = match (s.beep(), enh.is_some()) {
            (Some(n), true) => format!("original effect {n}, then enhanced {}.wav", s.name()),
            (Some(n), false) => format!("original effect {n} only (no enhanced sound: falls back)"),
            (None, true) => format!("silent in the original; enhanced {}.wav", s.name()),
            (None, false) => "silent in both".to_string(),
        };
        lines.push(format!("{:2}. {:>2}:{:04.1}  {:<15} {what}", i + 1, (at / 60.0) as u32, at % 60.0, s.name()));
    }
    (out, lines)
}

/// Reads a PCM WAV (8- or 16-bit, any channel count; channels are
/// averaged). Returns the rate and mono samples.
pub fn decode_wav(b: &[u8]) -> Result<(u32, Vec<i16>), String> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("not a WAV file".into());
    }
    let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let u32_at = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let mut o = 12;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    while o + 8 <= b.len() {
        let id = &b[o..o + 4];
        let len = u32_at(o + 4) as usize;
        let body = o + 8;
        let end = (body + len).min(b.len());
        if id == b"fmt " && len >= 16 {
            fmt = Some((u16_at(body), u16_at(body + 2), u32_at(body + 4), u16_at(body + 14)));
        } else if id == b"data" {
            let (tag, ch, rate, bits) = fmt.ok_or("data before fmt")?;
            if tag != 1 {
                return Err(format!("format {tag} is not PCM"));
            }
            let ch = usize::from(ch.max(1));
            let d = &b[body..end];
            let samples: Vec<i32> = match bits {
                16 => d.chunks_exact(2).map(|c| i32::from(i16::from_le_bytes([c[0], c[1]]))).collect(),
                8 => d.iter().map(|&v| (i32::from(v) - 128) << 8).collect(),
                _ => return Err(format!("{bits}-bit samples are not supported")),
            };
            let mono = samples.chunks_exact(ch).map(|f| (f.iter().sum::<i32>() / ch as i32) as i16).collect();
            return Ok((rate, mono));
        }
        o = body + len + (len & 1);
    }
    Err("no data chunk".into())
}

/// Linear-interpolation resampling.
pub fn resample(s: &[i16], from: u32, to: u32) -> Vec<i16> {
    if from == to || s.is_empty() || from == 0 {
        return s.to_vec();
    }
    let n = (s.len() as u64 * u64::from(to) / u64::from(from)) as usize;
    let step = f64::from(from) / f64::from(to);
    (0..n)
        .map(|i| {
            let x = i as f64 * step;
            let k = x as usize;
            let f = x - k as f64;
            let a = f64::from(s[k.min(s.len() - 1)]);
            let b = f64::from(s[(k + 1).min(s.len() - 1)]);
            (a + (b - a) * f).round() as i16
        })
        .collect()
}

/// Writes mono 16-bit PCM as a WAV file image.
pub fn encode_wav(rate: u32, s: &[i16]) -> Vec<u8> {
    let data = (s.len() * 2) as u32;
    let mut v = Vec::with_capacity(44 + data as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data.to_le_bytes());
    for x in s {
        v.extend_from_slice(&x.to_le_bytes());
    }
    v
}

/// The beeps' level in the engine (the speaker wave's amplitude).
pub const BEEP_AMP: i16 = 6000;

/// Loudness as heard: the RMS of the loudest 50 ms of a sound (the whole
/// sound if shorter).
pub fn loudness(s: &[i16], rate: u32) -> f64 {
    let w = ((rate / 20) as usize).clamp(1, s.len().max(1));
    if s.is_empty() {
        return 0.0;
    }
    let sq: Vec<f64> = s.iter().map(|&x| f64::from(x) * f64::from(x)).collect();
    let mut acc: f64 = sq[..w.min(sq.len())].iter().sum();
    let mut best = acc;
    for i in w..sq.len() {
        acc += sq[i] - sq[i - w];
        best = best.max(acc);
    }
    (best / w as f64).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_round_trip_and_resample() {
        let s: Vec<i16> = (0..100).map(|i| (i * 100) as i16).collect();
        let (r, d) = decode_wav(&encode_wav(22050, &s)).unwrap();
        assert_eq!((r, d.clone()), (22050, s));
        let up = resample(&d, 22050, 44100);
        assert_eq!(up.len(), 200);
        assert_eq!(up[2], d[1]);
        assert_eq!(up[3], (d[1] + d[2]) / 2);
        assert!(decode_wav(b"RIFF....WAVEjunk").is_err());
    }

    #[test]
    fn stereo_8bit_is_mixed_down() {
        // A hand-built 8-bit stereo file: frames (255, 1) and (128, 128).
        let mut v = encode_wav(8000, &[]);
        v[22] = 2; // channels
        v[34] = 8; // bits
        v.truncate(40);
        v.extend_from_slice(&4u32.to_le_bytes());
        v.extend_from_slice(&[255, 1, 128, 128]);
        let (_, d) = decode_wav(&v).unwrap();
        assert_eq!(d, vec![0, 0]);
    }

    #[test]
    fn missing_files_fall_back() {
        let dir = std::env::temp_dir().join(format!("u1snd{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hit.wav"), encode_wav(44100, &[1, 2, 3])).unwrap();
        std::fs::write(dir.join("laser.wav"), b"junk").unwrap();
        let p = SoundPack::load(&dir, 44100);
        assert_eq!(p.get(Sfx::Hit).map(|c| c.as_slice()), Some(&[1i16, 2, 3][..]));
        assert!(p.get(Sfx::Laser).is_none());
        assert!(p.get(Sfx::Bump).is_none());
        assert_eq!(p.warnings.len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn demo_reel_lists_every_event() {
        let mut p = SoundPack { rate: 8000, ..Default::default() };
        p.clips.insert(Sfx::Hit, Arc::new(vec![100; 80]));
        let (w, l) = demo_reel(&p);
        assert_eq!(l.len(), Sfx::ALL.len());
        assert!(l[4].contains("hit") && l[4].contains("then enhanced"));
        assert!(l[0].contains("falls back"));
        assert!(w.len() > 8000 * 28);
    }

    #[test]
    fn shipped_pack_covers_every_event_and_credits_each() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/enhanced/sounds");
        let p = SoundPack::load(&dir, 44100);
        assert!(p.warnings.is_empty(), "{:?}", p.warnings);
        let credits = std::fs::read_to_string(dir.join("CREDITS")).unwrap();
        for s in Sfx::ALL {
            let c = p.get(s).unwrap_or_else(|| panic!("no {}.wav", s.name()));
            assert!(!c.is_empty() && c.len() < 44100 * 3, "{}", s.name());
            assert!(credits.contains(&format!("{}.wav", s.name())), "{} not credited", s.name());
        }
        assert!(credits.contains("CC-BY-SA 4.0"));
    }

    #[test]
    fn modes_parse() {
        for m in [SoundMode::Original, SoundMode::Enhanced, SoundMode::Mute] {
            assert_eq!(SoundMode::parse(m.as_str()), Some(m));
        }
        assert_eq!(SoundMode::default(), SoundMode::Original);
        assert_eq!(SoundMode::parse("loud"), None);
    }

    #[test]
    fn shipped_sounds_are_levelled_to_the_beeps() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/enhanced/sounds");
        let p = SoundPack::load(&dir, 44100);
        let beep = |n| loudness(&crate::sound::render(&crate::sound::effect(n), 44100, crate::sound::CPU_HZ, BEEP_AMP), 44100);
        for s in Sfx::ALL {
            let c = p.get(s).unwrap();
            let l = loudness(c, 44100);
            let peak = c.iter().map(|x| i32::from(*x).abs()).max().unwrap();
            assert!(peak <= 2 * i32::from(BEEP_AMP), "{} peaks at {peak}", s.name());
            let ms = c.len() as f64 * 1000.0 / 44100.0;
            match (s, s.beep()) {
                // Ambient: well under the beeps.
                (Sfx::Step | Sfx::NpcStep | Sfx::Starfield, Some(n)) => assert!(l < 0.2 * beep(n), "{} {l}", s.name()),
                // Roughly as loud as the original effect it replaces.
                (_, Some(n)) => {
                    let r = l / beep(n);
                    assert!((0.7..=0.95).contains(&r), "{} at {r:.2} of the beep", s.name());
                }
                // The original is silent here: quiet and short.
                (_, None) => {
                    assert!(l < 0.35 * f64::from(BEEP_AMP), "{} {l}", s.name());
                    assert!(ms <= 400.5, "{} lasts {ms} ms", s.name());
                }
            }
        }
    }
}
