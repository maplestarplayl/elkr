use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
};

const PT_LOAD: u32 = 1;
const PF_R: u32 = 4;
const PF_W: u32 = 2;
const PF_X: u32 = 1;

use crate::elf::{
    header::{ET_EXEC, ElfHeader, parse_elf_header},
    relocation::{R_AARCH64_CALL26, R_AARCH64_PREL32, parse_rela_table},
    section::{
        SHT_NOBITS, SHT_PROGBITS, SHT_RELA, SHT_SYMTAB, SectionHeader, get_section_name,
        parse_section_header_table,
    },
    symbol::{Symbol, get_symbol_name, parse_symbol_table},
};

pub struct InputFile<'a> {
    filename: String,
    content: &'a [u8],
    header: ElfHeader,
    sections: Vec<SectionHeader>,
    symbols: Vec<Symbol>,
    shstrtab_data: &'a [u8],
    strtab_data: &'a [u8],
}

/// Represents a merged section
pub struct OutputSection {
    name: String,
    header: SectionHeader,
    data: Vec<u8>,
}

pub struct GlobalSymbol<'a> {
    _name: &'a str,
    final_addr: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct ProgramHeader {
    pub p_type: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}
pub struct LinkerContext<'a> {
    input_files: Vec<InputFile<'a>>,
    output_sections: HashMap<String, OutputSection>,
    global_symbols: HashMap<&'a str, GlobalSymbol<'a>>,
    current_addr: u64, // an address counter for allocating addresses
    // Track where each input file's section starts within the output section
    input_section_offsets: HashMap<(usize, usize), u64>, // (file_index, section_index) -> offset_in_output_section
}
impl<'a> Default for LinkerContext<'a> {
    fn default() -> Self {
        Self {
            input_files: Default::default(),
            output_sections: Default::default(),
            global_symbols: Default::default(),
            current_addr: 0x400_000,
            input_section_offsets: Default::default(),
        }
    }
}
impl<'a> LinkerContext<'a> {
    pub fn add_file(&mut self, filename: String, content: &'a [u8]) {
        let (_, header) = parse_elf_header(content).unwrap();
        let (_, sections) = parse_section_header_table(content, &header).unwrap();

        let shstrtab_h = &sections[header.e_shstrndx as usize];
        let shstrtab_data =
            &content[shstrtab_h.offset as usize..(shstrtab_h.offset + shstrtab_h.size) as usize];

        let symtab_h = sections.iter().find(|h| h.sh_type == SHT_SYMTAB).unwrap();
        let strtab_h = &sections[symtab_h.link as usize];
        let strtab_data =
            &content[strtab_h.offset as usize..(strtab_h.offset + strtab_h.size) as usize];

        let (_, symbols) = parse_symbol_table(content, symtab_h).unwrap();

        self.input_files.push(InputFile {
            filename,
            content,
            header,
            sections,
            symbols,
            shstrtab_data,
            strtab_data,
        });
    }

    pub fn layout_and_merge_sections(&mut self) {
        // 1. Calculate sizes and create output sections
        for file in &self.input_files {
            for section in file.sections.iter() {
                if section.sh_type == SHT_PROGBITS || section.sh_type == SHT_NOBITS {
                    let name = get_section_name(file.shstrtab_data, section)
                        .unwrap_or("")
                        .to_string();
                    if name.is_empty() {
                        continue;
                    }
                    if name.starts_with(".rel") {
                        panic!("Shouldn't happen this");
                    }

                    // Only include allocatable sections (with SHF_ALLOC flag)
                    const SHF_ALLOC: u64 = 0x2;
                    if (section.flags & SHF_ALLOC) == 0 {
                        continue; // Skip non-allocatable sections like .comment, .note.GNU-stack
                    }

                    let entry = self.output_sections.entry(name.clone()).or_insert_with(|| {
                        let mut new_header = section.clone();
                        new_header.size = 0;
                        OutputSection {
                            name,
                            header: new_header,
                            data: Vec::new(),
                        }
                    });
                    entry.header.size += section.size;
                }
            }
        }

        // 2. Assign address and allocate data buffers
        // Calculate header sizes to know where sections should start in virtual memory
        let elf_header_size = 64u64;
        let program_header_size = 56u64;
        let num_program_headers = 2u64;
        let headers_total_size = elf_header_size + (num_program_headers * program_header_size);

        // Sections should start after the headers in virtual memory
        self.current_addr += headers_total_size;

        // Sort sections in a logical order: .text, .rodata, .data, .bss
        let mut sorted_sections: Vec<_> = self.output_sections.values_mut().collect();
        sorted_sections.sort_by_key(|s| {
            match s.name.as_str() {
                ".text" => 0,
                ".rodata" => 1,
                ".data" => 2,
                ".bss" => 3,
                _ => 4, // Everything else after
            }
        });

        for section in sorted_sections {
            let align = section.header.addralign as usize;
            if align > 0 {
                self.current_addr = (self.current_addr + align as u64 - 1) & !(align as u64 - 1); // Check
            }
            section.header.addr = self.current_addr;
            section.data.resize(section.header.size as usize, 0);
            self.current_addr += section.header.size;
        }

        // 3. Copy data from input files to output sections
        let mut current_offsets: HashMap<String, u64> = HashMap::new(); // Global across all files
        for (file_idx, file) in self.input_files.iter().enumerate() {
            println!("Processing file {} for data copying", file.filename);
            for (section_idx, section) in file.sections.iter().enumerate() {
                if section.sh_type == SHT_PROGBITS {
                    let name = get_section_name(file.shstrtab_data, section)
                        .unwrap_or("")
                        .to_string();
                    if let Some(output_section) = self.output_sections.get_mut(&name) {
                        let current_offset = current_offsets.entry(name.clone()).or_insert(0);

                        println!(
                            "  Section {} (idx {}) -> output section {} at offset 0x{:x}",
                            name, section_idx, name, *current_offset
                        );

                        // Record where this input section starts in the output section
                        self.input_section_offsets
                            .insert((file_idx, section_idx), *current_offset);

                        let start = *current_offset as usize;
                        let end = start + section.size as usize;
                        let data = &file.content
                            [section.offset as usize..(section.offset + section.size) as usize];
                        output_section.data[start..end].copy_from_slice(data);
                        *current_offset += section.size;
                    }
                }
            }
        }
    }

