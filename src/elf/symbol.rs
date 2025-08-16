use nom::{
    IResult, Parser,
    number::complete::{le_u16, le_u32, le_u64, u8},
};

use crate::elf::section::SectionHeader;

///An object file's symbol table holds information needed to locate and relocate a program's symbolic definitions and references.
pub struct Symbol {
    ///holds an index into the object file's symbol string table
    pub name_offset: u32,
    /// specifies the symbol's type and binding attributes
    pub info: u8,
    pub other: u8,
    pub shndx: u16,
    pub value: u64,
    pub size: u64,
}

impl Symbol {
    /// Get the binding info of the symbol, eg: LOCAL, GLOBAL, WEAK
    pub fn get_bind(&self) -> u8 {
        self.info >> 4
    }
    /// Get the type info of the symbol. eg: NOTYPE, OBJECT, FUNC
    pub fn get_type(&self) -> u8 {
        self.info & 0x0F
    }
}

pub fn parse_symbol(input: &[u8]) -> IResult<&[u8], Symbol> {
    let (input, (name_offset, info, other, shndx, value, size)) =
        (le_u32, u8, u8, le_u16, le_u64, le_u64).parse(input)?;

    Ok((
        input,
        Symbol {
            name_offset,
            info,
            other,
            shndx,
            value,
            size,
        },
    ))
}

pub fn parse_symbol_table<'a>(
    file: &'a [u8],
    symtab_header: &SectionHeader,
) -> IResult<&'a [u8], Vec<Symbol>> {
    if symtab_header.entsize == 0 || symtab_header.size % symtab_header.entsize != 0 {
        return Err(nom::Err::Error(nom::error::Error::new(
            file,
            nom::error::ErrorKind::Verify,
        )));
    }

    let num_symbols = (symtab_header.size / symtab_header.entsize) as usize;
    let table_data = &file[symtab_header.offset as usize..];

    nom::multi::count(parse_symbol, num_symbols).parse(table_data)
}

pub fn get_symbol_name<'a>(strtab_data: &'a [u8], symbol: &Symbol) -> Option<&'a str> {
    let start = symbol.name_offset as usize;

    if start >= strtab_data.len() {
        return None;
    }

    std::ffi::CStr::from_bytes_until_nul(&strtab_data[start..])
        .ok()
        .and_then(|cstr| cstr.to_str().ok())
}
