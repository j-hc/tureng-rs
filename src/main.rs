use std::io::{self, BufWriter, Write};
use std::process::ExitCode;
use std::str::FromStr;
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
    fn get_args(args: &mut std::env::Args) -> Option<Self> {
        let mut res_args = Self::default();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--limit" => res_args.limit = args.next()?.parse().ok()?,
                "--interactive" | "-i" => res_args.interactive = true,
                "--lang" | "-l" => res_args.lang = Lang::from_str(&args.next()?).ok()?,
                _ => res_args.word = Some(arg),
            }
        }
        Some(res_args)
    }
}

fn main() -> ExitCode {
    let mut envargs = std::env::args();
    let program = envargs.next().unwrap();
    let Some(args) = Args::get_args(&mut envargs) else {
        eprintln!("Usage: {program} <word> <optional --lang, -l ende|enes|enfr|entr> <optional --interactive, -i>");
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
                eprintln!("No selection!");
                return ExitCode::SUCCESS;
            }
        }
    } else {
        if let Some(word) = args.word {
            word
        } else {
            eprintln!("ERROR: No word was supplied!");
            return ExitCode::FAILURE;
        }
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
    const WIDTH: usize = 30;
    const WIDTH2: usize = 32;
    if swap {
        for r in results.iter_mut() {
            std::mem::swap(&mut r.term_a, &mut r.term_b);
            std::mem::swap(&mut r.category_text_a, &mut r.category_text_b);
            std::mem::swap(&mut r.term_type_text_a, &mut r.term_type_text_b);
        }
    }
    println!(
        "┌{:─^WIDTH$}┐   ┌{:─^WIDTH$}┐   ┌{:─^WIDTH$}┐   ┌{:─^WIDTH$}┐   ┌{:─^WIDTH$}┐\n",
        "Input".red(),
        "Translation".red(),
        "Category".red(),
        "Term Type".red(),
        "Is slang?".red()
    );
    for r in results {
        println!(
            "{: ^WIDTH2$}   {: ^WIDTH2$}   {: ^WIDTH2$}   {: ^WIDTH2$}   {: ^WIDTH2$}",
            r.term_a.magenta(),
            r.term_b.green(),
            r.category_text_b.yellow(),
            r.term_type_text_b.as_deref().unwrap_or("null").yellow(),
            r.is_slang.yellow()
        );
    }
}

fn interactive(lang: Lang, popup_sz: u16) -> io::Result<Option<String>> {
    const PROMPT: &str = "> ";
    let mut input = String::new();
    let mut index: usize = 0;
    let mut input_cursor: usize = 0;

    let mut stdout = BufWriter::new(io::stdout().lock().into_raw_mode()?);
    let stdin = io::stdin().lock();

    write!(
        stdout,
        "{}{}{}",
        "\n".repeat(popup_sz as usize),
        cursor::Up(popup_sz),
        PROMPT.green()
    )?;
    stdout.flush()?;
    let mut tr_results: Vec<String> = Vec::new();
    let mut agent = ureq::agent();
    let mut resp_buf: Vec<u8> = Vec::new();
    for key in stdin.keys() {
        let key = key?;
        let prev_ip_len = input.len();
        match key {
            Key::Char('\n') => {
                write!(stdout, "{}\n\r", cursor::Down(tr_results.len() as u16))?;
                stdout.flush()?;
                return Ok(tr_results.get(index).cloned());
            }
            Key::Backspace => {
                if input_cursor > 0 {
                    input_cursor -= 1;
                    input.remove(input_cursor);
                }
            }
            Key::Char(c) => {
                input.push(c);
                input_cursor += 1;
            }
            Key::Right => {
                if input_cursor < input.len() {
                    input_cursor += 1
                }
            }
            Key::Left => input_cursor = input_cursor.saturating_sub(1),
            Key::Up => index = index.saturating_sub(1),
            Key::Down => {
                if index + 1 < tr_results.len().min(popup_sz as usize) {
                    index += 1;
                }
            }
            Key::Ctrl('c') => {
                write!(stdout, "{}\n\r", cursor::Down(tr_results.len() as u16))?;
                stdout.flush()?;
                return Ok(None);
            }
            _ => continue,
        }

        if prev_ip_len != input.len() {
            if let Ok(r) = tureng_ac(&input, lang, &mut agent, &mut resp_buf) {
                tr_results = r;
            }
        }
        write!(
            stdout,
            "\r{}{}{}\r\n",
            clear::AfterCursor,
            PROMPT.green(),
            input
        )?;

        for (i, s) in tr_results.iter().take(popup_sz as usize).enumerate() {
            write!(stdout, "{}", '↪'.green())?;
            if i == index {
                write!(stdout, " {}", s.white_bg())?;
            } else {
                write!(stdout, "  {}", s)?;
            }
            write!(stdout, "\r\n")?;
        }
        write!(
            stdout,
            "{}{}",
            cursor::Up(popup_sz.min(tr_results.len() as u16) + 1),
            cursor::Right((PROMPT.len() + input_cursor) as u16)
        )?;
        stdout.flush()?;
    }
    Ok(None)
}
