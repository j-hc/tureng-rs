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
    const BASE: &str = "http://ac.tureng.co/?t=";
    const PARAM: &str = "&l=";
    let mut url = String::with_capacity(BASE.len() + PARAM.len() + word.len() + 4);
    url.push_str(BASE);
    url.push_str(word);
    url.push_str(PARAM);
    url.push_str(lang.to_str());
    let r = agent.get(&url).call()?;
    let s = reader_to_json_with_buf(&mut r.into_reader(), buf)?;
    Ok(s)
}

pub fn translate(word: &str, lang: Lang) -> Result<RespRoot, LocError> {
    const BASE: &str = "http://api.tureng.com/v1/dictionary/";
    let lang = lang.to_str();
    let mut url = String::with_capacity(BASE.len() + 4 + 1 + word.len());
    url.push_str(BASE);
    url.push_str(lang);
    url.push('/');
    url.push_str(word);
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
    IOErr(io::Error),
    // UreqErr(ureq::Error),
    UreqErr,
    SerdeErr(miniserde::Error),
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
        Self::new(LocErr::UreqErr)
    }
}
impl From<io::Error> for LocError {
    #[track_caller]
    fn from(value: io::Error) -> Self {
        Self::new(LocErr::IOErr(value))
    }
}

impl From<miniserde::Error> for LocError {
    #[track_caller]
    fn from(value: miniserde::Error) -> Self {
        Self::new(LocErr::SerdeErr(value))
    }
}
