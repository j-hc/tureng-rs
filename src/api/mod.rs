pub mod resp;

use miniserde::json;
pub use resp::*;
use std::sync::OnceLock;

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
    url: &str,
    headers: &[(&'static str, &'static str)],
) -> Result<String, reqwest::Error> {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    let client = CLIENT.get_or_init(|| {
        reqwest::ClientBuilder::new()
            .gzip(true)
            .connection_verbose(true)
            .http1_title_case_headers()
            .http1_ignore_invalid_headers_in_responses(true)
            .build()
            .expect("reqwest client")
    });
    let mut req = client.get(url);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let r = req.send().await?.text().await?;
    Ok(r)
}

pub async fn tureng_ac(word: &str, lang: Lang) -> Result<Vec<String>, reqwest::Error> {
    let url = f!("https://ac.tureng.co/?t=", &word, "&l=", lang.to_str());
    let headers = [("accept-encoding", "gzip"), ("user-agent", "okhttp/4.11.0")];
    let r = req(&url, &headers).await?;
    match json::from_str::<Vec<String>>(&r) {
        Ok(json) => Ok(json),
        Err(_) => panic!("tureng invalid response: '{r}' to '{url}'"),
    }
}

pub async fn translate(word: &str, lang: Lang) -> Result<RespRoot, reqwest::Error> {
    // TODO: make this https. for some reason, it doesnt work with https
    let url = f!(
        "http://api.tureng.com/v1/dictionary/",
        lang.to_str(),
        "/",
        word,
    );
    let headers = [
        ("Accept", "application/json"),
        ("User-Agent", "Dalvik/1.0.0 (Linux)"),
        ("Host", "api.tureng.com"),
        ("Connection", "Keep-Alive"),
        ("Accept-Encoding", "gzip"),
    ];
    let r = req(&url, &headers).await?;
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
