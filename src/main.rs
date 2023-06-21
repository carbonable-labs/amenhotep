use anyhow::Result;
use std::path::PathBuf;
use thiserror::Error;
use writer::{ConsoleWriter, FileWriter, Writer};

use crate::{
    generator::generate_indexer,
    parser::{files_to_parse, parse_cairo_file, FileDomain},
};

mod generator;
mod parser;
mod writer;

fn main() -> Result<()> {
    let cmd = clap::Command::new("amenhotep")
        .bin_name("amenhotep")
        .subcommand_required(true)
        .subcommand(
            clap::Command::new("dry-run")
                .about("Check the expected output")
                .arg(
                    clap::arg!(<PATH> ... "The repository to parse out.")
                        .value_parser(clap::value_parser!(PathBuf)),
                )
                .arg_required_else_help(true),
        )
        .subcommand(
            clap::Command::new("generate")
                .about("Generate the files output")
                .arg(
                    clap::arg!(<PATH> ... "The repository to parse out.")
                        .value_parser(clap::value_parser!(PathBuf)),
                )
                .arg_required_else_help(true),
        );

    let matches = cmd.get_matches();

    match matches.subcommand() {
        Some(("dry-run", matches)) => {
            let paths = matches
                .get_many::<std::path::PathBuf>("PATH")
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            handle_generate_indexer(paths, ConsoleWriter {})?
        }
        Some(("generate", matches)) => {
            let paths = matches
                .get_many::<std::path::PathBuf>("PATH")
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            handle_generate_indexer(paths, FileWriter {})?
        }
        _ => unreachable!("clap should ensure we don't get here"),
    }

    Ok(())
}

fn handle_generate_indexer(paths: Vec<&PathBuf>, writer: impl Writer) -> Result<()> {
    let mut files = Vec::new();
    for path in paths {
        let mut to_parse = files_to_parse(path, files.clone())?;
        files.append(&mut to_parse);
    }

    println!("Files to parse : {:#?}", files);

    let mut file_domains: Vec<FileDomain> = Vec::new();
    for file in files {
        let events = parse_cairo_file(file)?;
        file_domains.push(events);
    }

    let files = generate_indexer(&file_domains)?;

    for file in files {
        writer.write(&file)?;
    }

    Ok(())
}
