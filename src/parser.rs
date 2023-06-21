use anyhow::Result;
use regex::Regex;
use std::io::BufRead;
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("file must have a .cairo extension")]
    InvalidFileExtension,
}

pub(crate) fn files_to_parse<P: AsRef<Path> + std::convert::AsRef<std::ffi::OsStr>>(
    file: P,
    mut files: Vec<PathBuf>,
) -> Result<Vec<PathBuf>> {
    if is_directory(&file)? {
        for entry in std::fs::read_dir(&file)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                files = files_to_parse(path, files)?;
                continue;
            }

            files = files_to_parse(path, files)?;
        }
    }

    if let Ok(_) = is_cairo_file(&file) {
        files.push((&file).into());
    }

    Ok(files)
}

pub(crate) fn parse_cairo_file<P: AsRef<Path> + std::convert::AsRef<std::ffi::OsStr>>(
    file: P,
) -> Result<FileDomain> {
    let mut file_domain = FileDomain::new(&file);
    if let Ok(mut lines) = read_lines(file) {
        let mut line_nr = 0;
        while let Some(line) = lines.next() {
            if let Ok(l) = line {
                line_nr += 1;

                let is_event_line = l.contains("#[event]");
                if is_event_line {
                    if let Some(Ok(event_line)) = lines.next() {
                        // increment there too as going to the next line
                        line_nr += 1;
                        let mut cairo_event: CairoEvent = event_line.into();
                        // not parsing emitted_at yet cause code comprehension is not impl yet.
                        // this might be enough for poc
                        cairo_event.definined_at(line_nr);
                        file_domain.add_cairo_event(cairo_event);
                    }
                }
            }
        }
    }

    Ok(file_domain)
}

#[derive(Debug)]
pub(crate) struct FileDomain {
    pub(crate) name: String,
    pub(crate) events: Vec<CairoEvent>,
}
impl FileDomain {
    fn new<P: AsRef<Path> + std::convert::AsRef<std::ffi::OsStr>>(file: P) -> Self {
        let path: String = <P as AsRef<Path>>::as_ref(&file)
            .to_str()
            .unwrap()
            .to_string();

        let name = path
            .split("/")
            .last()
            .unwrap()
            .to_string()
            .replace(".cairo", "");
        let name = capitalize(&name);
        Self {
            name,
            events: vec![],
        }
    }
    fn add_cairo_event(&mut self, event: CairoEvent) {
        self.events.push(event);
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[derive(Debug, Default)]
pub struct CairoEvent {
    name: String,
    arguments: Vec<CairoArgument>,
    definition_at: usize,
    emitted_at: Vec<usize>,
}
impl CairoEvent {
    pub fn definined_at(&mut self, line: usize) {
        self.definition_at = line;
    }

    pub fn to_js_function_string(&self) -> String {
        format!("handle{}", self.name)
    }

    pub fn to_js_function_name_string(&self) -> String {
        format!("new_{}", self.name.to_lowercase())
    }

    pub fn to_js_function(&self) -> String {
        format!(
            r#"
export async function handle{}({{ block, tx, event, mysql }}: Parameters<CheckpointWriter>[0]) {{
    if (!event) return;

    new Error('Not implemented yet !');
}}
            "#,
            self.name
        )
    }
}

/// As everything would be felt252 or u256 only string is truly required
#[derive(Debug)]
pub(crate) enum PostgresType {
    String,
    Int,
    Float,
    Bool,
}
impl From<&str> for PostgresType {
    fn from(value: &str) -> Self {
        match value {
            "ContractAddress" => Self::String,
            "u256" => Self::String,
            _ => Self::String,
        }
    }
}
impl ToString for PostgresType {
    fn to_string(&self) -> String {
        match self {
            Self::String => "String!".to_string(),
            Self::Int => "Int!".to_string(),
            Self::Float => "Float!".to_string(),
            Self::Bool => "Boolean!".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct CairoArgument {
    name: String,
    r#type: PostgresType,
}

impl CairoArgument {
    pub fn js_function_name(&self) -> String {
        format!("handle{}", self.name)
    }
}

impl From<&str> for CairoArgument {
    fn from(value: &str) -> Self {
        let v: Vec<&str> = value.split(": ").collect();
        Self {
            name: v[0].to_string(),
            r#type: v[1].into(),
        }
    }
}

impl ToString for &CairoArgument {
    fn to_string(&self) -> String {
        format!("{}: {:?},", self.name, self.r#type)
    }
}

impl From<String> for CairoEvent {
    fn from(value: String) -> Self {
        let expr = Regex::new(r"(fn )(?<fn_name>[a-zA-z]+)\((?<args>.*)\)").unwrap();
        let captures = expr.captures(&value).unwrap();
        let name = captures.name("fn_name").unwrap().as_str();
        let args = captures
            .name("args")
            .unwrap()
            .as_str()
            .split(", ")
            .map(|s| s.into())
            .collect::<Vec<CairoArgument>>();

        Self {
            name: name.to_string(),
            arguments: args,
            ..Default::default()
        }
    }
}

impl ToString for &CairoEvent {
    fn to_string(&self) -> String {
        self.arguments
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join("\n\t")
    }
}

fn read_lines<P: AsRef<Path>>(filename: P) -> Result<std::io::Lines<std::io::BufReader<File>>> {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn is_directory<P: AsRef<Path>>(file: P) -> Result<bool> {
    Ok(File::open(file)?.metadata()?.is_dir())
}

fn is_cairo_file<P: AsRef<Path> + std::convert::AsRef<std::ffi::OsStr>>(file: P) -> Result<bool> {
    let path = Path::new(&file);
    if let Some(extension) = path.extension() {
        if extension == "cairo" {
            return Ok(true);
        }
    }

    Err(ParserError::InvalidFileExtension.into())
}
