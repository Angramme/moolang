use std::{error::Error, fs};
use std::fmt;
use std::io::BufRead;
use std::path::{Path, PathBuf, Display};
use itertools::Itertools;
use std::iter::once;
use owo_colors::OwoColorize as _;

use crate::frontend::tokenizer::{slice_into_snippets, Location};

#[derive(Debug)]
pub struct LocalizedError(Box<dyn Error>, Location);
#[derive(Debug)]
pub struct LocalizedSourcedError(Box<dyn Error>, Location, PathBuf);


impl Error for LocalizedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.0)
    }
}

impl Error for LocalizedSourcedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.0)
    }
}

pub trait LocalizableError {
    fn with_location(self, location: Location) -> LocalizedError;
}

impl<E> LocalizableError for E 
where E: Error + 'static {
    fn with_location(self, location: Location) -> LocalizedError {
        LocalizedError::new(Box::new(self), location)
    }
}

impl LocalizedError {
    pub fn new(error: Box<dyn Error + 'static>, location: Location) -> Self 
    {
        Self(error, location)
    }
    pub fn location(&self) -> &Location {
        &self.1
    }
    pub fn with_source<P>(self, source_path: P) -> LocalizedSourcedError 
    where P: AsRef<Path> {
        LocalizedSourcedError(self.0, self.1, source_path.as_ref().to_path_buf())
    }
}

impl LocalizedSourcedError {
    pub fn new<E>(error: Box<E>, location: Location, source_path: PathBuf) -> Self 
    where E: Error + 'static {
        Self(error, location, source_path)
    }
    pub fn source_path(&self) -> &Path {
        &self.2
    }
    pub fn location(&self) -> &Location {
        &self.1
    }
}

impl fmt::Display for LocalizedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Error at [line:{},column:{}]:", self.1.line, self.1.column)?;
        write!(f, "{}", self.0)
    } 
}

impl fmt::Display for LocalizedSourcedError {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result 
    {
        let file = std::fs::File::open(self.source_path());
        if let Err(err) = file {
            write!(f, "{}", self)?;
            return write!(f, "Couldn't show snippet, error opening file: {}", err);
        }
        let file = file.unwrap();
        let reader = std::io::BufReader::new(file);
        let (prev, line, next) = once("".to_string())
            .chain(reader.lines().map(Result::unwrap))
            .chain(once("".to_string()))
            .skip(self.location().line.saturating_sub(1))
            .take(3)
            .collect_tuple()
            .unwrap();
        
        writeln!(f, "{}", self.red())?;
        writeln!(f, "Inside file '{}':", fs::canonicalize(self.source_path()).unwrap().display())?;

        let pad = self.location().line.to_string().len() + 1;
        assert!(pad <= line.len()); // avoid uncontrolled padding

        writeln!(f, "{}─┬{}", "─".repeat(pad), "─".repeat(f.width().unwrap_or(30)))?;
        writeln!(f, "{:pad$} │ {}", self.location().line-1, prev, pad=pad)?;
        writeln!(f, "{:pad$} │", "", pad=pad)?; 

        write!(f, "{:pad$} │ ", self.location().line.red(), pad=pad)?;
        let mut last = line.as_ptr() as usize;
        for (i, tok) in slice_into_snippets(line.as_str()).enumerate() {
            let pad = tok.as_ptr() as usize - last;
            assert!(pad <= line.len()); // avoid uncontrolled padding
            write!(f, "{:pad$}", "", pad=pad)?;
            if i == self.location().column {
                write!(f, "{}", tok.red().bold())?;
            } else {
                write!(f, "{}", tok)?;
            }
            last = tok.as_ptr() as usize + tok.len();
        }

        let snippet = slice_into_snippets(line.as_str()).nth(self.location().column).unwrap();
        let padd = snippet.as_ptr() as usize - line.as_ptr() as usize;
        writeln!(f, "\n{0:pad$} │ {0:padd$}{1}", "", "^".repeat(snippet.len()).red(), pad=pad, padd=padd)?;

        writeln!(f, "{:pad$} │ {}", self.location().line+1, next, pad=pad)?;
        write!(f, "{}─┴{}", "─".repeat(pad), "─".repeat(f.width().unwrap_or(30)))
    }
}


// impl<P> Error for CompileErrorWithSource<P>
// where P: AsRef<std::path::Path> + fmt::Debug {
//     fn source(&self) -> Option<&(dyn Error + 'static)> {
//         Some(&self.error)
//     }
// }