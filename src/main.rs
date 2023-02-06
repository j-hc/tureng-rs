use std::io::{self, Write};
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

fn main() -> ExitCode {
    let mut args = std::env::args();
    let program = args.next().unwrap();
    let Some(mut word) = args.next() else {
        eprintln!("Usage: {program} <word> <optional ende|enes|enfr|entr>");
        return ExitCode::FAILURE;
    };
    let lang = if let Some(arg) = args.next() {
        let Ok(lang) = Lang::from_str(&arg) else {
            eprintln!("Usage: {program} <word> <optional ende|enes|enfr|entr>");
            return ExitCode::FAILURE;
        };
        lang
    } else {
        Lang::ENTR
    };

    if word == "-i" {
        match interactive(lang) {
            Ok(Some(w)) => word = w,
            Err(err) => {
                eprintln!("ERROR: {err}");
                return ExitCode::FAILURE;
            }
            _ => {
                eprintln!("Not selection!");
                return ExitCode::SUCCESS;
            }
        }
    }
    let tr = match translate(&word, lang) {
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
    println!();
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

fn interactive(lang: Lang) -> io::Result<Option<String>> {
    const POPUP_SZ: u16 = 10;
    const PROMPT: &str = "> ";
    let mut ip: Vec<char> = Vec::new();
    let mut index: usize = 0;
    let mut input_cursor: usize = 0;

    let mut stdout = io::stdout().lock().into_raw_mode()?;
    let stdin = io::stdin().lock();

    write!(
        stdout,
        "{}{}{}",
        "\n".repeat(POPUP_SZ as usize),
        cursor::Up(POPUP_SZ),
        PROMPT.green()
    )?;
    stdout.flush()?;
    let mut tr_results: Vec<String> = Vec::with_capacity(0);
    let mut agent = ureq::agent();
    let mut resp_buf: Vec<u8> = Vec::new();
    for key in stdin.keys() {
        let key = key?;
        let prev_ip_len = ip.len();
        match key {
            Key::Char('\n') => {
                write!(stdout, "{}\n\r", cursor::Down(tr_results.len() as u16))?;
                stdout.flush()?;
                return Ok(tr_results.get(index).cloned());
            }
            Key::Backspace => {
                if input_cursor > 0 {
                    input_cursor -= 1;
                    ip.remove(input_cursor);
                }
            }
            Key::Char(c) => {
                ip.push(c);
                input_cursor += 1;
            }
            Key::Right => {
                if input_cursor < ip.len() {
                    input_cursor += 1
                }
            }
            Key::Left => input_cursor = input_cursor.saturating_sub(1),
            Key::Up => index = index.saturating_sub(1),
            Key::Down => {
                if index + 1 < tr_results.len().min(POPUP_SZ as usize) {
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

        let ip_str = String::from_iter(ip.iter());
        if prev_ip_len != ip.len() {
            if let Ok(r) = tureng_ac(&ip_str, lang, &mut agent, &mut resp_buf) {
                tr_results = r;
            }
        }
        write!(
            stdout,
            "\r{}{}{}\r\n",
            clear::AfterCursor,
            PROMPT.green(),
            ip_str
        )?;

        for (i, s) in tr_results.iter().take(POPUP_SZ as usize).enumerate() {
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
            cursor::Up((POPUP_SZ.min(tr_results.len() as u16) + 1) as u16),
            cursor::Right((PROMPT.len() + input_cursor) as u16)
        )?;
        stdout.flush()?;
    }
    Ok(None)
}
