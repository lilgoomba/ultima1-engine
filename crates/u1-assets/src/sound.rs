//! The original's ten PC-speaker effects (SPEC §19).
//!
//! The original drives the speaker by toggling its data bit directly and
//! times every half-wave with a counted busy loop, so its pitch follows the
//! CPU. Each effect is described here as the list of intervals between
//! speaker writes, measured in 8088 clock cycles (instruction timings of
//! the 8088 data sheet), and rendered at 4.77 MHz, the IBM PC the loops
//! were written for.

/// The 8088 clock of the IBM PC/XT.
pub const CPU_HZ: f64 = 4_772_727.0;

/// Number of effects (numbers 0..=9, as in the original).
pub const EFFECTS: u8 = 10;

// 8088 cycle costs of the instructions the loops use.
const OUT: u32 = 10;
const XOR_IMM: u32 = 4;
const MOV_IMM: u32 = 4;
const MOV_RR: u32 = 2;
const DEC16: u32 = 2;
const DEC8: u32 = 3;
const ADD_IMM: u32 = 4;
const CMP_IMM: u32 = 4;
const JCC_TAKEN: u32 = 16;
const JCC_NOT: u32 = 4;
const LOOP_TAKEN: u32 = 17;
const LOOP_NOT: u32 = 5;
const IN_PUSH_AND: u32 = 10 + 15 + 4;
const POP: u32 = 12;

/// A `dec reg16 / jnz` delay of `n` passes (0 means 65536).
fn wait16(n: u32) -> u32 {
    let n = if n == 0 { 65536 } else { n };
    n * DEC16 + (n - 1) * JCC_TAKEN + JCC_NOT
}

/// A `dec reg8 / jnz` (or `inc` to wrap) delay of `n` passes (0 means 256).
fn wait8(n: u32) -> u32 {
    let n = if n == 0 { 256 } else { n };
    n * DEC8 + (n - 1) * JCC_TAKEN + JCC_NOT
}

/// One effect: the speaker level and how long (in cycles) it is held,
/// from the routine's entry to its last speaker write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub spans: Vec<(bool, u32)>,
}

impl Effect {
    pub fn cycles(&self) -> u64 {
        self.spans.iter().map(|&(_, c)| u64::from(c)).sum()
    }
    pub fn millis(&self) -> f64 {
        self.cycles() as f64 * 1000.0 / CPU_HZ
    }
    /// Number of speaker writes that change its level.
    pub fn edges(&self) -> usize {
        self.spans.windows(2).filter(|w| w[0].0 != w[1].0).count()
    }
}

/// Builds the span list: `gap` cycles pass, then the speaker is written.
struct B {
    spans: Vec<(bool, u32)>,
    level: bool,
    acc: u32,
}
impl B {
    fn new() -> Self {
        B { spans: Vec::new(), level: false, acc: IN_PUSH_AND }
    }
    fn run(&mut self, c: u32) {
        self.acc += c;
    }
    /// `out 61h` with the current value, then `xor al,2`.
    fn out(&mut self, next: bool) {
        self.acc += OUT;
        self.spans.push((self.level, self.acc));
        self.acc = 0;
        self.level = next;
    }
    fn end(mut self) -> Effect {
        // pop ax; out 61h: the speaker is left off.
        self.acc += POP + OUT;
        self.spans.push((self.level, self.acc));
        Effect { spans: self.spans }
    }
}

/// The value written alternates: the first write is "off", then "on", ...
fn toggles() -> impl FnMut() -> bool {
    let mut on = true;
    move || {
        on = !on;
        on
    }
}

