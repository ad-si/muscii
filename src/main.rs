//! Muscii — parse ABC notation and render it as ASCII staff notation.

use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::process::ExitCode;

use abc_parser::abc;
use abc_parser::datatypes::{HeaderLine, MusicSymbol, Note, Tune, TuneLine};

/// Diatonic positions (one unit per letter step) of the five staff lines,
/// bottom to top: E4, G4, B4, D5, F5 (standard treble clef). Middle C (C4 -> 7)
/// sits one ledger line below the bottom line, so a C-major scale starts below
/// the staff and steps up.
///
/// Odd positions land on lines, even positions on the spaces between them.
const STAFF_LINES: [i32; 5] = [9, 11, 13, 15, 17];

const HEAD: char = '\u{2B24}'; // ⬤  note head (rendered two display columns wide)
const LOW_BLOCK: char = '\u{2584}'; // ▄  lower half block
const UP_BLOCK: char = '\u{2580}'; // ▀  upper half block
const HEAVY: char = '\u{2501}'; // ━  outer staff line / heavy frame
const LIGHT: char = '\u{2500}'; // ─  inner staff line / ledger line
const REST: char = '\u{0292}'; // ʒ  stand-in for a quarter rest (widely supported)

/// ANSI SGR codes used to render the rest glyph in bold on a terminal.
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

// Box frame: corners, side joints, and bar-line crossings.
const FRAME_TL: char = '\u{250F}'; // ┏
const FRAME_TR: char = '\u{2513}'; // ┓
const FRAME_BL: char = '\u{2517}'; // ┗
const FRAME_BR: char = '\u{251B}'; // ┛
const FRAME_L: char = '\u{2523}'; // ┣
const FRAME_R: char = '\u{252B}'; // ┫
const BAR_TOP: char = '\u{2533}'; // ┳
const BAR_MID: char = '\u{2542}'; // ╂
const BAR_BOT: char = '\u{253B}'; // ┻

/// Display columns used by each slot. A note head (`⬤`) renders one column wide
/// in terminals, so its second body column carries the staff line — the head
/// then reads as `⬤━`, matching the two-column half-block pairs. A bar line is a
/// single column.
const NOTE_BODY: usize = 2;
const BAR_BODY: usize = 1;
/// Separator column inserted before every slot.
const SEP: usize = 1;
/// Fill columns kept between the last slot and the right frame.
const TRAIL: usize = 2;

fn main() -> ExitCode {
  let mut args = env::args().skip(1);
  let source = match args.next() {
    Some(flag) if flag == "-h" || flag == "--help" => {
      print_usage();
      return ExitCode::SUCCESS;
    }
    Some(path) if path != "-" => match fs::read_to_string(&path) {
      Ok(text) => text,
      Err(err) => {
        eprintln!("muscii: cannot read {path}: {err}");
        return ExitCode::FAILURE;
      }
    },
    // No path, or "-": read ABC from stdin.
    _ => {
      let mut text = String::new();
      if let Err(err) = io::stdin().read_to_string(&mut text) {
        eprintln!("muscii: cannot read stdin: {err}");
        return ExitCode::FAILURE;
      }
      text
    }
  };

  let book = match abc::tune_book(&source) {
    Ok(book) => book,
    Err(err) => {
      eprintln!("muscii: failed to parse ABC: {err}");
      return ExitCode::FAILURE;
    }
  };

  if book.tunes.is_empty() {
    eprintln!("muscii: no tunes found in input");
    return ExitCode::FAILURE;
  }

  // Only emit ANSI styling when writing to a terminal.
  let bold = io::stdout().is_terminal();
  for (i, tune) in book.tunes.iter().enumerate() {
    if i > 0 {
      println!();
    }
    let tune = render_tune(tune);
    print!("{}", if bold { embolden_rests(&tune) } else { tune });
  }

  ExitCode::SUCCESS
}

/// Wrap each rest glyph in ANSI bold for terminal display. Kept out of
/// `render_tune` so the rendered text (and snapshots) stay plain.
fn embolden_rests(text: &str) -> String {
  text.replace(REST, &format!("{BOLD}{REST}{RESET}"))
}

fn print_usage() {
  println!("Muscii — render ABC notation as ASCII staff notation.\n");
  println!("Usage:");
  println!("    muscii <file.abc>    Render the given ABC file");
  println!("    muscii -             Read ABC from stdin");
  println!("    muscii               Read ABC from stdin");
}

/// Render a tune (title, key, and staff) to text, each line newline-terminated.
fn render_tune(tune: &Tune) -> String {
  let mut out = String::new();
  if let Some(title) = header_field(&tune.header.lines, 'T') {
    out.push_str(title);
    out.push('\n');
  }
  if let Some(key) = header_field(&tune.header.lines, 'K') {
    out.push_str("Key: ");
    out.push_str(key);
    out.push('\n');
  }

  let Some(body) = &tune.body else {
    out.push_str("(no music)\n");
    return out;
  };

  for line in &body.lines {
    if let TuneLine::Music(music) = line {
      let events = collect_events(&music.symbols);
      if events.is_empty() {
        continue;
      }
      for row in render_staff(&events) {
        out.push_str(row.trim_end());
        out.push('\n');
      }
    }
  }
  out
}

