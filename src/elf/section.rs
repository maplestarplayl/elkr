use nom::{
    IResult, Parser,
    number::complete::{le_u32, le_u64},
};

use crate::elf::header::ElfHeader;



pub const SHT_RELA: u32 = 4;
pub struct SectionHeader {
    name_offset: u32, // an index into the section header string table section
    pub sh_type: u32,
    flags: u64,
    addr: u64,
    pub offset: u64,
    /// section's size in bytes.
    pub size: u64,
    /// holds a section header table index link, whose interpretation depends on the section type
    pub link: u32,
    /// holds extra information, whose interpretation depends on the section type.
    pub info: u32,
    addralign: u64,
    /// entry size of a section containing fixed-sized entries
    pub entsize: u64,
}

fn parse_section_header(input: &[u8]) -> IResult<&[u8], SectionHeader> {
    let (input, (name_offset, sh_type, flags, addr, offset, size, link, info, addralign, entsize)) =
        (
            le_u32, // name offset
            le_u32, // section type
            le_u64, // flags
            le_u64, // address
            le_u64, // offset
            le_u64, // size
            le_u32, // link
            le_u32, // info
            le_u64, // address alignment
            le_u64, // entry size
        )
            .parse(input)?;

    let section_header = SectionHeader {
        name_offset,
        sh_type,
        flags,
        addr,
        offset,
        size,
        link,
        info,
        addralign,
        entsize,
    };

    Ok((input, section_header))
}

pub fn parse_section_header_table<'a>(
    file: &'a [u8],
    elf_header: &ElfHeader,
) -> IResult<&'a [u8], Vec<SectionHeader>> {
    let offset = elf_header.e_shoff as usize;
    let num_headers = elf_header.e_shnum as usize;

    let table_input = &file[offset..];

    nom::multi::count(parse_section_header, num_headers).parse(table_input)
}

pub fn get_section_name<'a>(
    sshstrtab_data /*section cthat store the names of all sections */: &'a [u8],
    section_header: &SectionHeader,
) -> Option<&'a str> {
    let name_offset = section_header.name_offset as usize;
    if name_offset >= sshstrtab_data.len() {
        return None; // Invalid offset
    }

    std::ffi::CStr::from_bytes_until_nul(&sshstrtab_data[name_offset..])
        .ok()
        .and_then(|cstr| cstr.to_str().ok())
}
