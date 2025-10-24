use std::env;
use std::fs;
use std::io::Read;
use std::io::Seek;
use std::io::Write;

#[repr(C)]
pub struct Superblock {
    pub total_size: u32,
    pub block_size: u32,
    pub total_blocks: u32,
    pub total_inodes: u32,
    pub used_blocks: u32,
    pub used_inodes: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Inode {
    pub used: u8,
    pub name: [u8; 32],
    pub size: u32,
    pub start_block: u32,
    pub block_count: u32,
}

pub struct Bitmap {
    pub bits: Vec<u8>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args[1].as_str();
    match cmd {
        "ls" => {
            ls();
        }
        "create" => {
            if args.len() < 4 {
                println!("Usage: create <filename> <content>");
                return;
            }
            let filename = &args[2];
            let content = &args[3];
            create(filename, content);
        }
        "cat" => {
            if args.len() < 3 {
                println!("Usage: cat <filename>");
                return;
            }
            let filename = &args[2];
            cat(filename);
        }
        "format" => {
            format();
            // Aquí iría la lógica para formatear el sistema de archivos
        }
        "--help" | "-h" => {
            println!("Uso:");
            println!(
                "  ls [directorio]        Listar archivos en el directorio especificado (o raíz si no se especifica)"
            );
            println!("  create <archivo> <contenido>  Crear un archivo con el contenido dado");
            println!("  format                 Formatear el sistema de archivos");
            println!("  --help, -h            Mostrar esta ayuda");
        }
        _ => {
            println!(
                "Comando desconocido: {}\nEscriba --help para más información",
                cmd
            );
        }
    }
}

fn cat(filename: &str) {
    // Abre el archivo del sistema de archivos
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(".bin")
        .expect("Sistema de archivo no encontrado corre 'format' primero!");

    let superblock = read_superblock(&mut file);
    // Aquí iría la lógica para leer y mostrar el contenido del archivo
    let inode_table_offset = std::mem::size_of::<Superblock>() as u64;
    let inode_size = std::mem::size_of::<Inode>();

    for i in 0..superblock.total_inodes {
        file.seek(std::io::SeekFrom::Start(
            inode_table_offset + (i as u64 * inode_size as u64),
        ))
        .unwrap();
        let mut inode_bytes = vec![0u8; inode_size];
        file.read_exact(&mut inode_bytes).unwrap();
        let inode = unsafe { std::ptr::read(inode_bytes.as_ptr() as *const Inode) };
        let name = String::from_utf8_lossy(&inode.name)
            .trim_end_matches('\0')
            .to_string();
        if name == filename {
            println!("{:<32}{:>8}", name, inode.size);
            let bitmap_offset =
                inode_table_offset + (superblock.total_inodes as u64 * inode_size as u64);
            let bitmap_size = ((superblock.total_blocks + 7) / 8) as usize;
            let data_area_offset = bitmap_offset + bitmap_size as u64;
            let mut content_bytes = vec![0u8; inode.size as usize];
            for block in 0..inode.block_count {
                let block_offset = data_area_offset
                    + ((inode.start_block + block) as u64 * superblock.block_size as u64);
                file.seek(std::io::SeekFrom::Start(block_offset)).unwrap();
                let to_read = if block == inode.block_count - 1 {
                    inode.size as usize - (block as usize * superblock.block_size as usize)
                } else {
                    superblock.block_size as usize
                };
                file.read_exact(
                    &mut content_bytes[block as usize * superblock.block_size as usize
                        ..block as usize * superblock.block_size as usize + to_read],
                )
                .unwrap();
            }
            println!(
                "Contenido de {}: {}",
                name,
                String::from_utf8_lossy(&content_bytes)
            );
        }
    }
}



fn ls() {
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(".bin")
        .expect("Sistema de archivo no encontrado corre 'format' primero!");

    let superblock = read_superblock(&mut file);
    let inode_table_offset = std::mem::size_of::<Superblock>() as u64;
    let inode_size = std::mem::size_of::<Inode>();
    println!("{:<32}{:>8}", "nombre", "size");
    for i in 0..superblock.total_inodes {
        file.seek(std::io::SeekFrom::Start(
            inode_table_offset + (i as u64 * inode_size as u64),
        ))
        .unwrap();
        let mut inode_bytes = vec![0u8; inode_size];
        file.read_exact(&mut inode_bytes).unwrap();
        let inode = unsafe { std::ptr::read(inode_bytes.as_ptr() as *const Inode) };

        if inode.used == 1 {
            {
                let name = String::from_utf8_lossy(&inode.name)
                    .trim_end_matches('\0')
                    .to_string();
                println!("{:<32}{:>8}", name, inode.size);
            }
        }
    }
}

fn read_superblock(file: &mut fs::File) -> Superblock {
    let mut sb_bytes = vec![0u8; std::mem::size_of::<Superblock>()];
    file.read_exact(&mut sb_bytes).unwrap();

    let superblock = unsafe { std::ptr::read(sb_bytes.as_ptr() as *const Superblock) };
    println!(
        "Informacion - Tamaño total: {}, Tamaño del bloque: {}, Bloques totales: {}",
        superblock.total_inodes, superblock.block_size, superblock.total_blocks
    );
    return superblock;
}

fn find_free_inode(file: &mut fs::File, superblock: &Superblock) -> Option<u32> {
    let inode_table_offset = std::mem::size_of::<Superblock>() as u64;
    let inode_size = std::mem::size_of::<Inode>();

    for i in 0..superblock.total_inodes {
        file.seek(std::io::SeekFrom::Start(
            inode_table_offset + (i as u64 * inode_size as u64),
        ))
        .unwrap();
        let mut inode_bytes = vec![0u8; inode_size];
        file.read_exact(&mut inode_bytes).unwrap();
        let inode = unsafe { std::ptr::read(inode_bytes.as_ptr() as *const Inode) };

        if inode.used == 0 {
            return Some(i);
        }
    }
    None
}

fn find_free_block(
    file: &mut fs::File,
    superblock: &Superblock,
    content: &str,
) -> Option<(Vec<u32>, Vec<u8>)> {  // 👈 Return tuple (blocks, bitmap)
    let inode_table_offset = std::mem::size_of::<Superblock>() as u64;
    let inode_size = std::mem::size_of::<Inode>();
    let bitmap_offset = inode_table_offset + (superblock.total_inodes as u64 * inode_size as u64);
    file.seek(std::io::SeekFrom::Start(bitmap_offset)).unwrap();

    let bitmap_size = ((superblock.total_blocks + 7) / 8) as usize;
    let mut bitmap_bits = vec![0u8; bitmap_size];
    file.read_exact(&mut bitmap_bits).unwrap();

    let content_bytes = content.as_bytes();
    let blocks_needed =
        ((content_bytes.len() as u32 + superblock.block_size - 1) / superblock.block_size).max(1);

    let mut allocated_blocks = Vec::new();
    for block in 0..superblock.total_blocks {
        let byte_idx = (block / 8) as usize;
        let bit_idx = block % 8;

        if bitmap_bits[byte_idx] & (1 << bit_idx) == 0 {
            // Mark as used
            bitmap_bits[byte_idx] |= 1 << bit_idx;
            allocated_blocks.push(block);

            if allocated_blocks.len() == blocks_needed as usize {
                break;
            }
        }
    }

    if allocated_blocks.len() < blocks_needed as usize {
        panic!("Not enough free blocks!");
    }
    Some((allocated_blocks, bitmap_bits))  // 👈 Return BOTH
}

fn write_content_to_blocks(
    file: &mut fs::File,
    superblock: &Superblock,
    allocated_blocks: &Vec<u32>,

    content: &str,
) {
    let inode_table_offset = std::mem::size_of::<Superblock>() as u64;
    let inode_size = std::mem::size_of::<Inode>();
    let bitmap_offset = inode_table_offset + (superblock.total_inodes as u64 * inode_size as u64);
    let bitmap_size = ((superblock.total_blocks + 7) / 8) as usize;

    // let data_area_offset = superblock.block_size as u64 * superblock.total_blocks as u64
    //     - (superblock.block_size as u64 * superblock.total_blocks as u64
    //         - bitmap_offset
    //         - bitmap_size as u64);

    let content_bytes = content.as_bytes();
    for (i, &block) in allocated_blocks.iter().enumerate() {
        let block_offset =
            bitmap_offset + bitmap_size as u64 + (block as u64 * superblock.block_size as u64);
        file.seek(std::io::SeekFrom::Start(block_offset)).unwrap();

        let start = i * superblock.block_size as usize;
        let end = ((i + 1) * superblock.block_size as usize).min(content_bytes.len());
        file.write_all(&content_bytes[start..end]).unwrap();
    }
    // return data_area_offset;
}

fn write_inode(
    name: &str,
    file: &mut fs::File,
    superblock: &Superblock,
    inode_idx: u32,
    allocated_blocks: &Vec<u32>,
    content: &str,
) {
    let mut name_bytes = [0u8; 32];
    let name_len = name.len().min(31);
    name_bytes[..name_len].copy_from_slice(&name.as_bytes()[..name_len]);
    let content_bytes = content.as_bytes();
    let blocks_needed =
        ((content_bytes.len() as u32 + superblock.block_size - 1) / superblock.block_size).max(1);
    let new_inode = Inode {
        used: 1,
        name: name_bytes,
        size: content_bytes.len() as u32,
        start_block: allocated_blocks[0],
        block_count: blocks_needed,
    };
    let inode_table_offset = std::mem::size_of::<Superblock>() as u64;
    let inode_size = std::mem::size_of::<Inode>();
    file.seek(std::io::SeekFrom::Start(
        inode_table_offset + (inode_idx as u64 * inode_size as u64),
    ))
    .unwrap();
    let inode_bytes =
        unsafe { std::slice::from_raw_parts(&new_inode as *const Inode as *const u8, inode_size) };
    file.write_all(inode_bytes).unwrap();
}

fn create(name: &str, content: &str) {
    print!("Creando archivo: {} con el contenido: {}\n", name, content);
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(".bin")
        .expect("Sistema de archivo no encontrado corre 'format' primero!");

    let superblock = read_superblock(&mut file);

    let inode_table_offset = std::mem::size_of::<Superblock>() as u64;
    let inode_size = std::mem::size_of::<Inode>();
    let free_inode_index = find_free_inode(&mut file, &superblock);
    
    // 👇 Now we get BOTH blocks and bitmap
    let (allocated_blocks, bitmap_bits) = find_free_block(&mut file, &superblock, content)
        .expect("Failed to allocate blocks");
    
    let content_bytes = content.as_bytes();
    let blocks_needed =
        ((content_bytes.len() as u32 + superblock.block_size - 1) / superblock.block_size).max(1);
    let bitmap_offset = inode_table_offset + (superblock.total_inodes as u64 * inode_size as u64);
    
    println!(
        "Free inode index: {:?}, allocated blocks: {:?}",
        free_inode_index, allocated_blocks
    );
    
    write_content_to_blocks(
        &mut file,
        &superblock,
        &allocated_blocks,  // 👈 No need to clone/unwrap
        content,
    );
    
    write_inode(
        name,
        &mut file,
        &superblock,
        free_inode_index.unwrap(),
        &allocated_blocks,  // 👈 No need to unwrap
        content,
    );

    // ✅ Write the UPDATED bitmap (not a fresh one!)
    file.seek(std::io::SeekFrom::Start(bitmap_offset)).unwrap();
    file.write_all(&bitmap_bits).unwrap();  // 👈 Use the bitmap from find_free_block!

    // Update superblock
    let updated_sb = Superblock {
        used_blocks: superblock.used_blocks + blocks_needed,
        used_inodes: superblock.used_inodes + 1,
        ..superblock
    };
    file.seek(std::io::SeekFrom::Start(0)).unwrap();
    let sb_bytes = unsafe {
        std::slice::from_raw_parts(
            &updated_sb as *const Superblock as *const u8,
            std::mem::size_of::<Superblock>(),
        )
    };
    file.write_all(sb_bytes).unwrap();

    file.sync_all().unwrap();
    println!("✅ archivo '{}' creado exitosamente!", name);
}
fn format() {
    let block_size: u32 = 512;
    let total_blocks: u32 = 1024;
    let total_inodes: u32 = 128;
    let superblock = Superblock {
        total_size: block_size * total_blocks,
        block_size,
        total_blocks,
        total_inodes,
        used_blocks: 0,
        used_inodes: 0,
    };
    let inode_table: Vec<Inode> = vec![
        Inode {
            used: 0,
            name: [0; 32],
            size: 0,
            start_block: 0,
            block_count: 0,
        };
        total_inodes as usize
    ];

    let bitmap = Bitmap {
        bits: vec![0; (total_blocks / 8) as usize],
    };

    let mut file = fs::File::create(".bin").expect("Unable to format filesystem");

    let sb_bytes = unsafe {
        std::slice::from_raw_parts(
            &superblock as *const Superblock as *const u8,
            std::mem::size_of::<Superblock>(),
        )
    };
    file.write_all(sb_bytes).unwrap();

    // Write inode table
    for inode in &inode_table {
        let inode_bytes = unsafe {
            std::slice::from_raw_parts(
                inode as *const Inode as *const u8,
                std::mem::size_of::<Inode>(),
            )
        };
        file.write_all(inode_bytes).unwrap();
    }

    // Write bitmap
    file.write_all(&bitmap.bits).unwrap();

    // Optionally write empty data blocks
    file.set_len((block_size * total_blocks) as u64).unwrap();

    println!("Filesystem formatted!");
}
