# Amenhotep

This ancient Egyptian scrib will help you scaffold all required code and files to initiate a [Checkpoint](https://docs.checkpoint.fyi/) indexer !


## Installation

```
git clone git@github.com:carbonable-labs/amenhotep.git ~/.local/amenhotep
cd ~/.local/amenhotep
cargo build --release
```
Update your path to add **amenhotep** exec to bin path
```
echo 'export PATH=~/.local/amenhotep/target/release:$PATH' >> .bashrc
```

## Commands

```
amenhotep dry-run ../cairo_contracts  Console print generated files
amenhotep generate ../cairo_contracts Generate whole bunch of files required to run an indexer
```


