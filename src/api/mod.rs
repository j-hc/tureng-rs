pub mod resp;
pub use resp::*;

use miniserde::{json};
use std::error::Error;
use std::fmt::Display;
use std::io::{self};
use std::panic::Location;
use std::str::FromStr;

pub async fn tureng_ac(
    word: String,
    lang: Lang,
) -> Result<Vec<String>, reqwest::Error> {
    let url = ["https://ac.tureng.co/?t=", &word, "&l=", lang.to_str()].concat();
    let r = reqwest::get(&url).await?.text().await?;
    Ok(json::from_str(&r).unwrap())
}

macro_rules! f {
    ($($arg:expr,)*) => {{
        let mut cap = 0;
        $(
            let arg: &str = $arg.as_ref();
            cap += arg.len();
        )*
        let mut b = String::with_capacity(cap);
        $(
            let arg: &str = $arg.as_ref();
            b.push_str(arg);
        )*
        b
    }};
}

pub async fn translate(word: &str, lang: Lang) -> Result<RespRoot, reqwest::Error> {
    let url = f!(
        "http://api.tureng.com/v1/dictionary/",
        lang.to_str(),
        "/",
        word,
    );
    let r = reqwest::get(&url).await?.text().await?;
    Ok(json::from_str(&r).unwrap())
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
