//! The engine's own area music (an option, off by default; SPEC §24).
//!
//! Nothing here comes from the original, which has no music. Every track
//! is an original composition kept as a compact score: a key, a tempo, a
//! chord progression, an arrangement and a seed for the melody writer
//! below. The score is rendered to samples at run time by our own small
//! chiptune synthesizer (pulse and triangle waves, a noise drum kit, a
//! soft pad and an echo), so the repository holds no audio files. The
//! compositions (the scores below) are CC-BY-SA 4.0 like the rest of the
//! enhanced material; the code is GPL-3.0-or-later like the engine (see
//! CREDITS).

use std::f32::consts::TAU;

/// What is playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Track {
    Title,
    Overworld,
    Town,
    Castle,
    Dungeon,
    Space,
    Lair,
    /// Short and played once: Mondain defeated.
    Victory,
    /// Short and played once: the player has died.
    Death,
}

impl Track {
    pub const ALL: [Track; 9] = [Track::Title, Track::Overworld, Track::Town, Track::Castle, Track::Dungeon, Track::Space, Track::Lair, Track::Victory, Track::Death];

    /// Whether it loops (the area tracks) or plays once (the stings).
    pub fn loops(self) -> bool {
        !matches!(self, Track::Victory | Track::Death)
    }

    pub fn name(self) -> &'static str {
        match self {
            Track::Title => "title",
            Track::Overworld => "overworld",
            Track::Town => "town",
            Track::Castle => "castle",
            Track::Dungeon => "dungeon",
            Track::Space => "space",
            Track::Lair => "lair",
            Track::Victory => "victory",
            Track::Death => "death",
        }
    }

    /// The area track for one of the game's modes (0 outdoors, 1 town,
    /// 2 castle, 3 dungeon).
    pub fn for_mode(mode: u8) -> Track {
        match mode {
            1 => Track::Town,
            2 => Track::Castle,
            3 => Track::Dungeon,
            _ => Track::Overworld,
        }
    }
}

#[derive(Clone, Copy)]
enum Inst {
    /// Pulse lead with this duty (eighths) and decay to a sustain.
    Lead(f32),
    /// Short plucked pulse.
    Pluck,
    /// Triangle bass, 4-bit stepped like the old chips.
    Bass,
    /// Thin fast arpeggio pulse.
    Arp,
    /// Two detuned soft saws through a low-pass: the "SNES strings".
    Pad,
    Kick,
    Snare,
    Hat,
}

struct Note {
    at: f32,
    len: f32,
    midi: f32,
    vel: f32,
    inst: Inst,
    wet: bool,
}

/// One chord: its root above the key's root (semitones) and its third,
/// fifth (and seventh) above that.
#[derive(Clone, Copy)]
struct Chord(i32, [i32; 3]);

const MAJ: [i32; 3] = [4, 7, 12];
const MIN: [i32; 3] = [3, 7, 12];
const DIM: [i32; 3] = [3, 6, 12];

struct Style {
    bpm: f32,
    root: i32,
    scale: [i32; 7],
    a: [Chord; 4],
    b: [Chord; 4],
    seed: u32,
    lead: Inst,
    /// Notes in the melody per bar, roughly (0: no melody).
    density: u32,
    arp: bool,
    pad: bool,
    /// Bass: 0 whole notes, 1 root-fifth halves, 2 driving eighths.
    bass: u8,
    /// Sixteen steps; k kick, s snare, h hat, '.' rest.
    drums: &'static str,
}

const IONIAN: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const AEOLIAN: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];
const DORIAN: [i32; 7] = [0, 2, 3, 5, 7, 9, 10];
const PHRYGIAN: [i32; 7] = [0, 1, 3, 5, 7, 8, 10];
const HARMONIC: [i32; 7] = [0, 2, 3, 5, 7, 8, 11];

