use nom::{
    IResult, Parser,
    bytes::complete::{tag, take},
    number::complete::{le_u16, le_u32, le_u64, u8},
};

const ELF_MAGIC: &[u8] = &[0x7f, b'E', b'L', b'F'];

// Enums for `e_class` field
pub const EI_CLASS_64: u8 = 2; // 64-bit
// Enums for `data` field
pub const EI_DATA_2LSB: u8 = 1; // Little Endian
// Enums for `e_type` field
pub const ET_REL: u16 = 1; // Relocatable file
pub const ET_EXEC: u16 = 2; // Executable file
// Enums for `e_machine` field
pub const EM_AARCH64: u16 = 183; // AArch64 architecture

#[derive(Clone)]
pub struct ElfHeader {
    // --- e_ident [16] ---
    pub class: u8, // file class (32-bit or 64-bit)
    pub data: u8,  // Data Encoding
    pub version: u8,
    os_abi: u8,
    abi_version: u8,
    // --- other header fields ---
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    // Gives the virtual address to which the system first transfers control, thus starting the process. 
    pub e_entry: u64,
    // holds the program header table's file offset in bytes.
    pub e_phoff: u64,
    pub e_shoff: u64, // section header table offset
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    // holds the number of entries in the program header table.
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,    // section header table entry count
    pub e_shstrndx: u16, // index of the section storing all section names
}

pub fn parse_elf_header(input: &[u8]) -> IResult<&[u8], ElfHeader> {
    let (
        input,
        (
            _, // magic number
            class,
            data,
            version,
            os_abi,
            abi_version,
            _padding,
            e_type,
            e_machine,
            e_version,
            e_entry,
            e_phoff,
            e_shoff,
            e_flags,
            e_ehsize,
            e_phentsize,
            e_phnum,
            e_shentsize,
            e_shnum,
            e_shstrndx,
        ),
    ) = (
        tag(ELF_MAGIC),
        u8,        // e_ident[EI_CLASS]
        u8,        // e_ident[EI_DATA]
        u8,        // e_ident[EI_VERSION]
        u8,        // e_ident[EI_OSABI]
        u8,        // e_ident[EI_ABIVERSION]
        take(7u8), // padding
        le_u16,    // e_type
        le_u16,    // e_machine
        le_u32,    // e_version
        le_u64,    // e_entry
        le_u64,    // e_phoff
        le_u64,    // e_shoff
        le_u32,    // e_flags
        le_u16,    // e_ehsize
        le_u16,    // e_phentsize
        le_u16,    // e_phnum
        le_u16,    // e_shentsize
        le_u16,    // e_shnum
        le_u16,    // e_shstrndx
    )
        .parse(input)?;

    let elf_header = ElfHeader {
        class,
        data,
        version,
        os_abi,
        abi_version,
        e_type,
        e_machine,
        e_version,
        e_entry,
        e_phoff,
        e_shoff,
        e_flags,
        e_ehsize,
        e_phentsize,
        e_phnum,
        e_shentsize,
        e_shnum,
        e_shstrndx,
    };

    Ok((input, elf_header))
}
