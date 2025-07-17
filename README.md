# ofc

Ordered File Container (`.ofc`) is a simple archive file format which stores an
ordered list of files.

## Usage

```
USAGE:
    ofc <subcommand> <args>

SUBCOMMANDS:
    create    Create an `.ofc` file containing the files in the given directory
    read      Output the contents of a single file within a given `.ofc` to stdout
    info      List the positions (length and offset) of each file within the given `.ofc`
```

## Building

ofc can be built from source by cloning this repository and using Cargo.

```shell
git clone https://github.com/rsookram/ofc
cd ofc
cargo build --release
```