fn style(t: Track) -> Option<Style> {
    let c = |r, q| Chord(r, q);
    Some(match t {
        Track::Title => Style {
            bpm: 76.0,
            root: 50,
            scale: AEOLIAN,
            a: [c(0, MIN), c(8, MAJ), c(3, MAJ), c(10, MAJ)],
            b: [c(5, MIN), c(0, MIN), c(8, MAJ), c(7, MAJ)],
            seed: 11,
            lead: Inst::Lead(0.5),
            density: 3,
            arp: true,
            pad: true,
            bass: 0,
            drums: "................",
        },
        Track::Overworld => Style {
            bpm: 128.0,
            root: 55,
            scale: IONIAN,
            a: [c(0, MAJ), c(7, MAJ), c(9, MIN), c(5, MAJ)],
            b: [c(5, MAJ), c(7, MAJ), c(4, MIN), c(9, MIN)],
            seed: 23,
            lead: Inst::Lead(0.25),
            density: 5,
            arp: true,
            pad: false,
            bass: 2,
            drums: "k.h.s.h.k.k.s.hh",
        },
        Track::Town => Style {
            bpm: 108.0,
            root: 53,
            scale: IONIAN,
            a: [c(0, MAJ), c(9, MIN), c(2, MIN), c(7, MAJ)],
            b: [c(5, MAJ), c(0, MAJ), c(2, MIN), c(7, MAJ)],
            seed: 37,
            lead: Inst::Pluck,
            density: 6,
            arp: false,
            pad: false,
            bass: 1,
            drums: "k...h...k.k.h...",
        },
        Track::Castle => Style {
            bpm: 92.0,
            root: 48,
            scale: IONIAN,
            a: [c(0, MAJ), c(5, MAJ), c(7, MAJ), c(0, MAJ)],
            b: [c(9, MIN), c(5, MAJ), c(0, MAJ), c(7, MAJ)],
            seed: 41,
            lead: Inst::Lead(0.5),
            density: 4,
            arp: false,
            pad: true,
            bass: 1,
            drums: "k...s.ssk...s.s.",
        },
        Track::Dungeon => Style {
            bpm: 66.0,
            root: 45,
            scale: PHRYGIAN,
            a: [c(0, MIN), c(1, MAJ), c(0, MIN), c(7, DIM)],
            b: [c(5, MIN), c(0, MIN), c(8, MAJ), c(7, DIM)],
            seed: 53,
            lead: Inst::Pluck,
            density: 2,
            arp: false,
            pad: true,
            bass: 0,
            drums: "k.........k.....",
        },
        Track::Space => Style {
            bpm: 100.0,
            root: 52,
            scale: DORIAN,
            a: [c(0, MIN), c(5, MAJ), c(0, MIN), c(5, MAJ)],
            b: [c(8, MAJ), c(10, MAJ), c(0, MIN), c(0, MIN)],
            seed: 67,
            lead: Inst::Lead(0.125),
            density: 3,
            arp: true,
            pad: true,
            bass: 1,
            drums: "k.....h.....h...",
        },
        Track::Lair => Style {
            bpm: 144.0,
            root: 47,
            scale: HARMONIC,
            a: [c(0, MIN), c(8, MAJ), c(7, MAJ), c(0, MIN)],
            b: [c(5, MIN), c(7, MAJ), c(8, MAJ), c(7, MAJ)],
            seed: 79,
            lead: Inst::Lead(0.25),
            density: 6,
            arp: true,
            pad: false,
            bass: 2,
            drums: "k.hsk.hsk.ksk.ss",
        },
        Track::Victory | Track::Death => return None,
    })
}

/// A tiny repeatable random sequence for the melody writer.
struct Rng(u32);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n.max(1)
    }
}

/// Bar rhythms in sixteenths, from sparse to busy.
const RHYTHMS: [&[u32]; 9] = [&[16], &[8, 8], &[12, 4], &[4, 4, 8], &[6, 2, 8], &[4, 4, 4, 4], &[6, 2, 4, 4], &[2, 2, 4, 4, 4], &[3, 3, 2, 4, 2, 2]];

/// The nearest scale degree's pitch to `want` among `root` + `scale`.
fn snap(want: i32, root: i32, scale: &[i32]) -> i32 {
    (want - 3..=want + 3).filter(|p| scale.contains(&((p - root).rem_euclid(12)))).min_by_key(|p| (p - want).abs()).unwrap_or(want)
}

