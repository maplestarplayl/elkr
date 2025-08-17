use std::{env, fs};

use elkr::linker::LinkerContext;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <output_file> <file1.o> <file2.o> ...", args[0]);
        panic!("Not enough arguments provided");
    }
    let output_path = &args[1];
    let contents: Vec<_> = args[2..]
        .iter()
        .map(|path| fs::read(path).unwrap())
        .collect();

    let mut linker = LinkerContext::default();

    println!("--- 0. Loading input files ---");
    for (i, path) in args[2..].iter().enumerate() {
        linker.add_file(path.clone(), &contents[i]);
    }

    println!("--- 1. Laying out and merging sections ---");
    linker.layout_and_merge_sections();

    println!("--- 2. Resolving symbols ---");
    linker.resolve_symbols();

    println!("--- 3. Applying relocations ---");
    linker.apply_relocations();

    println!("--- 4. Writing executable file to '{}' ---", output_path);
    linker
        .write_executable(output_path)
        .expect("Failed to write executable");

    println!("--- Linking finished successfully! ---");
}
