use crate::generator::GeneratedFile;
use std::fs::File;
use std::io::Write;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WriterError {
    #[error("failed to create file")]
    FailedToCreateFile,
    #[error("failed to write content to file")]
    FailedToWriteContent,
}

pub trait Writer {
    fn write(&self, file: &GeneratedFile) -> Result<(), WriterError>;
}

pub struct FileWriter {}

impl Writer for FileWriter {
    fn write(&self, file: &GeneratedFile) -> Result<(), WriterError> {
        let mut fs_file = File::create(&file.name).map_err(|_| WriterError::FailedToCreateFile)?;
        fs_file
            .write_all(file.content.as_bytes())
            .map_err(|_| WriterError::FailedToWriteContent)?;
        Ok(())
    }
}

pub struct ConsoleWriter {}

impl Writer for ConsoleWriter {
    fn write(&self, file: &GeneratedFile) -> Result<(), WriterError> {
        println!("{}", file.name);
        println!("{}", file.content);
        Ok(())
    }
}