/// The score as notes (times in beats).
fn score(t: Track) -> (Vec<Note>, f32, f32) {
    let n = |at: f32, len: f32, midi: i32, vel: f32, inst: Inst, wet: bool| Note { at, len, midi: midi as f32, vel, inst, wet };
    let mut v = Vec::new();
    let Some(s) = style(t) else {
        // The stings, written out.
        let (bpm, beats) = if t == Track::Victory {
            let tune = [(0.0, 0.33, 67), (0.33, 0.33, 72), (0.67, 0.33, 76), (1.0, 1.0, 79), (2.0, 0.5, 77), (2.5, 0.5, 79), (3.0, 3.0, 84)];
            for (a, l, m) in tune {
                v.push(n(a, l, m, 0.8, Inst::Lead(0.25), true));
                v.push(n(a, l, m - 12, 0.3, Inst::Arp, true));
            }
            for m in [60, 64, 67, 72] {
                v.push(n(3.0, 3.0, m, 0.35, Inst::Pad, true));
            }
            v.push(n(3.0, 3.0, 36, 0.8, Inst::Bass, false));
            for b in [0.0, 1.0, 2.0, 2.5, 3.0] {
                v.push(n(b, 0.25, 0, 0.8, if b == 3.0 { Inst::Kick } else { Inst::Snare }, false));
            }
            (120.0, 7.0)
        } else {
            let tune = [(0.0, 1.0, 76), (1.0, 1.0, 74), (2.0, 1.0, 72), (3.0, 1.0, 71), (4.0, 3.0, 69)];
            for (a, l, m) in tune {
                v.push(n(a, l, m, 0.7, Inst::Lead(0.5), true));
            }
            for m in [57, 60, 64] {
                v.push(n(0.0, 4.0, m - 12, 0.25, Inst::Pad, true));
                v.push(n(4.0, 3.0, m, 0.3, Inst::Pad, true));
            }
            v.push(n(4.0, 3.0, 33, 0.8, Inst::Bass, false));
            v.push(n(4.0, 0.5, 0, 0.9, Inst::Kick, false));
            (70.0, 8.0)
        };
        return (v, bpm, beats);
    };
    let mut r = Rng(s.seed.wrapping_mul(2_654_435_761).max(1));
    let bars = 16;
    // The melody: four-bar phrases A A' B A'', each bar's rhythm and shape
    // written once and replayed on the chord of the moment.
    let mut motifs: Vec<(Vec<u32>, Vec<i32>)> = Vec::new();
    for _ in 0..4 {
        let pick = (s.density as usize + r.below(3) as usize).saturating_sub(1).min(RHYTHMS.len() - 1);
        let rh = RHYTHMS[pick].to_vec();
        let shape = rh.iter().map(|_| r.below(5) as i32 - 2).collect();
        motifs.push((rh, shape));
    }
    let mut last = s.root + 24;
    for bar in 0..bars {
        let ch = if bar < 8 { s.a[bar % 4] } else { s.b[bar % 4] };
        let base = s.root + ch.0;
        let beat = bar as f32 * 4.0;
        let tones: Vec<i32> = std::iter::once(0).chain(ch.1).map(|x| base + x).collect();
        // Pad and arpeggio.
        if s.pad {
            for &x in &tones[..3] {
                v.push(n(beat, 4.0, x + 12, 0.16, Inst::Pad, true));
            }
        }
        if s.arp {
            let step = if s.bpm > 110.0 { 0.25 } else { 0.5 };
            let mut i = 0;
            let mut at = 0.0;
            while at < 4.0 {
                let x = tones[[0, 1, 2, 3, 2, 1][i % 6]];
                v.push(n(beat + at, step * 0.9, x + 12, 0.22, Inst::Arp, true));
                at += step;
                i += 1;
            }
        }
        // Bass.
        let low = base - 12 - if base - 12 > 45 { 12 } else { 0 };
        match s.bass {
            0 => v.push(n(beat, 4.0, low, 0.8, Inst::Bass, false)),
            1 => {
                v.push(n(beat, 2.0, low, 0.8, Inst::Bass, false));
                v.push(n(beat + 2.0, 2.0, low + ch.1[1], 0.7, Inst::Bass, false));
            }
            _ => {
                for k in 0..8 {
                    let x = if k % 4 == 3 { low + 12 } else { low };
                    v.push(n(beat + k as f32 * 0.5, 0.45, x, 0.7, Inst::Bass, false));
                }
            }
        }
        // Drums (a fill on each phrase's last bar).
        let pat: Vec<u8> = if bar % 4 == 3 && s.drums.contains('s') { "k.s.k.s.s.ssssss".bytes().collect() } else { s.drums.bytes().collect() };
        for (k, &d) in pat.iter().enumerate() {
            let inst = match d {
                b'k' => Inst::Kick,
                b's' => Inst::Snare,
                b'h' => Inst::Hat,
                _ => continue,
            };
            v.push(n(beat + k as f32 * 0.25, 0.25, 0, 0.7, inst, false));
        }
        // Melody.
        if s.density == 0 {
            continue;
        }
        let which = [0, 1, 2, 0][bar % 4] + if bar % 8 >= 4 { 0 } else { 0 };
        let (rh, shape) = &motifs[if bar >= 8 && bar % 4 != 3 { (which + 2) % 4 } else { which }];
        let mut at = 0u32;
        for (j, (&len, &d)) in rh.iter().zip(shape).enumerate() {
            let strong = at % 8 == 0;
            let mut want = last + d * 2;
            if want > s.root + 31 {
                want -= 7;
            }
            if want < s.root + 14 {
                want += 7;
            }
            let p = if strong {
                // A chord tone on the strong beats.
                *tones.iter().flat_map(|&x| [x + 12, x + 24]).collect::<Vec<_>>().iter().min_by_key(|&&x| (x - want).abs()).unwrap_or(&want)
            } else {
                snap(want, s.root, &s.scale)
            };
            let end_phrase = bar % 8 == 7 && j + 1 == rh.len();
            let p = if end_phrase { base + 24 - if base + 24 > s.root + 31 { 12 } else { 0 } } else { p };
            // A rest now and then keeps it breathing.
            if !(j > 0 && r.below(7) == 0) {
                v.push(n(beat + at as f32 / 4.0, len as f32 / 4.0 * 0.92, p, 0.55, s.lead, true));
            }
            last = p;
            at += len;
        }
    }
    (v, s.bpm, bars as f32 * 4.0)
}

