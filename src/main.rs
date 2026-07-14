//! horizon — a tiny landscape that lives in your statusline.
//!
//! The sky follows your real local time through ten palette keyframes
//! (night, pre-dawn, sunrise, morning, midday, afternoon, golden hour,
//! sunset, dusk, night). Night skies carry the actual phase of the moon,
//! and stars only come out when the sky is dark enough.
//!
//! The mountains are grown from a hash of your project path and git
//! branch — every branch has its own skyline. The sun (or moon) crosses
//! the ridge as your context window fills, trailing a bloom of light;
//! when it sets behind the right edge, the day is over: time to compact.
//!
//! Reads Claude Code's statusline JSON on stdin. Run with `--gallery`
//! to preview a full day in your terminal.

use chrono::{Datelike, Local, TimeZone, Timelike, Utc};
use serde_json::Value;
use std::f32::consts::TAU;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

// the scene stretches to fill whatever the terminal gives us
const MIN_SCENE: usize = 18;
const MAX_SCENE: usize = 80;
const RESET: &str = "\x1b[0m";
const BLOCKS: [char; 7] = [' ', '▁', '▂', '▃', '▄', '▅', '▆'];
const MOON: [char; 8] = ['○', '◔', '◐', '◕', '●', '◕', '◐', '◔'];

// ─── color ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Rgb(f32, f32, f32);

impl Rgb {
    fn mix(self, o: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        Rgb(
            self.0 + (o.0 - self.0) * t,
            self.1 + (o.1 - self.1) * t,
            self.2 + (o.2 - self.2) * t,
        )
    }
    fn fg(self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.0 as u8, self.1 as u8, self.2 as u8)
    }
    fn bg(self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.0 as u8, self.1 as u8, self.2 as u8)
    }
    fn lum(self) -> f32 {
        0.299 * self.0 + 0.587 * self.1 + 0.114 * self.2
    }
}

// ─── sky ────────────────────────────────────────────────────────────────
// keyframes: (hour, horizon, zenith, mountain ink)

type SkyKey = (f32, [f32; 3], [f32; 3], [f32; 3]);

const SKY: [SkyKey; 11] = [
    (0.0, [30.0, 34.0, 84.0], [9.0, 11.0, 31.0], [5.0, 6.0, 16.0]),
    (4.5, [64.0, 44.0, 110.0], [16.0, 15.0, 44.0], [8.0, 7.0, 22.0]),
    (6.0, [255.0, 120.0, 72.0], [58.0, 60.0, 130.0], [30.0, 20.0, 52.0]),
    (7.5, [255.0, 196.0, 120.0], [126.0, 168.0, 224.0], [60.0, 70.0, 110.0]),
    (11.0, [176.0, 220.0, 244.0], [92.0, 158.0, 220.0], [76.0, 96.0, 138.0]),
    (15.0, [168.0, 212.0, 240.0], [80.0, 146.0, 212.0], [72.0, 90.0, 132.0]),
    (17.5, [255.0, 180.0, 92.0], [110.0, 124.0, 198.0], [62.0, 56.0, 104.0]),
    (19.0, [255.0, 96.0, 86.0], [78.0, 48.0, 124.0], [38.0, 24.0, 64.0]),
    (20.5, [108.0, 64.0, 148.0], [28.0, 26.0, 72.0], [14.0, 10.0, 34.0]),
    (22.0, [44.0, 42.0, 96.0], [12.0, 13.0, 36.0], [6.0, 7.0, 20.0]),
    (24.0, [30.0, 34.0, 84.0], [9.0, 11.0, 31.0], [5.0, 6.0, 16.0]),
];

struct Sky {
    horizon: Rgb,
    zenith: Rgb,
    ink: Rgb,
}

fn sky_at(hour: f32) -> Sky {
    let h = hour.rem_euclid(24.0);
    let i = SKY.iter().rposition(|k| k.0 <= h).unwrap_or(0);
    let (h0, a1, a2, a3) = SKY[i];
    let (h1, b1, b2, b3) = SKY[(i + 1).min(SKY.len() - 1)];
    let t = if h1 > h0 { (h - h0) / (h1 - h0) } else { 0.0 };
    let rgb = |a: [f32; 3]| Rgb(a[0], a[1], a[2]);
    Sky {
        horizon: rgb(a1).mix(rgb(b1), t),
        zenith: rgb(a2).mix(rgb(b2), t),
        ink: rgb(a3).mix(rgb(b3), t),
    }
}