/// Effect `n` (0..=9).
pub fn effect(n: u8) -> Effect {
    let mut b = B::new();
    let mut t = toggles();
    match n {
        // 0: 16 half-waves of a 1000-pass wait (a low buzz).
        // 1: 48 half-waves of a 355-pass wait (a higher, longer buzz).
        0 | 1 => {
            let (count, inner) = if n == 0 { (16, 1000) } else { (48, 355) };
            b.run(MOV_IMM);
            for i in 0..count {
                b.out(t());
                b.run(XOR_IMM + MOV_IMM);
                if n == 1 {
                    b.run(MOV_IMM + wait16(inner) + DEC16 + JCC_NOT);
                } else {
                    b.run(wait16(inner));
                }
                b.run(if i + 1 < count { LOOP_TAKEN } else { LOOP_NOT });
            }
            // The last pass ends without a write; the routine's own
            // restore write follows.
        }
        // 2, 3, 4, 9: a wait that grows by 5 passes per half-wave (8-bit,
        // so it wraps), starting at 0x83 (0x7e + 5) or, for 9, at 5.
        2 | 3 | 4 | 9 => {
            let (count, mut w): (u32, u8) = match n {
                2 => (200, 0x7e),
                3 => (255, 0x7e),
                4 => (8, 0x7e),
                _ => (4, 0),
            };
            b.run(MOV_IMM * 2);
            for i in 0..count {
                w = w.wrapping_add(5);
                b.run(ADD_IMM + MOV_RR + wait8(u32::from(w)));
                b.out(t());
                b.run(XOR_IMM + DEC8 + if i + 1 < count { JCC_TAKEN } else { JCC_NOT });
            }
        }
        // 5 and 7: 26 steps of `reps` pulses; a pulse is "hi" passes then
        // "lo" passes; per step hi drops by one (8-bit, wrapping) and lo
        // rises by one, from lo = 1 up to 26.
        5 | 7 => {
            let (mut hi, reps): (u8, u32) = if n == 5 { (0xc0, 0x3c) } else { (5, 0x0f) };
            b.run(MOV_IMM * 2 + MOV_RR + MOV_IMM);
            for lo in 1u8..=26 {
                b.run(MOV_RR);
                for _ in 0..reps {
                    b.run(MOV_RR + wait8(u32::from(hi)));
                    b.out(t());
                    b.run(XOR_IMM + MOV_RR + wait8(u32::from(lo)));
                    b.out(t());
                    b.run(XOR_IMM + DEC8);
                    b.run(JCC_TAKEN);
                }
                b.acc -= JCC_TAKEN;
                hi = hi.wrapping_sub(1);
                b.run(JCC_NOT + DEC8 * 2 + CMP_IMM + if lo < 26 { JCC_TAKEN } else { JCC_NOT });
            }
        }
        // 6: 160 half-waves whose wait falls from 160 passes to 1 (a rising
        // whistle).
        6 => {
            b.run(MOV_IMM);
            for w in (1..=160u32).rev() {
                b.run(3 + MOV_RR + wait16(w));
                b.out(t());
                b.run(XOR_IMM + DEC8 + if w > 1 { JCC_TAKEN } else { JCC_NOT });
            }
        }
        // 8: 251 half-waves whose wait rises from 5 passes to 255 (a
        // falling whistle).
        8 => {
            b.run(MOV_IMM);
            for bl in (1..=251u32).rev() {
                b.out(t());
                b.run(XOR_IMM + MOV_RR + wait8(256 - bl) + DEC8 + if bl > 1 { JCC_TAKEN } else { JCC_NOT });
            }
        }
        _ => {}
    }
    b.end()
}

/// What a sound is for. The original only knows the ten effect numbers;
/// the engine also says which game event asked for one, so the optional
/// enhanced sound mode can give each event its own sound. A few events
/// (`cue_only`) are silent in the original and only sound in the enhanced
/// mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sfx {
    Bump,
    Crash,
    Burn,
    Error,
    Hit,
    Kill,
    Hurt,
    Death,
    Alarm,
    Explosion,
    MonsterAttack,
    Step,
    NpcStep,
    Magic,
    Quest,
    Resurrect,
    Victory,
    EnemySpell,
    SpellFail,
    Attack,
    Laser,
    EnemyLaser,
    Starfield,
    Miss,
    Dodge,
    Launch,
    Dock,
    Hyperjump,
}

/// How far (in squares, either way) a townsperson's step is heard in the
/// enhanced sound mode.
pub const NPC_STEP_RANGE: i32 = 4;