fn midi_hz(m: f32) -> f32 {
    440.0 * 2f32.powf((m - 69.0) / 12.0)
}

/// Render one note into `out` (additive).
fn voice(out: &mut [f32], rate: f32, start: usize, n: &Note, secs: f32, noise: &mut Rng) {
    let f = midi_hz(n.midi);
    let total = ((secs + 0.35) * rate) as usize;
    let mut phase = 0f32;
    let mut phase2 = 0f32;
    let mut lp = 0f32;
    let mut held = 0f32;
    for i in 0..total {
        let Some(o) = out.get_mut(start + i) else { break };
        let t = i as f32 / rate;
        let rel = if t > secs { (1.0 - (t - secs) / 0.08).max(0.0) } else { 1.0 };
        let x = match n.inst {
            Inst::Lead(duty) => {
                let vib = if t > 0.18 { 1.0 + 0.004 * (TAU * 5.5 * t).sin() } else { 1.0 };
                phase = (phase + f * vib / rate).fract();
                let env = (t / 0.01).min(1.0) * (0.7 + 0.3 * (-t * 6.0).exp());
                let s = if phase < duty { 1.0 } else { -1.0 };
                s * env * rel * 0.32
            }
            Inst::Pluck => {
                phase = (phase + f / rate).fract();
                let s = if phase < 0.25 { 1.0 } else { -1.0 };
                s * (-t * 7.0).exp() * rel.min(1.0) * 0.36
            }
            Inst::Arp => {
                phase = (phase + f / rate).fract();
                let s = if phase < 0.125 { 1.0 } else { -1.0 };
                s * (-t * 10.0).exp() * rel * 0.3
            }
            Inst::Bass => {
                phase = (phase + f / rate).fract();
                let tri = 4.0 * (phase - 0.5).abs() - 1.0;
                ((tri * 8.0).round() / 8.0) * rel * (0.85 + 0.15 * (-t * 4.0).exp()) * 0.5
            }
            Inst::Pad => {
                phase = (phase + f * 1.003 / rate).fract();
                phase2 = (phase2 + f * 0.997 / rate).fract();
                let saw = (phase * 2.0 - 1.0) + (phase2 * 2.0 - 1.0);
                lp += (saw - lp) * 0.06;
                let env = (t / 0.25).min(1.0);
                let rel = if t > secs { (1.0 - (t - secs) / 0.3).max(0.0) } else { 1.0 };
                lp * env * rel * 0.25
            }
            Inst::Kick => {
                if t > 0.18 {
                    break;
                }
                let fk = 120.0 * (-t * 25.0).exp() + 45.0;
                phase = (phase + fk / rate).fract();
                (TAU * phase).sin() * (-t * 18.0).exp() * 0.9
            }
            Inst::Snare => {
                if t > 0.15 {
                    break;
                }
                let nz = (noise.next() >> 16) as f32 / 32768.0 - 1.0;
                phase = (phase + 190.0 / rate).fract();
                (nz * 0.7 + (TAU * phase).sin() * 0.3) * (-t * 22.0).exp() * 0.55
            }
            Inst::Hat => {
                if t > 0.05 {
                    break;
                }
                let nz = (noise.next() >> 16) as f32 / 32768.0 - 1.0;
                let hp = nz - held;
                held = nz;
                hp * (-t * 70.0).exp() * 0.25
            }
        };
        *o += x * n.vel;
    }
}

