//! # TOMT (TOML Formatter)
//! This is the library component of the `tomt` formatter.
//!
//! It uses `taplo` under the hood.

use std::{
    convert::Infallible,
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};

/// TOMT - TOML formatter
#[derive(Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[non_exhaustive]
pub struct Args {
    /// Use the given `.tomlfmt.toml` for formatting options.
    ///
    /// Disables the hierarchical `.tomlfmt.toml` search.
    #[cfg_attr(feature = "clap", clap(short = 'C', long = "config"))]
    pub config_file: Option<String>,

    /// Checks that the format would not have caused
    /// any changes. Exits zero if it would. Useful for linting.
    #[cfg_attr(feature = "clap", clap(short = 'c', long = "check"))]
    pub check: bool,

    /// The directory or file to format.
    ///
    /// By default, finds a `.tomlfmt.toml` file in current
    /// and ancestor folders and runs against that directory.
    /// If not found, or if `-c` is specified, runs against
    /// current working directory by default.
    pub directory: Option<String>,
}

impl Args {
    fn get_config_path(&self) -> Option<PathBuf> {
        if let Some(config_file) = &self.config_file {
            return Some(PathBuf::from(config_file));
        }

        let mut cur = PathBuf::from(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        loop {
            let candidate = cur.join(".tomlfmt.toml");
            if candidate.is_file() {
                return Some(candidate);
            }
            if !cur.pop() {
                break;
            }
        }

        None
    }
}

fn read_config(config_path: impl AsRef<Path>) -> Config {
    let config_str = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        eprintln!(
            "Failed to read config file {}: {}",
            config_path.as_ref().display(),
            e
        );
        String::new()
    });

    let config = toml::from_str::<Config>(&config_str).unwrap_or_else(|e| {
        eprintln!(
            "warning: failed to parse config file {}: {}",
            config_path.as_ref().display(),
            e
        );
        Config::default()
    });

    config
}

/// Events that occur during formatting.
#[derive(Clone, Debug)]
pub enum FormatEvent {
    /// A file was formatted successfully.
    File(PathBuf),
    /// A file failed to format.
    FileError(PathBuf, String),
    /// The formatter finished processing all files.
    Done { success: bool },
}

struct RunIterator<I: Iterator<Item = PathBuf>> {
    formatter: Formatter,
    files: I,
    success: bool,
    has_finished: bool,
    check: bool,
}

impl<I> Iterator for RunIterator<I>
where
    I: Iterator<Item = PathBuf>,
{
    type Item = FormatEvent;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(file) = self.files.next() {
            let mut f = std::fs::OpenOptions::new()
                .read(true)
                .write(!self.check)
                .open(&file)
                .ok()?;

            if self.check {
                match self.formatter.would_format(&mut f) {
                    Ok(true) => Some(FormatEvent::FileError(file, "changes detected".into())),
                    Ok(false) => Some(FormatEvent::File(file)),
                    Err(err) => Some(FormatEvent::FileError(file, err.to_string())),
                }
            } else {
                match self.formatter.format_in_place(&mut f) {
                    Ok(()) => Some(FormatEvent::File(file)),
                    Err(err) => {
                        self.success = false;
                        Some(FormatEvent::FileError(file, err.to_string()))
                    }
                }
            }
        } else {
            if !self.has_finished {
                self.has_finished = true;
                return Some(FormatEvent::Done {
                    success: self.success,
                });
            }
            None
        }
    }
}

/// Runs the "CLI" of the formatter, as a library function.
pub fn run(args: &Args) -> Result<impl Iterator<Item = FormatEvent>, Box<dyn core::error::Error>> {
    let config_path = args.get_config_path();
    let root_dir = config_path
        .as_ref()
        .and_then(|p| p.parent())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let config = config_path.as_ref().map(read_config).unwrap_or_default();

    let formatter = Formatter::new(config.clone());

    let iter = glob_files(root_dir, &["**/*.toml"])?;

    Ok(RunIterator {
        formatter,
        files: iter,
        success: true,
        has_finished: false,
        check: args.check,
    })
}

pub type Config = taplo::formatter::Options;

#[derive(Default)]
pub struct Formatter {
    options: Config,
}

impl Formatter {
    #[inline]
    #[must_use]
    pub fn new(options: Config) -> Self {
        Self { options }
    }

    /// Formats the given string using the
    /// configured options.
    ///
    /// Skips over syntax errors.
    #[inline]
    pub fn format(&self, contents: &str) -> String {
        taplo::formatter::format(contents, self.options.clone())
    }

    /// Formats a read/write/seek stream in-place.
    pub fn format_in_place<S>(&self, stream: &mut S) -> Result<(), Box<dyn core::error::Error>>
    where
        S: Read + Write + Truncate + Seek,
    {
        let mut s = String::new();
        stream.read_to_string(&mut s)?;
        let formatted = self.format(&s);
        stream.seek(std::io::SeekFrom::Start(0))?;
        stream.truncate()?;
        stream.write_all(formatted.as_bytes())?;
        Ok(())
    }

    /// Checks if the stream _would_ have been formatted if
    /// [`Formatter::format_in_place`] were called.
    pub fn would_format<S>(&self, stream: &mut S) -> Result<bool, Box<dyn core::error::Error>>
    where
        S: Read + Write + Truncate + Seek,
    {
        let mut s = String::new();
        stream.read_to_string(&mut s)?;
        let formatted = self.format(&s);
        Ok(formatted != s)
    }
}

/// Truncates the given stream or format destination.
pub trait Truncate {
    type Error: core::error::Error + 'static;

    fn truncate(&mut self) -> Result<(), Self::Error>;
}

impl Truncate for std::fs::File {
    type Error = std::io::Error;

    fn truncate(&mut self) -> Result<(), Self::Error> {
        self.set_len(0)
    }
}

impl Truncate for String {
    type Error = Infallible;

    fn truncate(&mut self) -> Result<(), Self::Error> {
        self.truncate(0);
        Ok(())
    }
}

impl Truncate for Vec<u8> {
    type Error = Infallible;

    fn truncate(&mut self) -> Result<(), Self::Error> {
        self.truncate(0);
        Ok(())
    }
}

impl<T> Truncate for std::io::Cursor<T>
where
    T: Truncate,
{
    type Error = <T as Truncate>::Error;

    fn truncate(&mut self) -> Result<(), Self::Error> {
        self.set_position(0);
        self.get_mut().truncate()
    }
}

fn glob_files(
    root_dir: PathBuf,
    globs: &[&str],
) -> Result<impl Iterator<Item = std::path::PathBuf>, Box<dyn std::error::Error>> {
    let root_dir = format!("{}/", root_dir.to_string_lossy());

    Ok(ignore::Walk::new(&root_dir).filter_map(move |entry| {
        if entry.as_ref().is_ok_and(|e| {
            e.file_type().is_some_and(|ft| ft.is_file())
                && !e.path_is_symlink()
                && globs.iter().any(|glob| {
                    let pth = e.path().to_string_lossy().to_string();
                    fast_glob::glob_match(glob, pth.strip_prefix(&root_dir).unwrap_or(&pth))
                })
        }) {
            let Ok(pth) = entry.map(|e| e.into_path()) else {
                return None;
            };
            Some(pth)
        } else {
            None
        }
    }))
}
