use std::error::Error;
use std::fmt::Display;
use std::io::{self, Write};
use std::panic::Location;
use std::process::ExitCode;
use std::str::FromStr;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, cursor};

mod colorize;
use colorize::ToColored;

mod resp;
use resp::{RespResult, RespRoot};
use ureq::Agent;

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
                eprintln!("Not selected");
                return ExitCode::SUCCESS;
            }
        }
    }
    let tr = match translate(&word, lang) {
        Ok(resp) => resp,
        Err(err) => {
            eprintln!("ERROR: {}", err);
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

fn interactive(lang: Lang) -> io::Result<Option<String>> {
    let mut ip: Vec<char> = Vec::new();
    let mut index: usize = 0;
    let mut input_cursor: usize = 0;
    const POPUP_SZ: u16 = 8;
    const PROMPT: &str = "> ";

    let mut stdout = io::stdout().lock().into_raw_mode()?;
    let stdin = io::stdin().lock();
    let keys = stdin.keys();

    write!(
        stdout,
        "{}{}{}{}",
        "\n".repeat(POPUP_SZ as usize),
        cursor::Up(POPUP_SZ),
        clear::AfterCursor,
        PROMPT.green()
    )?;
    stdout.flush()?;
    let mut tr_results: Vec<String> = Vec::with_capacity(0);
    let mut agent = ureq::agent();
    for key in keys {
        let key = key?;
        let prev_ip_len = ip.len();
        match key {
            Key::Char('\n') => {
                write!(stdout, "{}\n\n\r", cursor::Down(POPUP_SZ),)?;
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
                write!(stdout, "{}\n\n\r", cursor::Down(POPUP_SZ),)?;
                stdout.flush()?;
                return Ok(None);
            }
            _ => continue,
        }
        if prev_ip_len != ip.len() {
            // let mut new_tr_results = Vec::new();
            // new_tr_results.extend(tr_results.into_iter());
            // new_tr_results.push("a bc".repeat(ip.len()).to_string());
            // tr_results = new_tr_results;
            let word = String::from_iter(ip.iter());
            if let Ok(r) = tureng_ac(&word, lang, &mut agent) {
                tr_results = r;
            }
        }

        write!(stdout, "{}\r{}", cursor::Down(1), clear::AfterCursor)?;

        for (i, s) in tr_results.iter().take(POPUP_SZ as usize).enumerate() {
            if i == index {
                write!(
                    stdout,
                    "{} {}{}\r",
                    '↪'.green(),
                    s.white_bg(),
                    cursor::Down(1)
                )?;
            } else {
                write!(stdout, "{}  {}{}\r", '↪'.green(), s, cursor::Down(1))?;
            };
        }

        write!(
            stdout,
            "{}{}{}{}\r{}",
            cursor::Up(POPUP_SZ.min(tr_results.len() as u16)),
            clear::CurrentLine,
            PROMPT.green(),
            String::from_iter(ip.iter()),
            cursor::Right((PROMPT.len() + input_cursor) as u16)
        )?;
        stdout.flush()?;
    }
    Ok(None)
}

#[allow(clippy::result_large_err)]
fn tureng_ac(word: &str, lang: Lang, agent: &mut Agent) -> Result<Vec<String>, LocError> {
    const BASE: &str = "http://ac.tureng.co/?t=";
    const PARAM: &str = "&l=";
    let lang = lang.to_str();
    let mut url = String::with_capacity(BASE.len() + PARAM.len() + word.len() + 4);
    url.push_str("http://ac.tureng.co/?t=");
    url.push_str(word);
    url.push_str("&l=");
    url.push_str(lang);
    let r = agent.get(&url).call()?;
    Ok(r.into_json::<Vec<String>>()?)
}

#[allow(clippy::result_large_err)]
fn translate(word: &str, lang: Lang) -> Result<RespRoot, LocError> {
    const BASE: &str = "http://api.tureng.com/v1/dictionary/";
    let lang = lang.to_str();
    let mut url = String::with_capacity(BASE.len() + 1 + word.len() + 4);
    url.push_str(BASE);
    url.push_str(lang);
    url.push('/');
    url.push_str(word);
    let r = ureq::get(&url).call()?.into_json::<RespRoot>()?;
    Ok(r)
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy)]
pub enum Lang {
    ENDE,
    ENES,
    ENFR,
    ENTR,
}

impl FromStr for Lang {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ende" => Ok(Self::ENDE),
            "enes" => Ok(Self::ENES),
            "enfr" => Ok(Self::ENFR),
            "entr" => Ok(Self::ENTR),
            _ => Err(()),
        }
    }
}

impl Lang {
    fn to_str(self) -> &'static str {
        match self {
            Lang::ENDE => "ende",
            Lang::ENES => "enes",
            Lang::ENFR => "enfr",
            Lang::ENTR => "entr",
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum LocErr {
    IOErr(io::Error),
    UreqErr(ureq::Error),
}

#[derive(Debug)]
pub struct LocError {
    err: LocErr,
    loc: &'static Location<'static>,
}

impl LocError {
    #[track_caller]
    pub fn new(err: LocErr) -> Self {
        Self {
            err,
            loc: Location::caller(),
        }
    }
}

impl Error for LocError {}
impl Display for LocError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{err:?}, {loc}", err = self.err, loc = self.loc)
    }
}
impl From<ureq::Error> for LocError {
    #[track_caller]
    fn from(value: ureq::Error) -> Self {
        Self::new(LocErr::UreqErr(value))
    }
}
impl From<io::Error> for LocError {
    #[track_caller]
    fn from(value: io::Error) -> Self {
        Self::new(LocErr::IOErr(value))
    }
}