fn moon_phase_index() -> usize {
    // reference new moon: 2000-01-06 18:14 UTC; synodic month 29.530588 days
    let epoch = Utc.with_ymd_and_hms(2000, 1, 6, 18, 14, 0).unwrap();
    let days = (Utc::now() - epoch).num_seconds() as f64 / 86_400.0;
    let phase = (days.rem_euclid(29.530_588)) / 29.530_588;
    ((phase * 8.0).round() as usize) % 8
}

// ─── deterministic ridge ────────────────────────────────────────────────

fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn roll(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn ridge(seed: u64, width: usize) -> Vec<u8> {
    // two overlaid sine waves + jitter: smooth silhouettes, unique per seed
    let f1 = 0.25 + (seed & 0xff) as f32 / 255.0 * 0.20;
    let f2 = 0.65 + ((seed >> 8) & 0xff) as f32 / 255.0 * 0.45;
    let p1 = ((seed >> 16) & 0xff) as f32 / 255.0 * TAU;
    let p2 = ((seed >> 24) & 0xff) as f32 / 255.0 * TAU;
    let mut rng = Rng((seed >> 32) | 1);
    (0..width)
        .map(|x| {
            let fx = x as f32;
            let y = 2.8
                + 2.4 * (fx * f1 + p1).sin()
                + 1.4 * (fx * f2 + p2).sin()
                + (rng.roll() - 0.5) * 0.8;
            y.round().clamp(0.0, 6.0) as u8
        })
        .collect()
}

// ─── session info ───────────────────────────────────────────────────────

struct Info {
    dir: String,
    branch: Option<String>,
    model: String,
    ctx: Option<f32>,
    cost: Option<f64>,
    dur_ms: Option<u64>,
    added: u64,
    removed: u64,
    seed: u64,
}

impl Info {
    fn from_json(data: &Value) -> Info {
        let cwd = data
            .pointer("/workspace/current_dir")
            .or_else(|| data.get("cwd"))
            .and_then(Value::as_str)
            .unwrap_or(".")
            .to_string();
        let branch = git_branch(Path::new(&cwd));
        let dir = match std::env::var("HOME") {
            Ok(h) if cwd == h => "~".to_string(),
            _ => Path::new(&cwd)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| cwd.clone()),
        };
        let model = data
            .pointer("/model/display_name")
            .and_then(Value::as_str)
            .unwrap_or("claude")
            .trim_start_matches("Claude ")
            .to_string();
        let project = data
            .pointer("/workspace/project_dir")
            .and_then(Value::as_str)
            .unwrap_or(&cwd);
        let seed = fnv1a(&format!("{}:{}", project, branch.as_deref().unwrap_or("")));
        let cost_obj = data.get("cost");
        let get_u64 = |k: &str| {
            cost_obj
                .and_then(|c| c.get(k))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        };
        Info {
            dir,
            branch,
            model,
            ctx: context_fraction(data),
            cost: cost_obj
                .and_then(|c| c.get("total_cost_usd"))
                .and_then(Value::as_f64),
            dur_ms: cost_obj
                .and_then(|c| c.get("total_duration_ms"))
                .and_then(Value::as_u64),
            added: get_u64("total_lines_added"),
            removed: get_u64("total_lines_removed"),
            seed,
        }
    }
}

fn git_branch(start: &Path) -> Option<String> {
    let mut dir = start;
    loop {
        let dotgit = dir.join(".git");
        let head_path = if dotgit.is_dir() {
            Some(dotgit.join("HEAD"))
        } else if dotgit.is_file() {
            // worktree / submodule: ".git" is a file containing "gitdir: <path>"
            std::fs::read_to_string(&dotgit)
                .ok()?
                .trim()
                .strip_prefix("gitdir: ")
                .map(|p| Path::new(p).join("HEAD"))
        } else {
            None
        };
        if let Some(hp) = head_path {
            let head = std::fs::read_to_string(hp).ok()?;
            let head = head.trim();
            return Some(match head.strip_prefix("ref: refs/heads/") {
                Some(branch) => branch.to_string(),
                None => head.chars().take(7).collect(),
            });
        }
        dir = dir.parent()?;
    }
}

fn context_fraction(data: &Value) -> Option<f32> {
    if let Ok(v) = std::env::var("HORIZON_CTX")
        && let Ok(pct) = v.parse::<f32>()
    {
        return Some(pct / 100.0);
    }
    // official context fields (Claude Code ≥ 2.1 provides these)
    if let Some(cw) = data.get("context_window") {
        let size = cw
            .get("context_window_size")
            .and_then(Value::as_u64)
            .filter(|&s| s > 0);
        if let (Some(size), Some(u)) = (size, cw.get("current_usage").filter(|u| u.is_object()))
        {
            let n = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
            let used =
                n("input_tokens") + n("cache_creation_input_tokens") + n("cache_read_input_tokens");
            if used > 0 {
                return Some(used as f32 / size as f32);
            }
        }
        if let Some(p) = cw.get("used_percentage").and_then(Value::as_u64) {
            return Some(p as f32 / 100.0);
        }
    }
    // fallback: last usage entry in the transcript
    let path = data.get("transcript_path")?.as_str()?;
    Some(transcript_tokens(path)? as f32 / 200_000.0)
}

