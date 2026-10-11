use std::{collections::HashMap, num::NonZero, path::PathBuf, time::Duration};

use mlua::{Lua, LuaOptions, StdLib};

use crate::{
    Error,
    error::ErrorInner,
    player::AudioStream,
    source::enumerate_modules,
};

#[derive(Debug)]
pub struct Runtime {
    lua: Lua,
    source_dir: PathBuf,
}

impl Runtime {
    pub fn new(source_dir: impl Into<PathBuf>) -> Result<Self, Error> {
        let lua_stdlib = StdLib::JIT
            | StdLib::TABLE
            | StdLib::STRING
            | StdLib::MATH
            | StdLib::PACKAGE
            | StdLib::BIT;
        let lua = Lua::new_with(lua_stdlib, LuaOptions::new())
            .map_err(ErrorInner::InitLua)?;

        Ok(Self { lua, source_dir: source_dir.into() })
    }

    pub fn preload(&mut self) -> Result<(), Error> {
        let mut modules = HashMap::new();
        let mut module_names = Vec::new();
        for result in enumerate_modules(self.source_dir.clone()) {
            let module = result?;
            module_names.push(module.name.clone());
            modules.insert(module.name.clone(), module);
        }
        let function = move |lua: &Lua,
                             (name, _): (Box<str>, ())|
              -> mlua::Result<mlua::Value> {
            let Some(module) = modules.get(&name[..]) else {
                Err(mlua::Error::RuntimeError(format!(
                    "Module name {name:?} not found"
                )))?
            };
            lua.load(module).eval()
        };
        let lua_function = self
            .lua
            .create_function(function)
            .map_err(ErrorInner::PreloadLua)?;
        for module_name in module_names {
            self.lua
                .preload_module(&module_name, lua_function.clone())
                .map_err(ErrorInner::PreloadLua)?;
        }
        Ok(())
    }

    pub fn load_project(&self) -> Result<Project, Error> {
        let require: mlua::Function = self
            .lua
            .globals()
            .get("require")
            .map_err(ErrorInner::GetLuaRequire)?;
        let main: mlua::Table =
            require.call(("main",)).map_err(ErrorInner::RequireLuaMain)?;

        let at: mlua::Function =
            main.get("at").map_err(ErrorInner::GetLuaAt)?;

        let duration: f64 =
            main.get("duration").map_err(ErrorInner::GetLuaDuration)?;

        let channels: Option<u16> =
            main.get("channels").map_err(ErrorInner::GetLuaChannels)?;

        let sample_rate: Option<u32> =
            main.get("channels").map_err(ErrorInner::GetLuaSampleRate)?;

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