    pub fn resolve_symbols(&mut self) {
        println!("=== Symbol Resolution ===");
        for (file_idx, file) in self.input_files.iter().enumerate() {
            println!("Processing file: {}", file.filename);
            for symbol in &file.symbols {
                if symbol.get_bind() == 1 {
                    // GLOBAL SYMBOL

                    let name = get_symbol_name(file.strtab_data, symbol).unwrap_or("");
                    println!(
                        "  Symbol: {} (value: 0x{:x}, shndx: {})",
                        name, symbol.value, symbol.shndx
                    );
                    if name.is_empty() || self.global_symbols.contains_key(name) {
                        continue;
                    }
                    if symbol.shndx > 0 && (symbol.shndx as usize) < file.sections.len() {
                        let section_of_symbol = &file.sections[symbol.shndx as usize];
                        let section_name =
                            get_section_name(file.shstrtab_data, section_of_symbol).unwrap();

                        println!("    Section: {}", section_name);

                        if let Some(output_sec) = self.output_sections.get(section_name) {
                            // Get the offset of this input section within the output section
                            let input_section_offset = self
                                .input_section_offsets
                                .get(&(file_idx, symbol.shndx as usize))
                                .unwrap_or(&0);

                            let final_addr =
                                output_sec.header.addr + input_section_offset + symbol.value;
                            println!(
                                "    Final address: 0x{:x} (section base: 0x{:x} + input offset: 0x{:x} + symbol offset: 0x{:x})",
                                final_addr,
                                output_sec.header.addr,
                                input_section_offset,
                                symbol.value
                            );
                            self.global_symbols.insert(
                                name,
                                GlobalSymbol {
                                    _name: name,
                                    final_addr,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn apply_relocations(&mut self) {
        for (file_idx, file) in self.input_files.iter().enumerate() {
            for section in file.sections.iter().filter(|s| s.sh_type == SHT_RELA) {
                let target_sec_idx = section.info as usize;
                println!("the target section index is {target_sec_idx}");
                let target_sec = &file.sections[target_sec_idx];
                let target_sec_name = get_section_name(file.shstrtab_data, target_sec)
                    .unwrap()
                    .to_string();

                if let Some(output_section) = self.output_sections.get_mut(&target_sec_name) {
                    // 传递重定位表section本身，而不是目标section
                    let (_, relocations) = parse_rela_table(file.content, section).unwrap();

                    for rela in relocations {
                        let sym_index = rela.get_symbol_index() as usize;
                        let symbol = &file.symbols[sym_index];
                        let sym_name = get_symbol_name(file.strtab_data, symbol).unwrap();

                        println!(
                            "  Relocation: {} type {} offset 0x{:x} addend {}",
                            sym_name,
                            rela.get_type(),
                            rela.offset,
                            rela.addend
                        );

                        if let Some(global_sym) = self.global_symbols.get(sym_name) {
                            let s = global_sym.final_addr;

                            // P is the address of the place being relocated
                            // Need to account for where this input section is within the output section
                            let input_section_offset = self
                                .input_section_offsets
                                .get(&(file_idx, target_sec_idx))
                                .unwrap_or(&0);
                            let p = output_section.header.addr + input_section_offset + rela.offset;
                            let a = rela.addend as u64;

                            println!(
                                "    S (symbol addr) = 0x{:x}, P (patch location) = 0x{:x} (section: 0x{:x} + input_offset: 0x{:x} + rela_offset: 0x{:x}), A (addend) = 0x{:x}",
                                s,
                                p,
                                output_section.header.addr,
                                input_section_offset,
                                rela.offset,
                                a
                            );

                            if rela.get_type() == R_AARCH64_CALL26 {
                                let offset = (s + a).wrapping_sub(p);
                                // The immediate is 26 bits, right-shifted by 2
                                let imm26 = (offset as i64 >> 2) & 0x03FFFFFF;

                                println!(
                                    "    CALL26: offset = 0x{:x}, imm26 = 0x{:x}",
                                    offset, imm26
                                );

                                // Read the original instruction - need to account for input section offset
                                let reloc_offset_in_buffer =
                                    (input_section_offset + rela.offset) as usize;
                                let mut instruction = u32::from_le_bytes(
                                    output_section.data
                                        [reloc_offset_in_buffer..reloc_offset_in_buffer + 4]
                                        .try_into()
                                        .unwrap(),
                                );
                                println!("    Original instruction: 0x{:x}", instruction);
                                // Clear the immediate field and patch in the new value
                                instruction &= 0xFC000000;
                                instruction |= imm26 as u32;
                                println!("    Patched instruction: 0x{:x}", instruction);

                                // Write the patched instruction back
                                output_section.data
                                    [reloc_offset_in_buffer..reloc_offset_in_buffer + 4]
                                    .copy_from_slice(&instruction.to_le_bytes());
                            } else if rela.get_type() == R_AARCH64_PREL32 {
                                // PC-relative 32-bit: S + A - P
                                let value = (s + a).wrapping_sub(p) as u32;

                                println!("    PREL32: value = 0x{:x}", value);

                                // Write the 32-bit value directly - need to account for input section offset
                                let reloc_offset_in_buffer =
                                    (input_section_offset + rela.offset) as usize;
                                output_section.data
                                    [reloc_offset_in_buffer..reloc_offset_in_buffer + 4]
                                    .copy_from_slice(&value.to_le_bytes());
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn write_executable(&self, path: &str) -> io::Result<()> {
        let mut file = fs::File::create(path)?;

        // Debug: Print global symbols
        println!("Global symbols:");
        for (name, sym) in &self.global_symbols {
            println!("  {} -> 0x{:x}", name, sym.final_addr);
        }

        let entry_point = self
            .global_symbols
            .get("_start")
            .or_else(|| self.global_symbols.get("main"))
            .unwrap()
            .final_addr;
        println!("Entry point: 0x{:x}", entry_point);

        let base_addr = 0x400_000;
        let page_size = 0x1000;

        // === Step 1. Assign sections to segments ===
        let mut code_sections = Vec::new();
        let mut data_sections = Vec::new();
        let mut sorted_sections: Vec<_> = self.output_sections.values().collect();
        sorted_sections.sort_by_key(|s| s.header.addr);

        println!("Output sections:");
        for sec in &sorted_sections {
            println!(
                "  {} @ 0x{:x}, size: 0x{:x}, flags: 0x{:x}",
                sec.name, sec.header.addr, sec.header.size, sec.header.flags
            );
        }

        for sec in sorted_sections {
            // SHF_EXECINSTR flag is 0x4
            if sec.header.flags & 0x4 != 0 {
                code_sections.push(sec);
            } else {
                data_sections.push(sec);
            }
        }

        // === Step 2. Calculate layout ===
        let elf_header_size = 64u64;
        let program_header_size = 56u64;
        let num_program_headers = 2u64;
        let headers_total_size = elf_header_size + (num_program_headers * program_header_size);

        println!("Layout calculations:");
        println!("  Headers total size: 0x{:x}", headers_total_size);

        // Code Segment Layout
        let code_segment_start_vaddr = base_addr;
        let code_segment_file_offset = 0u64;
        let code_segment_filesz =
            headers_total_size + code_sections.iter().map(|s| s.header.size).sum::<u64>();
        let code_segment_memsz = code_segment_filesz;

        let code_segment_file_offset_aligned = align_up(code_segment_file_offset, page_size);
        let code_segment_start_vaddr_aligned = align_up(code_segment_start_vaddr, page_size);

        println!(
            "  Code segment file offset: 0x{:x} -> aligned: 0x{:x}",
            code_segment_file_offset, code_segment_file_offset_aligned
        );
        println!(
            "  Code segment vaddr: 0x{:x} -> aligned: 0x{:x}",
            code_segment_start_vaddr, code_segment_start_vaddr_aligned
        );
        println!("  Code segment file size: 0x{:x}", code_segment_filesz);

        // Data Segment Layout
        let data_segment_start_vaddr =
            align_up(code_segment_start_vaddr + code_segment_memsz, page_size);
        let data_segment_file_offset = align_up(code_segment_filesz, page_size);
        let data_segment_filesz = data_sections
            .iter()
            .filter(|s| s.header.sh_type != SHT_NOBITS)
            .map(|s| s.header.size)
            .sum::<u64>();
        let data_segment_memsz = data_sections.iter().map(|s| s.header.size).sum::<u64>();

        // === Step 3. Create Program Headers ===
        let code_header = ProgramHeader {
            p_type: PT_LOAD,
            flags: PF_R | PF_X,
            offset: code_segment_file_offset_aligned, // Code segment starts from the beginning of the file   TODO:check
            vaddr: code_segment_start_vaddr_aligned,
            paddr: code_segment_start_vaddr_aligned,
            filesz: code_segment_filesz,
            memsz: code_segment_memsz,
            align: page_size,
        };

        let data_header = ProgramHeader {
            p_type: PT_LOAD,
            flags: PF_R | PF_W,
            offset: data_segment_file_offset,
            vaddr: data_segment_start_vaddr,
            paddr: data_segment_start_vaddr,
            filesz: data_segment_filesz,
            memsz: data_segment_memsz,
            align: page_size,
        };

        // === Step 4. Create ELF Header ===
        let mut header = self.input_files[0].header.clone();
        header.e_type = ET_EXEC;
        header.e_entry = entry_point;
        header.e_phoff = elf_header_size;
        header.e_phnum = num_program_headers as u16;
        header.e_phentsize = program_header_size as u16;
        header.e_shoff = 0; // No section headers
        header.e_shnum = 0;
        header.e_shstrndx = 0;

        // === Step 5. Write everything to a buffer ===
        let mut buffer = Vec::new();

        // ELF Header
        buffer.extend_from_slice(&[0x7f, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        buffer.extend_from_slice(&header.e_type.to_le_bytes());
        buffer.extend_from_slice(&header.e_machine.to_le_bytes());
        buffer.extend_from_slice(&header.e_version.to_le_bytes());
        buffer.extend_from_slice(&header.e_entry.to_le_bytes());
        buffer.extend_from_slice(&header.e_phoff.to_le_bytes());
        buffer.extend_from_slice(&header.e_shoff.to_le_bytes());
        buffer.extend_from_slice(&header.e_flags.to_le_bytes());
        buffer.extend_from_slice(&header.e_ehsize.to_le_bytes());
        buffer.extend_from_slice(&header.e_phentsize.to_le_bytes());
        buffer.extend_from_slice(&header.e_phnum.to_le_bytes());
        buffer.extend_from_slice(&header.e_shentsize.to_le_bytes());
        buffer.extend_from_slice(&header.e_shnum.to_le_bytes());
        buffer.extend_from_slice(&header.e_shstrndx.to_le_bytes());

        // Program Headers
        for p_header in &[code_header, data_header] {
            buffer.extend_from_slice(&p_header.p_type.to_le_bytes());
            buffer.extend_from_slice(&p_header.flags.to_le_bytes());
            buffer.extend_from_slice(&p_header.offset.to_le_bytes());
            buffer.extend_from_slice(&p_header.vaddr.to_le_bytes());
            buffer.extend_from_slice(&p_header.paddr.to_le_bytes());
            buffer.extend_from_slice(&p_header.filesz.to_le_bytes());
            buffer.extend_from_slice(&p_header.memsz.to_le_bytes());
            buffer.extend_from_slice(&p_header.align.to_le_bytes());
        }

        let code_padding = code_header.offset.saturating_sub(buffer.len() as u64);
        buffer.extend_from_slice(&vec![0; code_padding as usize]);
        // Code Section Data
        for sec in &code_sections {
            buffer.extend_from_slice(&sec.data);
        }

        // Padding to align data segment
        let padding_size = data_header.offset.saturating_sub(buffer.len() as u64);
        buffer.extend_from_slice(&vec![0; padding_size as usize]);

        // Data Section Data
        for sec in &data_sections {
            if sec.header.sh_type != SHT_NOBITS {
                buffer.extend_from_slice(&sec.data);
            }
        }

        file.write_all(&buffer)?;
        Ok(())
    }
}

fn align_up(addr: u64, page_size: u64) -> u64 {
    (addr + page_size - 1) & !(page_size - 1)
}
