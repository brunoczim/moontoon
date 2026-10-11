use std::{
    borrow::Cow,
    fs::{self, ReadDir},
    io,
    mem,
    path::{Path, PathBuf},
};

use mlua::chunk::AsChunk;

use crate::{Error, error::ErrorInner};

pub const MAX_READ_DIR_CONSEC_FAILURES: u32 = 16;

fn parse_module_segment(path: &Path) -> Result<&str, Error> {
    let Some(stem) = path.file_stem() else {
        Err(ErrorInner::InvalidModuleName { path: path.to_owned() })?
    };
    let Some(as_str) = stem.to_str() else {
        Err(ErrorInner::InvalidModuleName { path: path.to_owned() })?
    };
    if !as_str.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_') {
        Err(ErrorInner::InvalidModuleName { path: path.to_owned() })?
    }
    if !as_str.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        Err(ErrorInner::InvalidModuleName { path: path.to_owned() })?
    }
    Ok(as_str)
}

pub fn enumerate_modules(base_dir: PathBuf) -> ModuleEnumerator {
    ModuleEnumerator::new(base_dir)
}

#[derive(Debug, Clone, PartialEq)]
enum Op {
    LeaveValid,
    LeaveInvalid,
    Visit(PathBuf),
}

#[derive(Debug)]
struct IteratingDirData {
    reader: Option<ReadDir>,
    path: PathBuf,
    consec_failures: u32,
}

#[derive(Debug)]
enum State {
    Ended,
    ToOperate,
    ReadingDir(IteratingDirData),
}

impl State {
    pub fn ended(&self) -> bool {
        matches!(self, Self::Ended)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub path: PathBuf,
    pub name: String,
    pub content: Vec<u8>,
}

impl AsChunk for &'_ Module {
    fn name(&self) -> Option<String> {
        Some(self.name.clone())
    }

    fn source<'a>(&self) -> io::Result<Cow<'a, [u8]>>
    where
        Self: 'a,
    {
        Ok(self.content.clone().into())
    }
}

#[derive(Debug)]
pub struct ModuleEnumerator {
    module_prefix: Vec<String>,
    invalid_count: u64,
    ops: Vec<Op>,
    state: State,
}

impl ModuleEnumerator {
    fn new(base_dir: PathBuf) -> Self {
        Self {
            module_prefix: Vec::new(),
            invalid_count: 0,
            ops: vec![Op::Visit(base_dir)],
            state: State::ToOperate,
        }
    }

    fn transition(
        &mut self,
        state: State,
    ) -> (Option<Result<Module, Error>>, State) {
        match state {
            State::Ended => (None, State::Ended),

            State::ToOperate => match self.ops.pop() {
                None => (None, State::Ended),
                Some(Op::LeaveValid) => {
                    self.module_prefix.pop();
                    (None, State::ToOperate)
                },
                Some(Op::LeaveInvalid) => {
                    self.invalid_count -= 1;
                    (None, State::ToOperate)
                },
                Some(Op::Visit(path)) => {
                    let metadata = match fs::metadata(&path) {
                        Ok(x) => x,
                        Err(source) => {
                            let error = ErrorInner::ReadLua {
                                source,
                                path: path.clone(),
                            };
                            let error = error.into();
                            return (Some(Err(error)), State::ToOperate);
                        },
                    };

                    if metadata.is_dir() {
                        if path.extension().is_none()
                            && let Ok(segment) = parse_module_segment(&path)
                        {
                            self.module_prefix.push(segment.to_owned());
                        } else {
                            self.invalid_count += 1;
                        }
                        let reader = match fs::read_dir(&path) {
                            Ok(x) => x,
                            Err(source) => {
                                let error = ErrorInner::ReadLua {
                                    source,
                                    path: path.clone(),
                                };
                                let error = error.into();
                                return (Some(Err(error)), State::ToOperate);
                            },
                        };
                        let data = IteratingDirData {
                            reader: Some(reader),
                            path,
                            consec_failures: 0,
                        };
                        (None, State::ReadingDir(data))
                    } else {
                        if path.extension().is_none_or(|ext| ext != "lua") {
                            return (None, State::ToOperate);
                        }

                        if self.invalid_count > 0 {
                            let error = ErrorInner::InvalidModuleName {
                                path: path.to_owned(),
                            };
                            let error = error.into();
                            return (Some(Err(error)), State::ToOperate);
                        }

                        let module_segment = match parse_module_segment(&path) {
                            Ok(x) => x,
                            Err(e) => return (Some(Err(e)), State::ToOperate),
                        };

                        let content = match fs::read(&path) {
                            Ok(x) => x,
                            Err(source) => {
                                let error = ErrorInner::ReadLua {
                                    source,
                                    path: path.clone(),
                                };
                                let error = error.into();
                                return (Some(Err(error)), State::ToOperate);
                            },
                        };

                        let name;

                        if module_segment == "init"
                            && self.module_prefix.len() > 0
                        {
                            name = self.module_prefix.join(".");
                        } else {
                            self.module_prefix.push(module_segment.to_owned());
                            name = self.module_prefix.join(".");
                            self.module_prefix.pop();
                        }

                        let module = Module { path, content, name };

                        (Some(Ok(module)), State::ToOperate)
                    }
                },
            },

            State::ReadingDir(mut data) => {
                if data.consec_failures > MAX_READ_DIR_CONSEC_FAILURES {
                    let error = ErrorInner::TooManyDirErrors {
                        path: data.path.clone(),
                    };
                    let error = error.into();
                    data.reader = None;
                    data.consec_failures = 0;
                    return (Some(Err(error)), State::ReadingDir(data));
                }

                match data.reader.as_mut().and_then(|reader| reader.next()) {
                    Some(Ok(entry)) => {
                        data.consec_failures = 0;
                        self.ops.push(Op::Visit(entry.path()));
                        (None, State::ReadingDir(data))
                    },
                    Some(Err(source)) => {
                        data.consec_failures += 1;
                        let error = ErrorInner::ReadLua {
                            source,
                            path: data.path.clone(),
                        };
                        let error = error.into();
                        (Some(Err(error)), State::ReadingDir(data))
                    },
                    None => {
                        self.ops.push(if data.path.extension().is_some() {
                            Op::LeaveInvalid
                        } else {
                            Op::LeaveValid
                        });
                        (None, State::ToOperate)
                    },
                }
            },
        }
    }
}

impl Iterator for ModuleEnumerator {
    type Item = Result<Module, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let state = mem::replace(&mut self.state, State::Ended);
            if state.ended() {
                break None;
            }

            let (output, next) = self.transition(state);
            self.state = next;

            if let Some(output) = output {
                break Some(output);
            }
        }
    }
}
