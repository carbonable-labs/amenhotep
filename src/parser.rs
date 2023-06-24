use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::hash;
use std::io::BufRead;
use std::str::Split;
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
#[derive(PartialEq, PartialOrd)]
pub struct Identifier(String);
pub enum CairoType {
    Felt252,
    ContractAddress,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    LegacyMap(Box<CairoType>, Box<CairoType>),
    //    Tuple(Vec<CairoType>),
}

impl ToString for CairoType {
    fn to_string(&self) -> String {
        match self {
            Felt252 => String::from("String!"),
            ContractAddress => String::from("String!"),
            U8 => String::from("Int!"),
            U16 => String::from("Int!"),
            U32 => String::from("Int!"),
            U64 => String::from("String!"),
            U128 => String::from("String!"),
            U256 => String::from("String!"),
            LegacyMap(key, value) => {
                format!("Map!({}, {})", key.to_string(), value.to_string())
            }
        }
    }
}

pub struct CairoStorage {
    fields: HashMap<Identifier, CairoType>,
}

impl From<Vec<String>> for CairoStorage {
    fn from(value: Vec<String>) -> Self {
        let fields_vec: Vec<(Identifier, CairoType)> = value
            .iter()
            .map(|v| v.trim().split(":").into_iter().take(2).collect())
            .map(|v: Vec<&str>| {
                let key = v[0].trim();
                let value = v[1].trim();
                let key = Identifier(key.to_string());
                let value = CairoType::from(value.to_string());
                (key, value)
            })
            .collect();
        let mut fields = HashMap::new();
        fields_vec.iter().for_each(|(k, v)| {
            println!("{}: {}", &k.0, &v.to_string());
            fields.insert(k, v);
        });
        Self {
            fields: HashMap::new(),
        }
    }
}

impl From<String> for CairoType {
    fn from(value: String) -> Self {
        let value = value.trim();
        if value == "felt252" {
            return CairoType::Felt252;
        }

        if value == "ContractAddress" {
            return CairoType::ContractAddress;
        }

        if value == "u8" {
            return CairoType::U8;
        }

        if value == "u16" {
            return CairoType::U16;
        }

        if value == "u32" {
            return CairoType::U32;
        }

        if value == "u64" {
            return CairoType::U64;
        }

        if value == "u128" {
            return CairoType::U128;
        }

        if value == "u256" {
            return CairoType::U256;
        }

        if value.starts_with("LegacyMap") {
            let mut value = value.split("LegacyMap");
            let mut key = value.next().unwrap().trim();
            let mut value = value.next().unwrap().trim();
            key = key.trim_start_matches('<').trim_end_matches('>');
            value = value.trim_start_matches('<').trim_end_matches('>');
            return CairoType::LegacyMap(
                Box::new(CairoType::from(key.to_string())),
                Box::new(CairoType::from(value.to_string())),
            );
        }
        panic!("Unknown type: {}", value);
    }
}

pub(crate) fn parse_cairo_file<P: AsRef<Path> + std::convert::AsRef<std::ffi::OsStr>>(
    file: P,
) -> Result<FileDomain> {
    let mut file_domain = FileDomain::new(&file);
    let mut storage_buf: Vec<String> = Vec::new();
    let mut storage: Option<CairoStorage> = None;
    if let Ok(mut lines) = read_lines(file) {
        let mut line_nr = 0;
        while let Some(line) = lines.next() {
            if let Ok(l) = line {
                line_nr += 1;

                if l.contains("struct Storage") {
                    while storage.is_none() {
                        if let Some(l) = lines.next() {
                            if let Ok(l) = l {
                                line_nr += 1;

                                if l.contains('}') {
                                    storage = Some(storage_buf.clone().into());
                                }

                                storage_buf.push(l.clone());
                            }
                        }
                    }
                }

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
        let expr = Regex::new(r"(fn )(?<fn_name>[a-zA-Z]+)\((?<args>.*)\)").unwrap();
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

#[cfg(test)]
mod tests {
    use super::CairoStorage;

    #[test]
    fn test_parse_cairo_type() {
        let types = [
            "        _name: felt252,".to_owned(),
            "        _symbol: felt252,".to_owned(),
            "        _initial_supply: u256,".to_owned(),
            "        _total_supply: u256,".to_owned(),
            "        _balances: LegacyMap<ContractAddress, u256>,".to_owned(),
            "        _allowances: LegacyMap<(ContractAddress, ContractAddress), u256>,".to_owned(),
            "        _reference: ContractAddress,".to_owned(),
            "        _target: ContractAddress,".to_owned(),
            "        _intrication: u64,".to_owned(),
            "        _intrications: LegacyMap<ContractAddress, u64>,".to_owned(),
        ];

        let cairo_type: CairoStorage = types.to_vec().into();
    }
}