fn transcript_tokens(path: &str) -> Option<u64> {
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    f.seek(SeekFrom::Start(len.saturating_sub(256 * 1024))).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);
    for line in text.lines().rev() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if v.get("isSidechain").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let Some(u) = v.pointer("/message/usage") else {
            continue;
        };
        let n = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
        let total =
            n("input_tokens") + n("cache_read_input_tokens") + n("cache_creation_input_tokens");
        if total > 0 {
            return Some(total);
        }
    }
    None
}

// ─── the painting ───────────────────────────────────────────────────────

fn scene(hour: f32, info: &Info, width: usize) -> String {
    let sky = sky_at(hour);
    let heights = ridge(info.seed, width);
    let is_day = (6.0..19.5).contains(&hour.rem_euclid(24.0));

    // the sun/moon travels with context usage; without context data it
    // drifts with the hour instead, so the scene still moves
    let frac = info
        .ctx
        .unwrap_or(hour.rem_euclid(24.0) / 24.0)
        .clamp(0.0, 1.0);
    let body_x = (frac * (width - 1) as f32).round() as usize;

    let day_t = ((hour - 6.0) / 13.5).clamp(0.0, 1.0);
    let altitude = (day_t * std::f32::consts::PI).sin();
    let body_color = if is_day {
        Rgb(255.0, 122.0, 70.0).mix(Rgb(255.0, 232.0, 150.0), altitude)
    } else {
        Rgb(215.0, 225.0, 248.0)
    };
    let glow_color = if is_day {
        sky.horizon.mix(Rgb(255.0, 240.0, 200.0), 0.5)
    } else {
        sky.horizon.mix(Rgb(170.0, 190.0, 240.0), 0.45)
    };
    let glow_strength = if is_day { 0.85 } else { 0.4 };

    let base = sky.zenith.mix(sky.horizon, 0.30);
    let starry = sky.zenith.lum() < 48.0;
    let mut stars = Rng(info.seed ^ (Local::now().ordinal() as u64).wrapping_mul(0x9e37_79b9));
    let moon = MOON[moon_phase_index()];

    let mut out = String::new();
    for (x, &h) in heights.iter().enumerate() {
        let dx = (x as f32 - body_x as f32) / width as f32;
        let bloom = (-(dx * 3.2).powi(2)).exp() * glow_strength;
        let cell_sky = base.mix(glow_color, bloom);
        let star_roll = stars.roll();

        out.push_str(&cell_sky.bg());
        if x == body_x {
            out.push_str(&body_color.fg());
            out.push(if is_day { '●' } else { moon });
        } else if h == 0 && starry && star_roll < 0.30 {
            let twinkle = Rgb(150.0, 160.0, 190.0).mix(Rgb(235.0, 240.0, 255.0), stars.roll());
            out.push_str(&twinkle.fg());
            out.push(if star_roll < 0.05 { '✦' } else { '·' });
        } else {
            out.push_str(&sky.ink.fg());
            out.push(BLOCKS[h as usize]);
        }
    }
    out.push_str(RESET);
    out
}

fn visible_len(s: &str) -> usize {
    let mut n = 0;
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            in_escape = ch != 'm';
        } else if ch == '\x1b' {
            in_escape = true;
        } else {
            n += 1;
        }
    }
    n
}

fn terminal_cols() -> Option<usize> {
    if let Ok(v) = std::env::var("HORIZON_WIDTH")
        && let Ok(w) = v.parse::<usize>()
    {
        return Some(w);
    }
    // stdout is a pipe when Claude Code runs us; ask the controlling tty
    let tty = std::fs::File::open("/dev/tty").ok()?;
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let ret = unsafe {
        libc::ioctl(
            std::os::fd::AsRawFd::as_raw_fd(&tty),
            libc::TIOCGWINSZ,
            &mut ws,
        )
    };
    (ret == 0 && ws.ws_col > 0).then_some(ws.ws_col as usize)
}

