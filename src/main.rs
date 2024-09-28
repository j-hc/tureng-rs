use api::{translate, tureng_ac, Lang, RespResult};
use crossterm::cursor::{MoveDown, MoveRight, MoveUp};
use crossterm::event::{EventStream, KeyCode, KeyModifiers};
use crossterm::queue;
use crossterm::style::{Print, Stylize};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use futures_util::{FutureExt, StreamExt};
use std::future::Future;
use std::io::{self, BufWriter, Write};
use std::pin::Pin;
use std::process::ExitCode;
use std::time::Duration;
use tokio::select;

use crate::colorize::ToColored;

mod api;
mod colorize;
struct Args {
    interactive: bool,
    lang: Lang,
    word: Option<String>,
    limit: u16,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            interactive: false,
            lang: Default::default(),
            word: None,
            limit: 9,
        }
    }
}

impl Args {
    fn get_args(envargs: &mut std::env::Args) -> Option<Self> {
        let mut args = Self::default();
        while let Some(arg) = envargs.next() {
            match arg.as_str() {
                "--limit" => args.limit = envargs.next()?.parse().ok()?,
                "--interactive" | "-i" => args.interactive = true,
                "--lang" | "-l" => args.lang = Lang::from_str(&envargs.next()?).ok()?,
                _ => args.word = Some(arg),
            }
        }

        if !args.interactive && args.word.is_none() {
            return None;
        }
        Some(args)
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let mut envargs = std::env::args();
    let program = envargs.next().expect("program name");
    let Some(args) = Args::get_args(&mut envargs) else {
        eprintln!("Usage: {program} <word> <optional --lang, -l ende|enes|enfr|entr> <optional --interactive, -i> <option --limit>");
        return ExitCode::FAILURE;
    };

    let mut stdout = BufWriter::new(io::stdout().lock());
    let word = if args.interactive {
        match interactive(&mut stdout, args.lang, args.limit).await {
            Ok(Some(w)) => w,
            Err(err) => {
                eprintln!("ERROR: {err}");
                return ExitCode::FAILURE;
            }
            _ => {
                eprintln!("{}", "No selection!".red());
                return ExitCode::SUCCESS;
            }
        }
    } else if let Some(word) = args.word {
        word
    } else {
        eprintln!("ERROR: No word was supplied!");
        return ExitCode::FAILURE;
    };

    let tr = match translate(&word, args.lang).await {
        Ok(resp) => resp,
        Err(err) => {
            eprintln!("ERROR: {err}");
            return ExitCode::FAILURE;
        }
    };
    disable_raw_mode().expect("disable raw mode");

    if !tr.aresults.is_empty() {
        repr_results(&mut stdout, tr.aresults, false).unwrap();
    }
    writeln!(stdout).unwrap();
    if !tr.bresults.is_empty() {
        repr_results(&mut stdout, tr.bresults, true).unwrap();
    }
    ExitCode::SUCCESS
}

fn repr_results(
    stdout: &mut impl Write,
    mut results: Vec<RespResult>,
    swap: bool,
) -> std::io::Result<()> {
    if swap {
        for r in results.iter_mut() {
            std::mem::swap(&mut r.term_a, &mut r.term_b);
            std::mem::swap(&mut r.category_text_a, &mut r.category_text_b);
            std::mem::swap(&mut r.term_type_text_a, &mut r.term_type_text_b);
        }
    }
    const INPUT: &str = "Input";
    const TRANSLATION: &str = "Translation";

    let mut w1 = INPUT.len();
    let mut w2 = TRANSLATION.len();
    for r in &results {
        w1 = w1.max(r.term_a.len());
        w2 = w2.max(r.term_b.len());
    }
    w1 = (w1 / 2) * 2 + 1;
    w2 = (w2 / 2) * 2 + 1;
    writeln!(
        stdout,
        "┌{:─^w1$}┐   ┌{:─^w2$}┐   ┌{:─^16}┐   ┌{:─^11}┐\n",
        INPUT.with_red(),
        TRANSLATION.with_red(),
        "Category".with_red(),
        "Term Type".with_red(),
    )?;
    w1 += 2;
    w2 += 2;
    for r in results {
        writeln!(
            stdout,
            "{: ^w1$}   {: ^w2$}   {: ^18}   {: ^14}",
            r.term_a.with_magenta(),
            r.term_b.with_green(),
            r.category_text_b.with_yellow(),
            r.term_type_text_b
                .as_deref()
                .unwrap_or("null")
                .with_yellow(),
        )?;
    }
    Ok(())
}

struct UTF32String {
    inner: Vec<char>,
}
impl UTF32String {
    fn new() -> Self {
        Self { inner: Vec::new() }
    }
    fn insert(&mut self, index: usize, element: char) {
        self.inner.insert(index, element)
    }
    fn len(&self) -> usize {
        self.inner.len()
    }
    fn remove(&mut self, i: usize) {
        self.inner.remove(i);
    }
    fn trim(&self) -> &[char] {
        let s = self
            .inner
            .iter()
            .position(|c| !c.is_whitespace())
            .unwrap_or(0);
        let e = self
            .inner
            .iter()
            .rposition(|c| !c.is_whitespace())
            .unwrap_or(self.inner.len());
        &self.inner[s..e]
    }
    fn to_string_clone(&self) -> String {
        String::from_iter(self.inner.iter())
    }
}

impl std::fmt::Display for UTF32String {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for c in &self.inner {
            write!(f, "{c}")?;
        }
        Ok(())
    }
}

