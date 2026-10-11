use std::{error::Error as StdError, fmt, io, mem, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct Error {
    inner: Box<ErrorInner>,
}

impl Error {
    pub fn multi(errors: impl IntoIterator<Item = Self>) -> Result<(), Self> {
        let errors = errors.into_iter();
        let (low, _) = errors.size_hint();
        let mut collection = Vec::with_capacity(low);

        for mut error in errors {
            if let ErrorInner::Multi(multi) = &mut *error.inner {
                let mut to_flatten = mem::take(&mut multi.inner).into_vec();
                collection.append(&mut to_flatten);
            } else {
                collection.push(error);
            }
        }

        if collection.is_empty() {
            Ok(())
        } else {
            let multi = ErrorMulti { inner: collection.into() };
            Err(ErrorInner::Multi(multi))?
        }
    }
}

impl<E> From<E> for Error
where
    ErrorInner: From<E>,
{
    fn from(value: E) -> Self {
        Self { inner: Box::new(value.into()) }
    }
}

#[derive(Debug, Error)]
pub enum ErrorInner {
    #[error("failed to initialize Lua")]
    InitLua(#[source] mlua::Error),
    #[error("failed to preload Lua modules")]
    PreloadLua(#[source] mlua::Error),
    #[error("failed to get Lua require function")]
    GetLuaRequire(#[source] mlua::Error),
    #[error("failed to require Lua main")]
    RequireLuaMain(#[source] mlua::Error),
    #[error("failed to get Lua at function")]
    GetLuaAt(#[source] mlua::Error),
    #[error("failed to call Lua at function")]
    CallLuaAt(#[source] mlua::Error),
    #[error("failed to get Lua duration value")]
    GetLuaDuration(#[source] mlua::Error),
    #[error("failed to get Lua channels value")]
    GetLuaChannels(#[source] mlua::Error),
    #[error("invalid Lua channels value")]
    InvalidLuaChannels,
    #[error("failed to get Lua sample rate value")]
    GetLuaSampleRate(#[source] mlua::Error),
    #[error("invalid Lua sample rate")]
    InvalidLuaSampleRate,
    #[error("failed to read lua code in {}", path.display())]
    ReadLua {
        #[source]
        source: io::Error,
        path: PathBuf,
    },
    #[error("invalid module name in path {}", path.display())]
    InvalidModuleName { path: PathBuf },
    #[error(
        "duplicate module name {} found in path {}",
        module_name,
        path.display(),
    )]
    DuplicateModuleName { module_name: String, path: PathBuf },
    #[error(
        "omitting further errors, too many errors reading directory in path {}",
        path.display(),
    )]
    TooManyDirErrors { path: PathBuf },
    #[error("failed to open device sink")]
    OpenDeviceSink(#[from] rodio::DeviceSinkError),
    #[error("multiple errors found")]
    Multi(#[from] ErrorMulti),
}

#[derive(Debug)]
pub struct ErrorMulti {
    inner: Box<[Error]>,
}

impl fmt::Display for ErrorMulti {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for error in &self.inner {
            write!(f, "- {error}")?;
            let mut next = error.source();
            while let Some(curr) = next {
                write!(f, "  Caused by: {curr}")?;
                next = curr.source();
            }
        }
        Ok(())
    }
}

impl StdError for ErrorMulti {}
