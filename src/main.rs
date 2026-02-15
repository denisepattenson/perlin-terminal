use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    style,
    terminal,
};
use noise::{NoiseFn, Perlin};
use std::io::{self, Write, BufWriter};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "perlin-terminal", about = "Beautiful Perlin noise terminal animation")]
struct Args {
    /// Color theme: ocean, fire, aurora, matrix
    #[arg(short, long, default_value = "ocean")]
    theme: String,

    /// Noise scale (smaller = more zoomed in)
    #[arg(short, long, default_value_t = 0.06)]
    scale: f64,

    /// Animation speed multiplier
    #[arg(long, default_value_t = 0.4)]
    speed: f64,

    /// Target FPS
    #[arg(long, default_value_t = 60)]
    fps: u64,

    /// Noise seed
    #[arg(long, default_value_t = 42)]
    seed: u32,
}

fn color_from_theme(value: f64, theme: &str) -> (u8, u8, u8) {
    // value is roughly -1.0 to 1.0, normalize to 0.0-1.0
    let t = (value * 0.5 + 0.5).clamp(0.0, 1.0);

    match theme {
        "fire" => {
            // Black -> deep red -> orange -> yellow -> white
            if t < 0.2 {
                let s = t / 0.2;
                lerp_color((5, 0, 0), (140, 15, 0), s)
            } else if t < 0.45 {
                let s = (t - 0.2) / 0.25;
                lerp_color((140, 15, 0), (220, 80, 0), s)
            } else if t < 0.7 {
                let s = (t - 0.45) / 0.25;
                lerp_color((220, 80, 0), (255, 200, 30), s)
            } else {
                let s = (t - 0.7) / 0.3;
                lerp_color((255, 200, 30), (255, 255, 200), s)
            }
        }
        "aurora" => {
            // Deep purple -> teal -> green -> pink
            if t < 0.25 {
                let s = t / 0.25;
                lerp_color((20, 0, 40), (60, 20, 120), s)
            } else if t < 0.5 {
                let s = (t - 0.25) / 0.25;
                lerp_color((60, 20, 120), (0, 180, 160), s)
            } else if t < 0.75 {
                let s = (t - 0.5) / 0.25;
                lerp_color((0, 180, 160), (80, 255, 80), s)
            } else {
                let s = (t - 0.75) / 0.25;
                lerp_color((80, 255, 80), (220, 50, 180), s)
            }
        }
        "matrix" => {
            // Black -> dark green -> bright green
            if t < 0.4 {
                let s = t / 0.4;
                lerp_color((0, 2, 0), (0, 60, 0), s)
            } else if t < 0.7 {
                let s = (t - 0.4) / 0.3;
                lerp_color((0, 60, 0), (20, 180, 20), s)
            } else {
                let s = (t - 0.7) / 0.3;
                lerp_color((20, 180, 20), (150, 255, 150), s)
            }
        }
        _ => {
            // ocean: deep navy -> blue -> teal -> cyan
            if t < 0.2 {
                let s = t / 0.2;
                lerp_color((2, 2, 15), (10, 20, 80), s)
            } else if t < 0.45 {
                let s = (t - 0.2) / 0.25;
                lerp_color((10, 20, 80), (20, 60, 160), s)
            } else if t < 0.65 {
                let s = (t - 0.45) / 0.2;
                lerp_color((20, 60, 160), (0, 140, 180), s)
            } else if t < 0.85 {
                let s = (t - 0.65) / 0.2;
                lerp_color((0, 140, 180), (40, 220, 220), s)
            } else {
                let s = (t - 0.85) / 0.15;
                lerp_color((40, 220, 220), (180, 255, 255), s)
            }
        }
    }
}

#[inline(always)]
fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    (
        (a.0 as f64 + (b.0 as f64 - a.0 as f64) * t) as u8,
        (a.1 as f64 + (b.1 as f64 - a.1 as f64) * t) as u8,
        (a.2 as f64 + (b.2 as f64 - a.2 as f64) * t) as u8,
    )
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let perlin = Perlin::new(args.seed);
    let frame_duration = Duration::from_micros(1_000_000 / args.fps);

    // Setup terminal
    let mut stdout = BufWriter::new(io::stdout());
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let mut time: f64 = 0.0;

    loop {
        let frame_start = Instant::now();

        // Check for quit
        if event::poll(Duration::ZERO)? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q')
                    || key.code == KeyCode::Esc
                    || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    break;
                }
            }
        }

        let (cols, rows) = terminal::size()?;
        let cols = cols as usize;
        let rows = rows as usize;

        // Build frame buffer
        // Each terminal row uses ▀ with fg=top pixel, bg=bottom pixel
        // So we sample 2*rows vertical pixels
        execute!(stdout, cursor::MoveTo(0, 0))?;

        for row in 0..rows {
            let py_top = row * 2;
            let py_bot = row * 2 + 1;

            for col in 0..cols {
                let x = col as f64 * args.scale;
                let yt = py_top as f64 * args.scale;
                let yb = py_bot as f64 * args.scale;

                // Layer multiple octaves for richer noise
                let v_top = perlin.get([x, yt, time])
                    + 0.5 * perlin.get([x * 2.0, yt * 2.0, time * 1.3])
                    + 0.25 * perlin.get([x * 4.0, yt * 4.0, time * 1.7]);

                let v_bot = perlin.get([x, yb, time])
                    + 0.5 * perlin.get([x * 2.0, yb * 2.0, time * 1.3])
                    + 0.25 * perlin.get([x * 4.0, yb * 4.0, time * 1.7]);

                let (rt, gt, bt) = color_from_theme(v_top / 1.75, &args.theme);
                let (rb, gb, bb) = color_from_theme(v_bot / 1.75, &args.theme);

                write!(
                    stdout,
                    "\x1b[38;2;{rt};{gt};{bt}m\x1b[48;2;{rb};{gb};{bb}m▀"
                )?;
            }
            if row < rows - 1 {
                write!(stdout, "\x1b[0m\r\n")?;
            }
        }

        stdout.flush()?;
        time += args.speed * 0.016; // ~per frame at 60fps

        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    // Cleanup
    execute!(
        stdout,
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )?;
    terminal::disable_raw_mode()?;

    Ok(())
}