/// The track as mono 16-bit samples at `rate`. A looping track's end runs
/// seamlessly into its start (the tails of its last notes are folded
/// onto its beginning).
pub fn render(t: Track, rate: u32) -> Vec<i16> {
    let (notes, bpm, beats) = score(t);
    let r = rate as f32;
    let spb = 60.0 / bpm;
    let len = (beats * spb * r) as usize;
    let tail = (2.0 * r) as usize;
    let mut dry = vec![0f32; len + tail];
    let mut wet = vec![0f32; len + tail];
    let mut noise = Rng(0x1234_5678);
    for n in &notes {
        let start = (n.at * spb * r) as usize;
        voice(if n.wet { &mut wet } else { &mut dry }, r, start, n, n.len * spb, &mut noise);
    }
    // The echo: a dotted-eighth delay with feedback, on the wet bus.
    let d = ((0.75 * spb * r) as usize).max(1);
    for i in d..wet.len() {
        wet[i] += wet[i - d] * 0.32;
    }
    let mut mix: Vec<f32> = dry.iter().zip(&wet).map(|(a, b)| a + b).collect();
    // A soft low-pass over everything (the raw pulses are harsh; the
    // consoles' output stages rounded them off).
    let a = 1.0 - (-TAU * 5000.0 / r).exp();
    for _ in 0..2 {
        let mut y = 0f32;
        for x in mix.iter_mut() {
            y += (*x - y) * a;
            *x = y;
        }
    }
    if t.loops() {
        let (body, over) = mix.split_at_mut(len);
        for (i, x) in over.iter().enumerate() {
            body[i % len] += *x;
        }
        mix.truncate(len);
    } else {
        while mix.last().is_some_and(|x| x.abs() < 1e-4) {
            mix.pop();
        }
    }
    let peak = mix.iter().fold(0f32, |m, x| m.max(x.abs())).max(1e-6);
    let g = 0.45 * 32767.0 / peak;
    mix.iter().map(|x| (x * g).round().clamp(-32768.0, 32767.0) as i16).collect()
}

/// Music playback for a mixer: the current track, the one fading out,
/// the volume and the pause, mixed one sample at a time.
#[derive(Default)]
pub struct Player {
    cur: Option<(Track, std::sync::Arc<Vec<i16>>, usize)>,
    old: Option<(std::sync::Arc<Vec<i16>>, usize, bool)>,
    /// Samples into the crossfade, out of `fade_len`.
    fade: usize,
    fade_len: usize,
    /// 0.0 ..= 1.0.
    pub volume: f32,
    pub paused: bool,
}

impl Player {
    pub fn new(rate: u32) -> Self {
        Player { fade_len: rate as usize, volume: 0.5, ..Default::default() }
    }

