


# ELKR: A simple ELF linker in Rust
This is a simple elf linker written in Rust. I start this project to better learn the internals of program linking.
This linker aims to provide a basic implementation of an ELF linker, capable of handling symbol resolution and relocation,
and finally generates an executable ELF file.

## Features
- Basic ELF parsing
- Symbol resolution
- Relocation handling
- Support for multiple input files


## Drawbacks (now)
- Only support ELF64 on AArch64
- Only support static linking
- Unable to directly execute plain c files (need `_start` fn)


## Usage
```
$ gcc -c main.c -o main.o
$ gcc -c sum.c -o sum.o
$ gcc -c start.c -o start.o
$ cargo run -- output materials/start.o materials/main.o materials/sum.o
$ chmod +x output
$ ./output; echo "Exit code: $?"
```

## Implementation Details
TODO