/// Find the value of the first header field with the given tag (e.g. 'T', 'K').
fn header_field(lines: &[HeaderLine], tag: char) -> Option<&str> {
  lines.iter().find_map(|line| match line {
    HeaderLine::Field(field, _) if field.0 == tag => Some(field.1.trim()),
    _ => None,
  })
}

/// A single horizontal slot on the staff.
enum Event {
  /// One or more note heads (a chord) at the given diatonic positions.
  Notes(Vec<i32>),
  Bar,
  Rest,
}

/// The diatonic position of a note: letter step plus seven steps per octave.
/// Matches the staff-line constants (middle C -> 7).
fn note_position(note: Note, octave: i8) -> i32 {
  let letter = match note {
    Note::C => 0,
    Note::D => 1,
    Note::E => 2,
    Note::F => 3,
    Note::G => 4,
    Note::A => 5,
    Note::B => 6,
  };
  letter + 7 * octave as i32
}

/// Reduce a line of parsed symbols to the events we lay out on the staff.
fn collect_events(symbols: &[MusicSymbol]) -> Vec<Event> {
  let mut events = Vec::new();
  for symbol in symbols {
    match symbol {
      MusicSymbol::Note { note, octave, .. } => {
        events.push(Event::Notes(vec![note_position(*note, *octave)]));
      }
      MusicSymbol::Chord { notes, .. } => {
        let positions: Vec<i32> = notes
          .iter()
          .filter_map(|n| match n {
            MusicSymbol::Note { note, octave, .. } => {
              Some(note_position(*note, *octave))
            }
            _ => None,
          })
          .collect();
        if !positions.is_empty() {
          events.push(Event::Notes(positions));
        }
      }
      MusicSymbol::Rest(_) => events.push(Event::Rest),
      MusicSymbol::Bar(..) => events.push(Event::Bar),
      // Beams, slurs, decorations, spaces, etc. take no horizontal slot.
      _ => {}
    }
  }
  events
}

/// True when a diatonic position sits on a line (rather than a space).
fn on_line(position: i32) -> bool {
  position.rem_euclid(2) == 1
}

/// Render a sequence of events into staff rows (top row first).
///
/// One text row per staff line. Line notes are drawn as a head on their line;
/// space notes fill the gap between the bracketing lines with half blocks. The
/// five staff lines are framed by a box whose top and bottom edges are the
/// outer (heavy) lines.
fn render_staff(events: &[Event]) -> Vec<String> {
  // A trailing bar line is redundant: the closing frame edge already terminates
  // the staff. Drop any so the final bar does not draw an extra interior line.
  let mut events = events;
  while let [rest @ .., Event::Bar] = events {
    events = rest;
  }

  // Vertical span, in line positions (odd values). Always cover the five staff
  // lines, then extend by whole lines so every note — and both lines bracketing
  // a space note — has a row.
  let mut top = STAFF_LINES[4];
  let mut bottom = STAFF_LINES[0];
  for event in events {
    if let Event::Notes(positions) = event {
      for &p in positions {
        let (hi, lo) = if on_line(p) { (p, p) } else { (p + 1, p - 1) };
        top = top.max(hi);
        bottom = bottom.min(lo);
      }
    }
  }

  let rows = ((top - bottom) / 2 + 1) as usize;
  let row_of = |level: i32| -> usize { ((top - level) / 2) as usize };

  let mut width = 1; // left frame
  for event in events {
    width += SEP
      + if matches!(event, Event::Bar) {
        BAR_BODY
      } else {
        NOTE_BODY
      };
  }
  width += TRAIL + 1; // trailing fill + right frame
  let last = width - 1;

  let mut grid = vec![vec![' '; width]; rows];

  // Draw the five staff lines across the full width, then the box frame.
  for (i, &level) in STAFF_LINES.iter().enumerate() {
    let r = row_of(level);
    let line = if i == 0 || i == STAFF_LINES.len() - 1 {
      HEAVY
    } else {
      LIGHT
    };
    for cell in &mut grid[r] {
      *cell = line;
    }
    let (l, rr) = if level == STAFF_LINES[4] {
      (FRAME_TL, FRAME_TR)
    } else if level == STAFF_LINES[0] {
      (FRAME_BL, FRAME_BR)
    } else {
      (FRAME_L, FRAME_R)
    };
    grid[r][0] = l;
    grid[r][last] = rr;
  }

  let mut col = 1; // just past the left frame
  for event in events {
    col += SEP;
    match event {
      Event::Notes(positions) => {
        // Lower half-blocks first, then upper, so a stacked chord of space
        // notes reads as one solid column (upper blocks win on the shared
        // lines). Heads are drawn last so they stay visible.
        for &p in positions {
          if !on_line(p) {
            draw_block(&mut grid, UP_BLOCK, p - 1, col, width, row_of);
          }
        }
        for &p in positions {
          if !on_line(p) {
            draw_block(&mut grid, LOW_BLOCK, p + 1, col, width, row_of);
          }
        }
        for &p in positions {
          if on_line(p) {
            draw_head(&mut grid, p, col, width, row_of);
          }
        }
        col += NOTE_BODY;
      }
      Event::Bar => {
        for &level in &STAFF_LINES {
          let ch = if level == STAFF_LINES[4] {
            BAR_TOP
          } else if level == STAFF_LINES[0] {
            BAR_BOT
          } else {
            BAR_MID
          };
          grid[row_of(level)][col] = ch;
        }
        col += BAR_BODY;
      }
      Event::Rest => {
        // A rest sits on the middle line; its second body column keeps the
        // staff line behind it.
        grid[row_of(STAFF_LINES[2])][col] = REST;
        col += NOTE_BODY;
      }
    }
  }

  grid
    .into_iter()
    .map(|row| row.into_iter().collect())
    .collect()
}

