use std::{
    borrow::{Borrow, Cow},
    fmt,
    fs::{self, ReadDir},
    io,
    iter,
    mem,
    path::{Path, PathBuf},
    str::FromStr,
};

use mlua::chunk::AsChunk;
use serde::{Deserialize, Serialize};

use crate::{Error, error::ErrorInner};

pub const MAX_READ_DIR_CONSEC_FAILURES: u32 = 16;

pub fn enumerate_modules(
    base_dir: PathBuf,
    base_mod: &ModName,
) -> ModuleEnumerator {
    ModuleEnumerator::new(base_dir, Some(base_mod))
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
)]
#[serde(try_from = "&str", into = "&str")]
pub struct ModNameSegmentRef<'a> {
    content: &'a str,
}

impl<'a> ModNameSegmentRef<'a> {
    pub fn content(self) -> &'a str {
        self.content
    }

    pub fn to_owned(self) -> ModNameSegment {
        ModNameSegment { content: self.content.into() }
    }
}

impl fmt::Display for ModNameSegmentRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content())
    }
}

impl AsRef<str> for ModNameSegmentRef<'_> {
    fn as_ref(&self) -> &str {
        self.content()
    }
}

impl Borrow<str> for ModNameSegmentRef<'_> {
    fn borrow(&self) -> &str {
        self.content()
    }
}

impl PartialEq<str> for ModNameSegmentRef<'_> {
    fn eq(&self, other: &str) -> bool {
        self.content() == other
    }
}

impl PartialEq<ModNameSegmentRef<'_>> for str {
    fn eq(&self, other: &ModNameSegmentRef<'_>) -> bool {
        self == other.content()
    }
}

impl<'a> TryFrom<&'a str> for ModNameSegmentRef<'a> {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        if !value.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
        {
            Err(ErrorInner::InvalidModNameSegStr { input: value.to_owned() })?
        }
        if !value.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
            Err(ErrorInner::InvalidModNameSegStr { input: value.to_owned() })?
        }
        Ok(Self { content: value })
    }
}

impl<'a> TryFrom<&'a Path> for ModNameSegmentRef<'a> {
    type Error = Error;

    fn try_from(value: &'a Path) -> Result<Self, Self::Error> {
        let Some(stem) = value.file_stem() else {
            Err(ErrorInner::InvalidModNameSegPath { path: value.to_owned() })?
        };
        let Some(as_str) = stem.to_str() else {
            Err(ErrorInner::InvalidModNameSegPath { path: value.to_owned() })?
        };
        Self::try_from(as_str).map_err(|error| {
            error.replacing(ErrorInner::InvalidModNameSegPath {
                path: value.to_owned(),
            })
        })
    }
}

impl<'a> TryFrom<&'a PathBuf> for ModNameSegmentRef<'a> {
    type Error = Error;

    fn try_from(value: &'a PathBuf) -> Result<Self, Self::Error> {
        Self::try_from(&**value)
    }
}

impl<'a> TryFrom<&'a String> for ModNameSegmentRef<'a> {
    type Error = Error;

    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        Self::try_from(&**value)
    }
}

impl<'a> From<ModNameSegmentRef<'a>> for &'a str {
    fn from(value: ModNameSegmentRef<'a>) -> Self {
        value.content()
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(try_from = "&str", into = "String")]
pub struct ModNameSegment {
    content: Box<str>,
}

impl<'a> ModNameSegment {
    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn as_ref(&self) -> ModNameSegmentRef<'_> {
        ModNameSegmentRef { content: self.content() }
    }
}

impl fmt::Display for ModNameSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content())
    }
}

impl AsRef<Self> for ModNameSegment {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<str> for ModNameSegment {
    fn as_ref(&self) -> &str {
        self.content()
    }
}

impl Borrow<str> for ModNameSegment {
    fn borrow(&self) -> &str {
        self.content()
    }
}

impl PartialEq<str> for ModNameSegment {
    fn eq(&self, other: &str) -> bool {
        self.content() == other
    }
}

impl PartialEq<ModNameSegment> for str {
    fn eq(&self, other: &ModNameSegment) -> bool {
        self == other.content()
    }
}

impl<'a> TryFrom<&'a str> for ModNameSegment {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        ModNameSegmentRef::try_from(value).map(|r| r.to_owned())
    }
}

impl<'a> TryFrom<&'a Path> for ModNameSegment {
    type Error = Error;

    fn try_from(value: &'a Path) -> Result<Self, Self::Error> {
        ModNameSegmentRef::try_from(value).map(|r| r.to_owned())
    }
}

impl<'a> TryFrom<&'a PathBuf> for ModNameSegment {
    type Error = Error;

    fn try_from(value: &'a PathBuf) -> Result<Self, Self::Error> {
        Self::try_from(&**value)
    }
}

impl<'a> TryFrom<&'a String> for ModNameSegment {
    type Error = Error;

    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        Self::try_from(&**value)
    }
}

impl From<ModNameSegment> for String {
    fn from(value: ModNameSegment) -> Self {
        value.content.into_string()
    }
}

