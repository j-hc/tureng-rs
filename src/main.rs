use std::io::{self, BufWriter, Write};
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::mpsc::{channel, sync_channel, Receiver, SyncSender};
use std::time::Instant;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, cursor};

mod colorize;
use colorize::ToColored;

mod api;
use api::{translate, tureng_ac, Lang, RespResult};

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

fn main() -> ExitCode {
    let mut envargs = std::env::args();
    let program = envargs.next().unwrap();
    let Some(args) = Args::get_args(&mut envargs) else {
        eprintln!("Usage: {program} <word> <optional --lang, -l ende|enes|enfr|entr> <optional --interactive, -i> <option --limit>");
        return ExitCode::FAILURE;
    };

    let word = if args.interactive {
        match interactive(args.lang, args.limit) {
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

    let tr = match translate(&word, args.lang) {
        Ok(resp) => resp,
        Err(err) => {
            eprintln!("ERROR: {err}");
            return ExitCode::FAILURE;
        }
    };
    if !tr.aresults.is_empty() {
        repr_results(tr.aresults, false);
    }
    if !tr.bresults.is_empty() {
        repr_results(tr.bresults, true);
    }
    ExitCode::SUCCESS
}

fn repr_results(mut results: Vec<RespResult>, swap: bool) {
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
    println!(
        "┌{:─^w1$}┐   ┌{:─^w2$}┐   ┌{:─^16}┐   ┌{:─^11}┐   ┌{:─^8}┐\n",
        INPUT.red(),
        TRANSLATION.red(),
        "Category".red(),
        "Term Type".red(),
        "Slang?".red()
    );
    w1 += 2;
    w2 += 2;
    for r in results {
        println!(
            "{: ^w1$}   {: ^w2$}   {: ^18}   {: ^13}   {: ^10}",
            r.term_a.magenta(),
            r.term_b.green(),
            r.category_text_b.yellow(),
            r.term_type_text_b.as_deref().unwrap_or("null").yellow(),
            r.is_slang.yellow()
        );
    }
}

struct UTF32String {
    inner: Vec<char>,
}
impl UTF32String {
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
            .position(|c| !c.is_ascii_whitespace())
            .unwrap_or(0);
        let e = self
            .inner
            .iter()
            .rposition(|c| !c.is_ascii_whitespace())
            .unwrap_or(self.inner.len());
        &self.inner[s..e]
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

struct AutoComplete {
    buf: Vec<u8>,
    lang: Lang,

    rx: Receiver<String>,
    tx: SyncSender<Vec<String>>,
}
impl AutoComplete {
    fn complete(&mut self) {
        while let Ok(r) = self.rx.recv() {
            if let Ok(r) = tureng_ac(&r, self.lang, &mut self.buf) {
                if self.tx.send(r).is_err() {
                    return;
                }
            }
        }
    }
}

fn interactive(lang: Lang, popup_sz: u16) -> io::Result<Option<String>> {
    let (keys_tx, keys_rx) = channel();
    std::thread::spawn(move || {
        for k in io::stdin().lock().keys().flatten() {
            if keys_tx.send(k).is_err() {
                break;
            }
        }
    });
    let (sender, ac_receiver) = sync_channel(0);
    let (ac_sender, receiver) = sync_channel(0);
    std::thread::spawn(move || {
        AutoComplete {
            buf: Vec::new(),
            lang,
            rx: ac_receiver,
            tx: ac_sender,
        }
        .complete()
    });

    let mut stdout = BufWriter::new(io::stdout().lock().into_raw_mode()?);
    const PROMPT: &str = "> ";
    let mut input = UTF32String { inner: Vec::new() };
    let mut index: usize = 0;
    let mut input_cursor: usize = 0;

    let mut results_len: u16 = 0;
    write!(
        stdout,
        "{}{}{}",
        "\n".repeat(popup_sz as usize),
        cursor::Up(popup_sz),
        PROMPT.green()
    )?;
    stdout.flush()?;

    let mut results = Vec::new();
    let mut now = Instant::now();
    let mut input_prev_hash = 0;

    let mut loading_i = 0;
    let mut is_loading = false;
    loop {
        let mut rerender = false;
        if let Ok(key) = keys_rx.try_recv() {
            match key {
                Key::Char('\n') => {
                    write!(stdout, "{}\n\r", cursor::Down(results_len))?;
                    stdout.flush()?;
                    return Ok(results.get(index).cloned());
                }
                Key::Backspace => {
                    if input_cursor > 0 {
                        input_cursor -= 1;
                        input.remove(input_cursor);
                    }
                }
                Key::Char(c) => {
                    input.insert(input_cursor, c);
                    input_cursor += 1;
                }
                Key::Right => {
                    if input_cursor < input.len() {
                        input_cursor += 1
                    }
                }
                Key::Left => input_cursor = input_cursor.saturating_sub(1),
                Key::Up => {
                    index = index.saturating_sub(1);
                    rerender = true;
                }
                Key::Down => {
                    if index + 1 < results_len.min(popup_sz) as usize {
                        rerender = true;
                        index += 1;
                    }
                }
                Key::Ctrl('c') => {
                    write!(stdout, "{}\n\r", cursor::Down(results_len))?;
                    stdout.flush()?;
                    return Ok(None);
                }
                Key::Ctrl('w') => {
                    let e = input.inner[..input_cursor]
                        .iter()
                        .rposition(|c| c.is_ascii_whitespace())
                        .unwrap_or(0);
                    input.inner.drain(e..input_cursor);
                    input_cursor = e;
                }
                _ => (),
            }
            now = Instant::now();
            write!(
                stdout,
                "\r{}{}{}",
                clear::CurrentLine,
                PROMPT.green(),
                input,
            )?;
            write!(
                stdout,
                "\r{}",
                cursor::Right((PROMPT.len() + input_cursor) as u16)
            )?;
            stdout.flush()?;
        }
        if input.trim().len() != 0
            && input.inner.iter().map(|c| *c as u32).sum::<u32>() != input_prev_hash
            && now.elapsed().as_millis() > 200
            && sender.try_send(input.to_string()).is_ok()
        {
            input_prev_hash = input.inner.iter().map(|c| *c as u32).sum();
            is_loading = true;
        }

        if let Ok(r) = receiver.try_recv() {
            is_loading = false;
            rerender = true;
            results = r;
        }

        if is_loading {
            const LOADING: &[&str] = &[
                "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▁",
            ];
            loading_i += 1;
            write!(
                stdout,
                "\r{}  {}\r{}",
                cursor::Right((PROMPT.len() + input.len()) as u16),
                LOADING[loading_i % LOADING.len()],
                cursor::Right((PROMPT.len() + input_cursor) as u16),
            )?;
            stdout.flush()?;
        }

        if rerender {
            write!(stdout, "\r\n{}", clear::AfterCursor)?;
            for (i, s) in results.iter().take(popup_sz as usize).enumerate() {
                write!(stdout, "{}", '↪'.green())?;
                if i == index {
                    write!(stdout, " {}", s.black().white_bg())?;
                } else {
                    write!(stdout, "  {s}")?;
                }
                write!(stdout, "\r\n")?;
            }
            results_len = results.len() as u16;

            write!(
                stdout,
                "{}\r{}{}{}\r{}",
                cursor::Up(popup_sz.min(results_len) + 1),
                clear::CurrentLine,
                PROMPT.green(),
                input,
                cursor::Right((PROMPT.len() + input_cursor) as u16)
            )?;

            stdout.flush()?;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
