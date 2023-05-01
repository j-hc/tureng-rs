pub mod resp;
pub use resp::*;

use miniserde::{json, Deserialize};
use std::error::Error;
use std::fmt::Display;
use std::io::{self, Read};
use std::panic::Location;
use std::str::FromStr;
use ureq::Agent;

pub fn tureng_ac(
    word: &str,
    lang: Lang,
    agent: &mut Agent,
    buf: &mut Vec<u8>,
) -> Result<Vec<String>, LocError> {
    let url = ["http://ac.tureng.co/?t=", word,  "&l=", lang.to_str()].concat();
    let r = agent.get(&url).call()?;
    let s = reader_to_json_with_buf(&mut r.into_reader(), buf)?;
    Ok(s)
}

pub fn translate(word: &str, lang: Lang) -> Result<RespRoot, LocError> {
    let url = ["http://api.tureng.com/v1/dictionary/", lang.to_str(), "/", word].concat();
    let r = ureq::get(&url).call()?;
    let s = reader_to_json::<RespRoot>(&mut r.into_reader())?;
    Ok(s)
}

fn reader_to_json<T: Deserialize>(r: &mut impl Read) -> Result<T, LocError> {
    let mut buf = Vec::new();
    reader_to_json_with_buf(r, &mut buf)
}

fn reader_to_json_with_buf<T: Deserialize>(
    r: &mut impl Read,
    buf: &mut Vec<u8>,
) -> Result<T, LocError> {
    unsafe { buf.set_len(0) };
    let sz = r.read_to_end(buf)?;
    let str = unsafe { std::str::from_utf8_unchecked(&buf[..sz]) };
    Ok(json::from_str(str)?)
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy)]
pub enum Lang {
    ENDE,
    ENES,
    ENFR,
    ENTR,
}

impl Default for Lang {
    fn default() -> Self {
        Self::ENTR
    }
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

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum LocErr {
    IO(io::Error),
    // UreqErr(ureq::Error),
    Ureq,
    Serde(miniserde::Error),
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
    fn from(_value: ureq::Error) -> Self {
        Self::new(LocErr::Ureq)
    }
}
impl From<io::Error> for LocError {
    #[track_caller]
    fn from(value: io::Error) -> Self {
        Self::new(LocErr::IO(value))
    }
}

impl From<miniserde::Error> for LocError {
    #[track_caller]
    fn from(value: miniserde::Error) -> Self {
        Self::new(LocErr::Serde(value))
    }
}
