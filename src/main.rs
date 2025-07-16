use std::{
    env::args_os,
    error::Error,
    ffi::OsString,
    fs::{self, File},
    io::{self, stdout, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    process::exit,
};

fn main() {
    let mut args = args_os();
    args.next();

    let Some(subcommand) = args.next() else {
        eprintln!("no subcommand provided");
        exit(1);
    };

    match subcommand.as_encoded_bytes() {
        b"create" => {
            let Some(dir_path) = args.next() else {
                eprintln!("directory not provided");
                exit(1);
            };

            if let Err(e) = create(PathBuf::from(&dir_path)) {
                eprintln!("error creating ofc for {}: {e}", dir_path.display());
                exit(2);
            };
        }
        b"read" => {
            let Some(ofc_path) = args.next() else {
                eprintln!("path to .ofc not provided");
                exit(1);
            };
            let Some(index) = args.next() else {
                eprintln!("index of file to read not provided");
                exit(1);
            };

            let Ok(index) = index.to_string_lossy().parse::<u32>() else {
                eprintln!("given invalid index of {}", index.display());
                exit(1);
            };

            if let Err(e) = read(PathBuf::from(&ofc_path), index) {
                eprintln!("error reading {}: {e}", ofc_path.display());
                exit(2);
            };
        }
        b"info" => {
            let Some(ofc_path) = args.next() else {
                eprintln!("path to .ofc not provided");
                exit(1);
            };

            if let Err(e) = info(PathBuf::from(&ofc_path)) {
                eprintln!("error getting info of {}: {e}", ofc_path.display());
                exit(2);
            };
        }
        _ => {
            eprintln!("unknown subcommand provided");
            exit(1);
        }
    }
}

struct ContainerEntry {
    path: PathBuf,
    size_bytes: u64,
    name: OsString,
}

fn create(dir_path: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut entries = Vec::new();

    for entry in fs::read_dir(&dir_path)? {
        let entry = entry?;

        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }

        entries.push(ContainerEntry {
            path: entry.path(),
            size_bytes: metadata.len(),
            name: entry.file_name(),
        });
    }

    let Ok(entry_count) = u32::try_from(entries.len()) else {
        return Err(format!(
            "exceeded max number of files: {} > {}",
            entries.len(),
            u32::MAX
        )
        .into());
    };

    entries.sort_unstable_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

    let mut output = File::create(dir_path.with_extension("ofc"))?;

    // Write file header
    {
        let mut header = vec![0u8; 8 + 8 * entries.len()];

        // Magic number
        header[..4].copy_from_slice(b"ofc\0");

        // Number of files in container
        header[4..8].copy_from_slice(&entry_count.to_le_bytes());

        let mut end_offset = 0u64;
        let mut header_index = 8;
        for entry in &entries {
            end_offset += entry.size_bytes;

            header[header_index..][..8].copy_from_slice(&end_offset.to_le_bytes());

            header_index += 8;
        }

        output.write_all(&header)?;
    }

    // Write files
    for entry in entries {
        let mut f = File::open(entry.path)?;
        io::copy(&mut f, &mut output)?;
    }

    Ok(())
}

fn read(ofc_path: PathBuf, index: u32) -> Result<(), Box<dyn Error>> {
    let mut f = File::open(&ofc_path)?;

    let mut buf = [0u8; 8];
    f.read_exact(&mut buf)?;

    if &buf[..4] != b"ofc\0" {
        return Err("not a valid ofc file".into());
    }

    let num_files = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);

    if index >= num_files {
        return Err(format!("index out of bounds: {index} >= {num_files}").into());
    }

    let header_len = 8 + 8 * u64::from(num_files);

    if index == 0 {
        f.read_exact(&mut buf)?;
        let len = u64::from_le_bytes(buf);

        f.seek(SeekFrom::Start(header_len))?;

        io::copy(&mut f.take(len), &mut stdout())?;

        return Ok(());
    }

    f.seek(SeekFrom::Start(8 + 8 * u64::from(index - 1)))?;

    let mut buf = [0u8; 16];
    f.read_exact(&mut buf)?;

    let file_start_offset = u64::from_le_bytes([
        buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
    ]);
    let file_len = u64::from_le_bytes([
        buf[8], buf[9], buf[10], buf[11], buf[12], buf[13], buf[14], buf[15],
    ]) - file_start_offset;

    f.seek(SeekFrom::Start(header_len + file_start_offset))?;

    io::copy(&mut f.take(file_len), &mut stdout())?;

    Ok(())
}

fn info(ofc_path: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut f = File::open(&ofc_path)?;

    let mut buf = [0u8; 8];
    f.read_exact(&mut buf)?;

    if &buf[..4] != b"ofc\0" {
        return Err("not a valid ofc file".into());
    }

    let num_files = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);

    let mut end_offsets = vec![0u8; usize::try_from(num_files).unwrap() * 8];
    f.read_exact(&mut end_offsets)?;

    let mut current_offset = 0u64;
    for i in 0..num_files {
        let byte_offset = usize::try_from(i).unwrap() * 8;
        let end_offset = u64::from_le_bytes([
            end_offsets[byte_offset],
            end_offsets[byte_offset + 1],
            end_offsets[byte_offset + 2],
            end_offsets[byte_offset + 3],
            end_offsets[byte_offset + 4],
            end_offsets[byte_offset + 5],
            end_offsets[byte_offset + 6],
            end_offsets[byte_offset + 7],
        ]);

        println!(
            "{i} offset={current_offset}, len={}",
            end_offset - current_offset
        );

        current_offset = end_offset;
    }

    Ok(())
}
