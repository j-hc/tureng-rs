use std::fmt::Display;

pub struct Colored<D> {
    d: D,
    code: &'static str,
}

impl<D: Display> Display for Colored<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.code)?;
        self.d.fmt(f)?;
        f.write_str("\x1b[0m")?;
        Ok(())
    }
}

pub trait ToColored: Display + Sized {
    fn with_red(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: "\u{1b}[38;5;1m",
        }
    }

    fn with_magenta(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: "\u{1b}[38;5;5m",
        }
    }

    fn with_green(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: "\u{1b}[38;5;2m",
        }
    }

    fn with_yellow(&self) -> Colored<&Self> {
        Colored {
            d: self,
            code: "\u{1b}[38;5;3m",
        }
    }
}

impl<D: Display> ToColored for D {}