impl Sfx {
    pub const ALL: [Sfx; 28] = [
        Sfx::Bump,
        Sfx::Crash,
        Sfx::Burn,
        Sfx::Error,
        Sfx::Hit,
        Sfx::Kill,
        Sfx::Hurt,
        Sfx::Death,
        Sfx::Alarm,
        Sfx::Explosion,
        Sfx::MonsterAttack,
        Sfx::Step,
        Sfx::NpcStep,
        Sfx::Magic,
        Sfx::Quest,
        Sfx::Resurrect,
        Sfx::Victory,
        Sfx::EnemySpell,
        Sfx::SpellFail,
        Sfx::Attack,
        Sfx::Laser,
        Sfx::EnemyLaser,
        Sfx::Starfield,
        Sfx::Miss,
        Sfx::Dodge,
        Sfx::Launch,
        Sfx::Dock,
        Sfx::Hyperjump,
    ];

    /// The file stem in a sound pack (`sounds/<name>.wav`).
    pub fn name(self) -> &'static str {
        match self {
            Sfx::Bump => "bump",
            Sfx::Crash => "crash",
            Sfx::Burn => "burn",
            Sfx::Error => "error",
            Sfx::Hit => "hit",
            Sfx::Kill => "kill",
            Sfx::Hurt => "hurt",
            Sfx::Death => "death",
            Sfx::Alarm => "alarm",
            Sfx::Explosion => "explosion",
            Sfx::MonsterAttack => "monster_attack",
            Sfx::Step => "step",
            Sfx::NpcStep => "npc_step",
            Sfx::Magic => "magic",
            Sfx::Quest => "quest",
            Sfx::Resurrect => "resurrect",
            Sfx::Victory => "victory",
            Sfx::EnemySpell => "enemy_spell",
            Sfx::SpellFail => "spell_fail",
            Sfx::Attack => "attack",
            Sfx::Laser => "laser",
            Sfx::EnemyLaser => "enemy_laser",
            Sfx::Starfield => "starfield",
            Sfx::Miss => "miss",
            Sfx::Dodge => "dodge",
            Sfx::Launch => "launch",
            Sfx::Dock => "dock",
            Sfx::Hyperjump => "hyperjump",
        }
    }

    /// The original effect played for this event, or `None` for the
    /// events the original leaves silent.
    pub fn beep(self) -> Option<u8> {
        Some(match self {
            Sfx::Bump | Sfx::Crash | Sfx::Burn => 0,
            Sfx::Error => 1,
            Sfx::Hit | Sfx::Kill | Sfx::Hurt | Sfx::Death | Sfx::Alarm | Sfx::Explosion => 2,
            Sfx::MonsterAttack => 3,
            Sfx::Step | Sfx::NpcStep => 4,
            Sfx::Magic | Sfx::Quest | Sfx::Resurrect | Sfx::Victory | Sfx::EnemySpell => 5,
            Sfx::SpellFail => 6,
            Sfx::Attack => 7,
            Sfx::Laser | Sfx::EnemyLaser => 8,
            Sfx::Starfield => 9,
            Sfx::Miss | Sfx::Dodge | Sfx::Launch | Sfx::Dock | Sfx::Hyperjump => return None,
        })
    }

    /// The event assumed for a bare effect number (its commonest use).
    pub fn default_for(n: u8) -> Sfx {
        match n {
            0 => Sfx::Bump,
            1 => Sfx::Error,
            2 => Sfx::Hit,
            3 => Sfx::MonsterAttack,
            4 => Sfx::Step,
            5 => Sfx::Magic,
            6 => Sfx::SpellFail,
            7 => Sfx::Attack,
            8 => Sfx::Laser,
            _ => Sfx::Starfield,
        }
    }

    /// Whether a townsperson's step `dx`, `dy` squares from the player
    /// is near enough for the enhanced sound mode to play (the original
    /// beeps for every step).
    pub fn npc_step_heard(dx: i32, dy: i32) -> bool {
        dx.abs().max(dy.abs()) <= NPC_STEP_RANGE
    }

    pub fn cue_only(self) -> bool {
        self.beep().is_none()
    }
}

