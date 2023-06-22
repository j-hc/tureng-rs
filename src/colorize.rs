use std::{fmt::Display, io::IsTerminal, sync::Once};
use termion::color;

static mut IS_ATTY: bool = false;

fn is_atty() -> bool {
    static IS_ATTY_INIT: Once = Once::new();
    unsafe {
        IS_ATTY_INIT.call_once(|| IS_ATTY = std::io::stdout().is_terminal());
        IS_ATTY
    }
}

pub struct Colored<D> {
    d: D,
    code: &'static str,
}

impl<D: Display> Display for Colored<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let isatty = is_atty();
        if isatty {
            f.write_str(self.code)?;
        }
        self.d.fmt(f)?;
        if isatty {
            f.write_str("\x1b[0m")?;
        }
        Ok(())
    }
}

pub trait ToColored: Display + Sized {
    fn red(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: color::Red.fg_str(),
        }
    }

    fn white_bg(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: color::White.bg_str(),
        }
    }

    fn green(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: color::Green.fg_str(),
        }
    }

    fn black(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: color::Black.fg_str(),
        }
    }
    fn yellow(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: color::Yellow.fg_str(),
        }
    }
    fn blue(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: color::Black.fg_str(),
        }
    }
    fn magenta(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: color::Magenta.fg_str(),
        }
    }
    fn cyan(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: color::Cyan.fg_str(),
        }
    }
    fn white(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: color::White.fg_str(),
        }
    }
}

impl<D: Display> ToColored for D {}
