pub mod header;
pub mod relocation;
pub mod section;
pub mod symbol;

mod test {
    use std::path::Path;

    use crate::elf::{
        header::{EI_CLASS_64, EI_DATA_2LSB, EM_AARCH64, ET_REL, parse_elf_header},
        relocation::parse_rela_table,
        section::{SHT_RELA, get_section_name, parse_section_header_table},
        symbol::{get_symbol_name, parse_symbol_table},
    };

    #[test]
    fn test_parse_elf_file() {
        // 1. Parse the ELF Header
        let test_elf_path = Path::new("materials/hello.o");
        let elf_data = std::fs::read(test_elf_path).expect("Failed to read ELF file");
        let (_, elf_header) = parse_elf_header(&elf_data).expect("Failed to parse ELF header");
        // (1). Test parse header works
        assert_eq!(elf_header.class, EI_CLASS_64); // 64-bit
        assert_eq!(elf_header.data, EI_DATA_2LSB); // Little-Endian
        assert_eq!(elf_header.e_type, ET_REL); // Relocation file
        assert_eq!(elf_header.e_machine, EM_AARCH64); // AArch64 architecture
        // 2. Parse the Section Header Table
        let (_, section_headers) = parse_section_header_table(&elf_data, &elf_header)
            .expect("Failed to parse section header table");

        // 3. Get the section header string table data (.shstrtab)
        let shstrndx = elf_header.e_shstrndx as usize;
        let shstrtab_header = &section_headers[shstrndx];
        let start = shstrtab_header.offset as usize;
        let end = start + shstrtab_header.size as usize;
        let shstrtab_data = &elf_data[start..end];

        // 4. Get section names
        let mut names = vec![];
        for (i, header) in section_headers.iter().enumerate() {
            let section_name = get_section_name(shstrtab_data, header).unwrap_or("Null");
            println!(
                "[{:>2}] {:<20} {:<15x} {:<10x} {:<10x}",
                i, section_name, header.sh_type, header.offset, header.size
            );
            names.push(section_name.to_string());
        }

        // (2). Test section header names
        assert_eq!(section_headers.len(), elf_header.e_shnum as usize);
        assert_eq!(names[1], ".text");
        assert_eq!(names[2], ".rela.text");
        assert_eq!(names[3], ".data");
        assert_eq!(names[4], ".bss");
        assert_eq!(names[10], ".symtab");
        assert_eq!(names[11], ".strtab");
        assert_eq!(names[12], ".shstrtab");

        // 5. Get the .symtab section
        let symtab_header = &section_headers
            .iter()
            .find(|h| get_section_name(shstrtab_data, h) == Some(".symtab"))
            .expect("No .symtab section found");

        let strtab_header_index = symtab_header.link as usize;
        let strtab_header = &section_headers[strtab_header_index];
        let strtab_data_start = strtab_header.offset as usize;
        let strtab_data_end = strtab_data_start + strtab_header.size as usize;
        let strtab_data = &elf_data[strtab_data_start..strtab_data_end];

        // 6. Parse the symbol table
        let (_, symbols) =
            parse_symbol_table(&elf_data, symtab_header).expect("Failed to parse symbol table");

        for (i, symbol) in symbols.iter().enumerate() {
            let symbol_name = get_symbol_name(strtab_data, symbol).unwrap_or("Unknown");
            let bind_str = match symbol.get_bind() {
                0 => "LOCAL",
                1 => "GLOBAL",
                2 => "WEAK",
                _ => "UNKNOWN",
            };
            let type_str = match symbol.get_type() {
                0 => "NOTYPE",
                1 => "OBJECT",
                2 => "FUNC",
                3 => "SECTION",
                4 => "FILE",
                _ => "UNKNOWN",
            };
            println!(
                "[{:>2}] {:<20} {:<10x} {:<10x} {:<10x} {:<4} {:<4}",
                i, symbol_name, symbol.value, symbol.size, symbol.shndx, bind_str, type_str
            );
        }

        // 8. Print relocation sections
        println!("\n--- Relocation Sections ---");
        for section_header in section_headers.iter().filter(|h| h.sh_type == SHT_RELA) {
            let section_name = get_section_name(shstrtab_data, section_header).unwrap_or("N/A");
            println!(
                "\nRelocation section '{}' at offset {:#x}:",
                section_name, section_header.offset
            );

            let (_, relocations) = parse_rela_table(&elf_data, section_header)
                .expect("Failed to parse the relocation table");

            println!("Relocation entry num {}", relocations.len());

            println!(
                "{:<16} {:<24} {:<10} {:<10}",
                "Offset", "Symbol", "Type", "Addend"
            );

            for rela in relocations {
                let symbol_index = rela.get_symbol_index() as usize;
                let symbol = &symbols[symbol_index];
                let symbol_name = get_symbol_name(strtab_data, symbol).unwrap_or("N/A");

                let rela_type = rela.get_type();

                println!(
                    "{:<16x} {:<24} {:<10} {:<10x}",
                    rela.offset, symbol_name, rela_type, rela.addend
                );
            }
        }
    }
}