/// Renders an effect as signed 16-bit mono samples at `rate` Hz, `cpu_hz`
/// cycles per second, with the given amplitude. Each sample is the average
/// level over its time, which keeps the high-pitched parts from aliasing
/// into noise.
pub fn render(e: &Effect, rate: u32, cpu_hz: f64, amp: i16) -> Vec<i16> {
    let per = cpu_hz / f64::from(rate);
    let total = e.cycles() as f64;
    let n = (total / per).ceil() as usize;
    let mut out = Vec::with_capacity(n);
    let mut spans = e.spans.iter().copied();
    let mut cur = spans.next();
    let mut left = cur.map_or(0.0, |s| f64::from(s.1));
    for _ in 0..n {
        let mut need = per;
        let mut on = 0.0;
        while need > 0.0 {
            let Some((lvl, _)) = cur else { break };
            let take = need.min(left);
            if lvl {
                on += take;
            }
            need -= take;
            left -= take;
            if left <= 0.0 {
                cur = spans.next();
                left = cur.map_or(0.0, |s| f64::from(s.1));
            }
        }
        // Centre the wave on zero: off is -amp, on is +amp.
        let v = (2.0 * on / per - 1.0) * f64::from(amp);
        out.push(v.round() as i16);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_are_square_waves_of_the_documented_length() {
        // Level changes per effect (the restoring write after the last
        // span is not counted).
        let edges: Vec<usize> = (0..EFFECTS).map(|n| effect(n).edges()).collect();
        assert_eq!(edges, [15, 47, 199, 254, 7, 26 * 60 * 2 - 1, 159, 26 * 15 * 2 - 1, 250, 3].map(|x| x as usize));
        for n in 0..EFFECTS {
            let ms = effect(n).millis();
            assert!(ms > 0.05 && ms < 2000.0, "effect {n}: {ms} ms");
        }
    }

    #[test]
    fn buzz_pitches() {
        // Effect 0: half-wave of a 1000-pass wait = 18023 cycles: ~132 Hz.
        let e = effect(0);
        let half = e.spans[2].1;
        assert_eq!(half, 4 + 4 + wait16(1000) + 17 + 10);
        let hz = CPU_HZ / f64::from(2 * half);
        assert!((hz - 132.0).abs() < 1.0, "{hz}");
        // Effect 1 is about 2.8 times higher (a DOSBox capture gave
        // half-waves of 22 and 8 samples at 44.1 kHz: 2.75).
        let h1 = effect(1).spans[2].1;
        assert!((f64::from(half) / f64::from(h1) - 2.8).abs() < 0.05);
    }

    #[test]
    #[ignore]
    fn print_table() {
        for n in 0..EFFECTS {
            let e = effect(n);
            let halves: Vec<u32> = e.spans.iter().skip(1).map(|s| s.1).collect();
            let hz = |c: u32| CPU_HZ / f64::from(2 * c);
            println!(
                "{n}: {:.1} ms, {} changes, first {:.0} Hz, last {:.0} Hz",
                e.millis(),
                e.edges(),
                hz(halves[0]),
                hz(*halves.last().unwrap())
            );
        }
    }

    #[test]
    fn every_effect_has_an_event_and_names_are_unique() {
        for n in 0..EFFECTS {
            assert_eq!(Sfx::default_for(n).beep(), Some(n));
        }
        let mut names: Vec<&str> = Sfx::ALL.iter().map(|s| s.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), Sfx::ALL.len());
    }

    #[test]
    fn only_near_townspeople_steps_are_heard() {
        assert!(Sfx::npc_step_heard(0, 1) && Sfx::npc_step_heard(-4, 4) && Sfx::npc_step_heard(4, -2));
        assert!(!Sfx::npc_step_heard(5, 0) && !Sfx::npc_step_heard(0, -5) && !Sfx::npc_step_heard(-12, 9));
    }

    #[test]
    fn render_is_bounded_and_ends_silent() {
        let e = effect(6);
        let s = render(&e, 44100, CPU_HZ, 8000);
        assert!(s.iter().all(|&v| (-8000..=8000).contains(&v)));
        assert!(s.iter().any(|&v| v > 7000));
        let ms = s.len() as f64 * 1000.0 / 44100.0;
        assert!((ms - e.millis()).abs() < 0.1);
    }
}