const PROMPT: &str = "> ";

async fn interactive(
    stdout: &mut impl Write,
    lang: Lang,
    popup_sz: u16,
) -> io::Result<Option<String>> {
    enable_raw_mode().expect("raw mode");
    let mut events = EventStream::new();

    let mut input = UTF32String::new();
    let mut index: usize = 0;
    let mut input_cursor: usize = 0;

    queue!(
        stdout,
        Print("\n".repeat(popup_sz as usize)),
        MoveUp(popup_sz),
        Print(PROMPT.green())
    )?;
    stdout.flush()?;

    let mut results = Vec::new();
    let mut input_prev_hash = 0;
    let mut loading_i = 0;

    type OptionalBoxFuture<T> = Option<Pin<Box<dyn Future<Output = T>>>>;
    let mut tureng_ac_task: OptionalBoxFuture<Result<Vec<String>, reqwest::Error>> = None;

    let mut typing_delay = false;
    loop {
        let mut render_results = false;
        let event = events.next().fuse();
        select! {
            key = event => {
                let key = match key.unwrap().unwrap() {
                    crossterm::event::Event::Key(k) => k,
                    _ => continue,
                };
                match key.code {
                    KeyCode::Enter => {
                        queue!(stdout, MoveDown(results.len().min(popup_sz as usize) as u16), Print("\n\r"))?;
                        stdout.flush()?;
                        return Ok(results.get(index).cloned());
                    },
                    KeyCode::Backspace => {
                        if input_cursor > 0 {
                            input_cursor -= 1;
                            input.remove(input_cursor);
                        }
                    },
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        queue!(stdout, MoveDown(results.len() as u16), Print("\n\r"))?;
                        stdout.flush()?;
                        return Ok(None);
                    },
                    KeyCode::Char(c) => {
                        input.insert(input_cursor, c);
                        input_cursor += 1;
                    },
                    KeyCode::Right => {
                        if input_cursor < input.len() {
                            input_cursor += 1
                        }
                    },
                    KeyCode::Left => {
                        if input_cursor > 0 {
                            input_cursor -= 1
                        }
                    },
                    KeyCode::Up => {
                        index = index.saturating_sub(1);
                        render_results = true;
                    },
                    KeyCode::Down => {
                        if index + 1 < results.len().min(popup_sz as usize) {
                            render_results = true;
                            index += 1;
                        }
                    },
                    _ => continue,
                }
                queue!(
                    stdout,
                    Print("\r"),
                    Clear(ClearType::CurrentLine),
                    Print(PROMPT.green()),
                    Print(&input),
                    Print("\r"),
                    MoveRight((PROMPT.len() + input_cursor) as u16)
                )?;
                stdout.flush()?;

                let current_hash = input.inner.iter().map(|c| *c as u32).sum::<u32>();
                if !input.trim().is_empty() && current_hash != input_prev_hash {
                    input_prev_hash = current_hash;
                    tureng_ac_task = None;
                    typing_delay = true;
                } else {
                    typing_delay = false;
                }
            }

            ac = async { tureng_ac_task.as_mut().unwrap().await }, if tureng_ac_task.is_some() => {
                results = ac.expect("tureng request");
                render_results = true;
                tureng_ac_task = None;
            }

            _ = tokio::time::sleep(Duration::from_millis(250)), if typing_delay => {
                let input = input.to_string_clone();
                tureng_ac_task = Some(Box::pin(async move { tureng_ac(&input, lang).await }));
                typing_delay = false;
            }

            _ = tokio::time::sleep(Duration::from_millis(20)), if tureng_ac_task.is_some() => {
                const LOADING: &[&str] = &[
                    "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▁",
                ];
                loading_i = (loading_i + 1) % LOADING.len();
                queue!(stdout, Print("\r"), MoveRight((PROMPT.len() + input.len()) as u16))?;
                queue!(stdout, Print("  "), Print(LOADING[loading_i]))?;
                queue!(stdout, Print("\r"), MoveRight((PROMPT.len() + input_cursor) as u16))?;
                stdout.flush()?;
            }
        }

        if render_results {
            queue!(stdout, Print("\r\n"), Clear(ClearType::FromCursorDown))?;
            for (i, s) in results.iter().take(popup_sz as usize).enumerate() {
                let s = s.as_str();
                queue!(stdout, Print("↪".green()))?;
                if i == index {
                    queue!(stdout, Print(" "), Print(&s.black().on_white()))?;
                } else {
                    queue!(stdout, Print("  "), Print(s))?;
                }
                queue!(stdout, Print("\r\n"))?;
            }
            queue!(stdout, MoveUp(popup_sz.min(results.len() as u16) + 1))?;
            queue!(
                stdout,
                Print("\r"),
                Clear(ClearType::CurrentLine),
                Print(PROMPT.green()),
                Print(&input),
                Print("\r"),
                MoveRight((PROMPT.len() + input_cursor) as u16)
            )?;
            stdout.flush()?;
        }
    }
}
