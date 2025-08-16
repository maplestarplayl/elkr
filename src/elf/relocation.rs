use nom::{number::complete::{le_i64, le_u64}, IResult, Parser};

use crate::elf::section::SectionHeader;



/// Since we adopt the `ELF64` specification
/// We use `Rela` instead of `Rel`
pub struct Rela {
    pub offset: u64,
    pub info: u64,
    pub addend: i64, 
}

impl Rela {
    pub fn get_symbol_index(&self) -> u32 {
        (self.info >> 32) as u32
    }

    pub fn get_type(&self) -> u32 {
        (self.info & 0xFFFFFFFF) as u32
    }
}

pub fn parse_rela_entry(input: &[u8]) -> IResult<&[u8], Rela> {
    let (input, (offset, info, addend)) = (le_u64, le_u64, le_i64).parse(input)?;

    Ok((
        input,
        Rela {
            offset,
            info,
            addend,
        },
    ))
}

pub fn parse_rela_table<'a>(
    file: &'a [u8],
    rela_header: &SectionHeader,
) -> IResult<&'a [u8], Vec<Rela>> {
    if rela_header.entsize == 0  || rela_header.size % rela_header.entsize != 0 {
        return Err(nom::Err::Error(nom::error::Error::new(
            file,
            nom::error::ErrorKind::Verify,
        )));
    }

    let num_entries = (rela_header.size / rela_header.entsize) as usize;
    let table_data = &file[rela_header.offset as usize..];

    Ok(nom::multi::count(parse_rela_entry, num_entries).parse(&table_data)?)
}