/// `reserve`: columns already spoken for outside this line (labels, padding).
fn render(hour: f32, info: &Info, reserve: usize) -> String {
    let sky = sky_at(hour);
    let dim = Rgb(108.0, 112.0, 140.0);
    let text = Rgb(205.0, 210.0, 228.0);
    let accent = sky.horizon.mix(text, 0.55);
    let sep = format!(" {}·{} ", dim.fg(), RESET);

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("{}\x1b[1m{}{}", text.fg(), info.dir, RESET));
    if let Some(b) = &info.branch {
        parts.push(format!("{}⎇ {}{}", accent.fg(), b, RESET));
    }
    parts.push(format!(
        "{}✦ {}{}",
        Rgb(186.0, 164.0, 240.0).fg(),
        info.model,
        RESET
    ));
    if let Some(frac) = info.ctx {
        let pct = (frac * 100.0).round() as u32;
        let color = match pct {
            0..=49 => Rgb(158.0, 206.0, 106.0),
            50..=74 => Rgb(224.0, 175.0, 104.0),
            75..=89 => Rgb(255.0, 158.0, 100.0),
            _ => Rgb(247.0, 118.0, 142.0),
        };
        let fill = MOON[(frac * 4.0).round().clamp(0.0, 4.0) as usize];
        parts.push(format!("{}{} {}%{}", color.fg(), fill, pct, RESET));
    }
    if let Some(c) = info.cost
        && c >= 0.005
    {
        parts.push(format!(
            "{}${:.2}{}",
            Rgb(146.0, 196.0, 166.0).fg(),
            c,
            RESET
        ));
    }
    if let Some(ms) = info.dur_ms {
        let mins = ms / 60_000;
        let t = if mins >= 60 {
            format!("{}h{:02}m", mins / 60, mins % 60)
        } else {
            format!("{}m", mins)
        };
        parts.push(format!("{}{}{}", dim.fg(), t, RESET));
    }
    if info.added > 0 || info.removed > 0 {
        parts.push(format!(
            "{}+{}{} {}−{}{}",
            Rgb(120.0, 180.0, 120.0).fg(),
            info.added,
            RESET,
            Rgb(200.0, 120.0, 130.0).fg(),
            info.removed,
            RESET
        ));
    }

    let info_text = parts.join(&sep);
    let width = match terminal_cols() {
        Some(cols) => cols
            .saturating_sub(visible_len(&info_text) + 2 + reserve)
            .clamp(MIN_SCENE, MAX_SCENE),
        None => MIN_SCENE,
    };
    format!("{}  {}", scene(hour, info, width), info_text)
}

// ─── entry ──────────────────────────────────────────────────────────────

fn local_hour() -> f32 {
    if let Ok(v) = std::env::var("HORIZON_HOUR")
        && let Ok(h) = v.parse::<f32>()
    {
        return h;
    }
    let now = Local::now();
    now.hour() as f32 + now.minute() as f32 / 60.0
}

fn gallery() {
    let frames: [(f32, &str); 10] = [
        (0.5, "midnight"),
        (4.5, "pre-dawn"),
        (6.0, "sunrise"),
        (7.5, "morning"),
        (11.5, "midday"),
        (15.0, "afternoon"),
        (17.5, "golden hour"),
        (19.0, "sunset"),
        (20.5, "dusk"),
        (22.5, "night"),
    ];
    let dim = Rgb(108.0, 112.0, 140.0);
    let seed_str = std::env::var("HORIZON_SEED").unwrap_or_else(|_| "horizon:main".into());
    let (proj, branch) = seed_str.split_once(':').unwrap_or((&seed_str, "main"));
    println!();
    for (i, (hour, label)) in frames.iter().enumerate() {
        let info = Info {
            dir: proj.to_string(),
            branch: Some(branch.to_string()),
            model: "Fable 5".into(),
            ctx: Some(0.05 + 0.90 * i as f32 / (frames.len() - 1) as f32),
            cost: Some(0.87),
            dur_ms: Some(47 * 60_000),
            added: 120,
            removed: 33,
            seed: fnv1a(&seed_str),
        };
        println!(
            "  {}{:>12}{}  {}",
            dim.fg(),
            label,
            RESET,
            render(*hour, &info, 16)
        );
    }
    println!();
}

fn main() {
    if std::env::args().any(|a| a == "--gallery") {
        gallery();
        return;
    }
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    if let Ok(dump) = std::env::var("HORIZON_DUMP") {
        let path = if dump == "1" {
            format!(
                "{}/horizon-input.json",
                std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into())
            )
        } else {
            dump
        };
        let _ = std::fs::write(path, &input);
    }
    let data: Value = serde_json::from_str(&input).unwrap_or(Value::Null);
    let info = Info::from_json(&data);
    // Claude Code draws the statusline with a small margin of its own
    println!("{}", render(local_hour(), &info, 4));
}
