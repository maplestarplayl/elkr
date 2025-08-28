


# ELKR: A simple ELF linker in Rust
This is a simple elf linker written in Rust. I start this project to better learn the internals of program linking.
This linker aims to provide a basic implementation of an ELF linker, capable of handling symbol resolution and relocation,
and finally generates an executable ELF file.

## References
- [OS Dev Wiki](https://wiki.osdev.org/System_V_ABI)
- [System V Application Binary Interface](https://www.sco.com/developers/gabi/latest/contents.html)

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

### Core Components

The linker consists of three main components:

- **InputFile**: Represents a parsed object file containing sections, symbols, and relocations
- **LinkerContext**: Manages global symbol resolution and output section merging
- **OutputSection**: Accumulates merged section data with proper memory layout

### Linking Process

1. **Object File Parsing**: Uses the `object` crate to parse ELF object files and extract sections, symbols, and relocations

2. **Symbol Resolution**: Builds a global symbol table, detecting undefined references and multiple definitions

3. **Section Layout**: Merges input sections (`.text`, `.rodata`, `.data`) into contiguous output sections starting at virtual address `0x400000`

4. **Relocation Processing**: Applies PC-relative and absolute relocations by patching instruction bytes in the output sections

5. **ELF Generation**: Writes a complete executable ELF file with proper headers, section table, and program headers

## File Structure

```
elkr/
├── src/
│   ├── lib.rs              # Crate API: exports LinkerContext and elf module for external use
│   ├── main.rs             # CLI entry: parses args, reads .o files, drives LinkerContext pipeline
│   ├── linker.rs           # Core linker: InputFile, OutputSection, LinkerContext; layout/merge/relocate/write
│   └── elf/
│       ├── mod.rs          # Module glue: pub use of header/section/symbol/relocation for crate::elf::*
│       ├── header.rs       # ELF header model and parser: ElfHeader, ET_EXEC, parse_elf_header
│       ├── section.rs      # Section headers: SectionHeader, SHT_* consts, parse_section_header_table, get_section_name
│       ├── symbol.rs       # Symbols: Symbol model, parse_symbol_table, get_symbol_name
│       └── relocation.rs   # Relocations (RELA): types/constants (AArch64), parse_rela_table, helpers (get_type, get_symbol_index)
├── materials/
│   ├── main.c          # Example C source with main() function
│   ├── sum.c           # Example C source with sum() function  
│   └── start.c         # Example C source with _start() entry point
├── Cargo.toml          # Rust project configuration and dependencies
├── .github/
│   └── copilot-instructions.md  # AI coding assistant guidelines
└── README.md           # Project documentation
```

**Key File Purposes:**
- `src/linker.rs` - Symbol resolution, section merging, relocation processing
- `src/main.rs` - Command-line interface demonstrating LinkerContext usage
- `materials/*.c` - Test cases for generating object files with gcc
- `Cargo.toml` - Specifies dependencies: `object` crate for ELF parsing, `byteorder` for binary output




