use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct Error {
    inner: Box<ErrorInner>,
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
        "omitting further errors, too many errors reading directory in path {}",
        path.display(),
    )]
    TooManyDirErrors { path: PathBuf },
    #[error("failed to open device sink")]
    OpenDeviceSink(#[from] rodio::DeviceSinkError),
}