impl FromStr for ModNameSegment {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(try_from = "&str", into = "String")]
pub struct ModName {
    content: Box<str>,
}

impl ModName {
    pub fn from_refs_iter<'a>(
        iterable: impl IntoIterator<Item = ModNameSegmentRef<'a>>,
    ) -> Result<Self, Error> {
        let mut content = String::new();
        for (i, segment) in iterable.into_iter().enumerate() {
            if i > 0 {
                content.push('.');
            }
            content.push_str(segment.content());
        }
        Self::try_from(content)
    }

    pub fn from_iter(
        iterable: impl IntoIterator<Item = impl AsRef<ModNameSegment>>,
    ) -> Result<Self, Error> {
        let mut content = String::new();
        for (i, segment) in iterable.into_iter().enumerate() {
            if i > 0 {
                content.push('.');
            }
            content.push_str(segment.as_ref().content());
        }
        Self::try_from(content)
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn segments<'a>(
        &'a self,
    ) -> impl Iterator<Item = ModNameSegmentRef<'a>> + Send + Sync + 'a {
        self.content.split('.').map(|content| ModNameSegmentRef { content })
    }
}

impl fmt::Display for ModName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content())
    }
}

impl AsRef<str> for ModName {
    fn as_ref(&self) -> &str {
        self.content()
    }
}

impl Borrow<str> for ModName {
    fn borrow(&self) -> &str {
        self.content()
    }
}

impl PartialEq<str> for ModName {
    fn eq(&self, other: &str) -> bool {
        self.content() == other
    }
}

impl PartialEq<ModName> for str {
    fn eq(&self, other: &ModName) -> bool {
        self == other.content()
    }
}

impl<'a> TryFrom<&'a str> for ModName {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_owned())
    }
}

impl<'a> TryFrom<&'a Path> for ModName {
    type Error = Error;

    fn try_from(value: &'a Path) -> Result<Self, Self::Error> {
        let Some(stem) = value.file_stem() else {
            Err(ErrorInner::InvalidModNamePath { path: value.to_owned() })?
        };
        let Some(as_str) = stem.to_str() else {
            Err(ErrorInner::InvalidModNamePath { path: value.to_owned() })?
        };
        Self::try_from(as_str).map_err(|error| {
            error.replacing(ErrorInner::InvalidModNamePath {
                path: value.to_owned(),
            })
        })
    }
}

impl<'a> TryFrom<&'a PathBuf> for ModName {
    type Error = Error;

    fn try_from(value: &'a PathBuf) -> Result<Self, Self::Error> {
        Self::try_from(&**value)
    }
}

impl<'a> TryFrom<&'a String> for ModName {
    type Error = Error;

    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        Self::try_from(&**value)
    }
}

impl<'a> TryFrom<String> for ModName {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value
            .split('.')
            .map(ModNameSegmentRef::try_from)
            .map(|result| result.map(|_| ()))
            .collect::<Result<(), Error>>()?;
        Ok(Self { content: value.into_boxed_str() })
    }
}

impl From<ModName> for String {
    fn from(value: ModName) -> Self {
        value.content.into_string()
    }
}

impl FromStr for ModName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
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
    pub name: ModName,
    pub content: Vec<u8>,
}

impl AsChunk for &'_ Module {
    fn name(&self) -> Option<String> {
        Some(self.name.content().to_owned())
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
    module_prefix: Vec<ModNameSegment>,
    invalid_count: u64,
    ops: Vec<Op>,
    state: State,
}

impl ModuleEnumerator {
    fn new(base_dir: PathBuf, base_mod: Option<&ModName>) -> Self {
        Self {
            module_prefix: base_mod.map_or_else(Vec::new, |modname| {
                modname.segments().map(ModNameSegmentRef::to_owned).collect()
            }),
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
                            && let Ok(seg) = ModNameSegmentRef::try_from(&path)
                        {
                            self.module_prefix.push(seg.to_owned());
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
                            let error = ErrorInner::InvalidModNamePath {
                                path: path.to_owned(),
                            };
                            let error = error.into();
                            return (Some(Err(error)), State::ToOperate);
                        }

                        let module_segment =
                            match ModNameSegmentRef::try_from(&path) {
                                Ok(x) => x,
                                Err(e) => {
                                    return (Some(Err(e)), State::ToOperate);
                                },
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

                        let name = if &module_segment == "init"
                            && self.module_prefix.len() > 0
                        {
                            let refs = self
                                .module_prefix
                                .iter()
                                .map(ModNameSegment::as_ref);
                            match ModName::from_refs_iter(refs) {
                                Ok(x) => x,
                                Err(e) => {
                                    return (Some(Err(e)), State::ToOperate);
                                },
                            }
                        } else {
                            let refs = self
                                .module_prefix
                                .iter()
                                .map(ModNameSegment::as_ref)
                                .chain(iter::once(module_segment));
                            match ModName::from_refs_iter(refs) {
                                Ok(x) => x,
                                Err(e) => {
                                    return (Some(Err(e)), State::ToOperate);
                                },
                            }
                        };

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
