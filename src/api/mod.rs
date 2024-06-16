pub mod resp;

use miniserde::json;
use reqwest::header::{HeaderMap, HeaderValue};
pub use resp::*;

macro_rules! f {
    ($($arg:expr),* $(,)?) => {{
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

async fn req(
    client: &reqwest::Client,
    url: &str,
    headers: &[(&'static str, &'static str)],
) -> Result<String, reqwest::Error> {
    let mut hm = HeaderMap::with_capacity(headers.len());
    for (k, v) in headers {
        hm.append(*k, HeaderValue::from_static(v));
    }
    let r = client.get(url).headers(hm).send().await?.text().await?;
    Ok(r)
}

pub async fn tureng_ac(
    client: &reqwest::Client,
    word: &str,
    lang: Lang,
) -> Result<Vec<String>, reqwest::Error> {
    let url = f!("https://ac.tureng.co/?t=", &word, "&l=", lang.to_str());
    let headers = [
        ("Accept-Encoding", "gzip"),
        ("Connection", "Keep-Alive"),
        ("Host", "ac.tureng.co"),
        ("User-Agent", "okhttp/4.10.0"),
    ];
    let r = req(client, &url, &headers).await?;
    match json::from_str::<Vec<String>>(&r) {
        Ok(json) => Ok(json),
        Err(_) => panic!("tureng invalid response: '{r}' to '{url}'"),
    }
}

pub async fn translate(
    client: &reqwest::Client,
    word: &str,
    lang: Lang,
) -> Result<RespRoot, reqwest::Error> {
    let url = f!(
        "https://api.tureng.com/v1/dictionary/",
        lang.to_str(),
        "/",
        word,
    );
    let headers = [
        ("Accept", "application/json"),
        ("Accept-Encoding", "gzip"),
        ("Connection", "Keep-Alive"),
        ("Host", "api.tureng.com"),
        ("User-Agent", "okhttp/4.10.0"),
    ];
    let r = req(client, &url, &headers).await?;
    match json::from_str::<RespRoot>(&r) {
        Ok(json) => Ok(json),
        Err(_) => panic!("tureng invalid response: '{r}' to '{url}'"),
    }
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

impl Lang {
    pub fn to_str(self) -> &'static str {
        match self {
            Lang::ENDE => "ende",
            Lang::ENES => "enes",
            Lang::ENFR => "enfr",
            Lang::ENTR => "entr",
        }
    }
    pub fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "ende" => Ok(Self::ENDE),
            "enes" => Ok(Self::ENES),
            "enfr" => Ok(Self::ENFR),
            "entr" => Ok(Self::ENTR),
            _ => Err(()),
        }
    }
}
