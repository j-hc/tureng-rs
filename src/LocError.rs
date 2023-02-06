use std::error::Error;
use std::panic::Location;

#[derive(Debug)]
pub struct LocError<E> {
    err: E,
    location: &'static Location<'static>,
}

pub type LocResult<T, E> = Result<T, LocError<E>>;

impl<E: Error + 'static> Error for LocError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.err)
    }
}
impl<E: Error> Display for LocError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}, {}", self.err, self.location)
    }
}

impl<E: Error> From<E> for LocError<E> {
    #[track_caller]
    fn from(err: E) -> Self {
        Self {
            err,
            location: Location::caller(),
        }
    }
}
pub trait ToLocError<T, E> {
    fn to_loc(self) -> Result<T, LocError<E>>
    where
        Self: Sized;
}

impl<T: Debug, E: Error> ToLocError<T, E> for Result<T, E> {
    #[track_caller]
    fn to_loc(self) -> Result<T, LocError<E>> {
        self.map_err(|err| LocError {
            err,
            location: Location::caller(),
        })
    }
}