    /// What is playing (or fading in).
    pub fn current(&self) -> Option<Track> {
        self.cur.as_ref().map(|c| c.0)
    }

    /// Change to `t` with its samples, crossfading from what played.
    pub fn start(&mut self, t: Track, s: std::sync::Arc<Vec<i16>>) {
        if self.current() == Some(t) {
            return;
        }
        self.old = self.cur.take().map(|(t0, s0, p)| (s0, p, t0.loops()));
        self.fade = if self.old.is_some() { 0 } else { self.fade_len };
        // A sting starts at once, over a quick fade of the area track.
        if !t.loops() {
            self.fade = self.fade_len * 3 / 4;
        }
        self.cur = Some((t, s, 0));
    }

    /// Fade out to silence.
    pub fn stop(&mut self) {
        if let Some((t, s, p)) = self.cur.take() {
            self.old = Some((s, p, t.loops()));
            self.fade = 0;
        }
    }

    /// The next sample to add into the mix.
    pub fn sample(&mut self) -> i32 {
        if self.paused {
            return 0;
        }
        let fl = self.fade_len.max(1) as f32;
        let k = (self.fade as f32 / fl).min(1.0);
        let mut acc = 0f32;
        if let Some((t, s, p)) = &mut self.cur {
            if *p >= s.len() && t.loops() {
                *p = 0;
            }
            if let Some(&x) = s.get(*p) {
                acc += f32::from(x) * k;
                *p += 1;
            }
        }
        if let Some((s, p, lp)) = &mut self.old {
            if *p >= s.len() && *lp {
                *p = 0;
            }
            if let Some(&x) = s.get(*p) {
                acc += f32::from(x) * (1.0 - k);
                *p += 1;
            }
        }
        if self.fade < self.fade_len {
            self.fade += 1;
        } else {
            self.old = None;
        }
        (acc * self.volume) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_track_renders_and_loops_or_ends() {
        for t in Track::ALL {
            let s = render(t, 8000);
            let secs = s.len() as f32 / 8000.0;
            if t.loops() {
                assert!((20.0..70.0).contains(&secs), "{t:?} {secs}");
            } else {
                assert!((2.0..10.0).contains(&secs), "{t:?} {secs}");
            }
            let peak = s.iter().map(|x| i32::from(*x).abs()).max().unwrap();
            assert!(peak > 10000 && peak < 16000, "{t:?} level {peak}");
        }
    }

    #[test]
    fn rendering_is_repeatable() {
        assert_eq!(render(Track::Town, 4000), render(Track::Town, 4000));
        assert_ne!(render(Track::Town, 4000), render(Track::Castle, 4000));
    }

    #[test]
    fn player_crossfades_pauses_and_loops() {
        use std::sync::Arc;
        let mut p = Player::new(10);
        p.volume = 1.0;
        assert_eq!(p.sample(), 0, "silent before anything starts");
        p.start(Track::Town, Arc::new(vec![1000; 5]));
        assert_eq!(p.sample(), 1000, "the first track starts at full level");
        for _ in 0..20 {
            assert_eq!(p.sample(), 1000, "and loops");
        }
        p.start(Track::Castle, Arc::new(vec![-1000; 50]));
        let first = p.sample();
        assert!(first > 500, "still mostly the old one: {first}");
        for _ in 0..12 {
            p.sample();
        }
        assert_eq!(p.sample(), -1000, "faded over");
        p.paused = true;
        assert_eq!(p.sample(), 0);
        p.paused = false;
        p.volume = 0.5;
        assert_eq!(p.sample(), -500);
        p.start(Track::Castle, Arc::new(vec![7; 5]));
        assert_eq!(p.sample(), -500, "the same track again carries on");
        p.start(Track::Death, Arc::new(vec![2000; 3]));
        for _ in 0..40 {
            p.sample();
        }
        assert_eq!(p.sample(), 0, "a sting plays once");
    }

    #[test]
    fn modes_have_tracks() {
        assert_eq!([0, 1, 2, 3].map(Track::for_mode), [Track::Overworld, Track::Town, Track::Castle, Track::Dungeon]);
    }
}