/// Draw a line note's head at `col` (the start of its two-column body). A head
/// occupies one column; the rest of its body and the following separator stay as
/// the staff line, so it reads `⬤━━`. Outside the staff it gets a short ledger
/// line of its own.
fn draw_head<F: Fn(i32) -> usize>(
  grid: &mut [Vec<char>],
  position: i32,
  col: usize,
  width: usize,
  row_of: F,
) {
  let r = row_of(position);
  if off_staff(position) {
    draw_ledger(grid, r, col, width);
  }
  grid[r][col] = HEAD;
}

/// Draw one half-block of a space note on the staff line at `level`. A space
/// note is a pair of these: `▄▄` on the line above and `▀▀` on the line below.
/// A block landing on a ledger line gets a short ledger line of its own.
fn draw_block<F: Fn(i32) -> usize>(
  grid: &mut [Vec<char>],
  block: char,
  level: i32,
  col: usize,
  width: usize,
  row_of: F,
) {
  let r = row_of(level);
  if off_staff(level) {
    draw_ledger(grid, r, col, width);
  }
  grid[r][col] = block;
  grid[r][col + 1] = block;
}

/// True when a diatonic position lies above or below the five staff lines.
fn off_staff(position: i32) -> bool {
  position < STAFF_LINES[0] || position > STAFF_LINES[4]
}

/// Draw a short ledger line through a note's two-column body (and the separators
/// flanking it), without overwriting the right frame.
fn draw_ledger(grid: &mut [Vec<char>], row: usize, col: usize, width: usize) {
  grid[row][col - 1] = LIGHT;
  grid[row][col + 1] = LIGHT;
  if col + 2 < width - 1 {
    grid[row][col + 2] = LIGHT;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn middle_c_sits_one_ledger_line_below_the_staff() {
    // Uppercase `C` parses to octave 1; middle C is a ledger line below the
    // bottom staff line (E4), which is two diatonic steps lower.
    assert_eq!(note_position(Note::C, 1), STAFF_LINES[0] - 2);
    // E4 is the bottom line of the staff.
    assert_eq!(note_position(Note::E, 1), STAFF_LINES[0]);
  }

  #[test]
  fn one_octave_is_seven_positions() {
    assert_eq!(note_position(Note::C, 2) - note_position(Note::C, 1), 7);
  }

  #[test]
  fn line_and_space_notes_alternate() {
    // C and E are lines; the D between them is a space.
    assert!(on_line(note_position(Note::C, 1)));
    assert!(!on_line(note_position(Note::D, 1)));
    assert!(on_line(note_position(Note::E, 1)));
  }

  #[test]
  fn renders_heads_for_lines_and_blocks_for_spaces() {
    let book = abc::tune_book("X:1\nT:Test\nK:C\nCDE |\n").unwrap();
    let body = book.tunes[0].body.as_ref().unwrap();
    let TuneLine::Music(line) = &body.lines[0] else {
      panic!("expected a music line");
    };
    let events = collect_events(&line.symbols);
    // Three notes plus one bar line.
    assert_eq!(events.len(), 4);

    let rows = render_staff(&events);
    // Five staff lines plus the ledger row holding middle C below them.
    assert_eq!(rows.len(), STAFF_LINES.len() + 1);
    // C and E are lines (two heads); D is a space (a half-block pair).
    let heads: usize = rows.iter().map(|r| r.matches(HEAD).count()).sum();
    assert_eq!(heads, 2);
    let blocks: usize = rows.iter().map(|r| r.matches(LOW_BLOCK).count()).sum();
    assert_eq!(blocks, 2);
    // The frame is present: heavy top line and heavy bottom line.
    assert!(rows.first().unwrap().starts_with(FRAME_TL));
    assert!(rows.iter().any(|r| r.starts_with(FRAME_BL)));
  }

  /// Render every tune in an ABC source the way the CLI does.
  fn render(source: &str) -> String {
    let book = abc::tune_book(source).unwrap();
    book.tunes.iter().map(render_tune).collect()
  }

  #[test]
  fn scale_example() {
    insta::assert_snapshot!(render(include_str!("../examples/scale.abc")));
  }

  #[test]
  fn chords_example() {
    insta::assert_snapshot!(render(include_str!("../examples/chords.abc")));
  }
}
