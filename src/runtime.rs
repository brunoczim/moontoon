use std::{num::NonZero, path::PathBuf, sync::Arc, time::Duration};

use mlua::{Lua, LuaOptions, StdLib};

use crate::{Error, error::ErrorInner, package::Package, player::AudioStream};

#[derive(Debug)]
pub struct Runtime {
    lua: Lua,
    package: Arc<Package>,
}

impl Runtime {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, Error> {
        let root_dir = root_dir.into();

        let lua_stdlib = StdLib::JIT
            | StdLib::TABLE
            | StdLib::STRING
            | StdLib::MATH
            | StdLib::PACKAGE
            | StdLib::BIT;
        let lua = Lua::new_with(lua_stdlib, LuaOptions::new())
            .map_err(ErrorInner::InitLua)?;

        let package = Arc::new(Package::load(root_dir.clone())?);

        let module_names = package.modules.keys().cloned().collect::<Vec<_>>();

        let function = {
            let package = package.clone();

            move |lua: &Lua,
                  (name,): (Box<str>,)|
                  -> mlua::Result<mlua::Value> {
                let Some(module) = package.modules.get(&name[..]) else {
                    Err(mlua::Error::RuntimeError(format!(
                        "Module name {name:?} not found"
                    )))?
                };
                lua.load(module).eval()
            }
        };
        let lua_function =
            lua.create_function(function).map_err(ErrorInner::PreloadLua)?;
        for modname in module_names {
            lua.preload_module(modname.content(), lua_function.clone())
                .map_err(ErrorInner::PreloadLua)?;
        }

        Ok(Self { lua, package })
    }

    pub fn load_project(&self) -> Result<Project, Error> {
        let modname = self.package.config.name.content();

        let require: mlua::Function = self
            .lua
            .globals()
            .get("require")
            .map_err(ErrorInner::GetLuaRequire)?;
        let main: mlua::Table =
            require.call((modname,)).map_err(ErrorInner::RequireLuaMain)?;

        let at: mlua::Function =
            main.get("at").map_err(ErrorInner::GetLuaAt)?;

        let duration: f64 =
            main.get("duration").map_err(ErrorInner::GetLuaDuration)?;

        let channels: Option<u16> =
            main.get("channels").map_err(ErrorInner::GetLuaChannels)?;

        let sample_rate: Option<u32> =
            main.get("sample_rate").map_err(ErrorInner::GetLuaSampleRate)?;

        let channels = NonZero::new(channels.unwrap_or(1))
            .ok_or(ErrorInner::InvalidLuaChannels)?;

        let sample_rate = NonZero::new(sample_rate.unwrap_or(44100))
            .ok_or(ErrorInner::InvalidLuaSampleRate)?;

        let duration = Duration::from_secs_f64(duration);

        Ok(Project { at, duration, channels, sample_rate })
    }
}

#[derive(Debug)]
pub struct Project {
    at: mlua::Function,
    duration: Duration,
    channels: NonZero<u16>,
    sample_rate: NonZero<u32>,
}

impl AudioStream for Project {
    fn at(&self, seconds: f64, channel: u16) -> Result<f64, Error> {
        let sample =
            self.at.call((seconds, channel)).map_err(ErrorInner::CallLuaAt)?;
        Ok(sample)
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    fn channels(&self) -> NonZero<u16> {
        self.channels
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.sample_rate
    }
}
