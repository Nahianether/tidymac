use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

const CHUNK_SIZE: usize = 65536;
const PASSES: u32 = 3;

/// Securely shred a file by overwriting its content before deletion.
/// Pass pattern: random, zeros, random.
/// Returns bytes freed on success.
pub fn shred_file(
    path: &Path,
    progress_fn: &mut dyn FnMut(&str),
) -> Result<u64, std::io::Error> {
    let meta = std::fs::metadata(path)?;

    if meta.is_dir() {
        // For directories, shred each file inside then remove the directory
        let mut total = 0u64;
        for entry in walkdir::WalkDir::new(path)
            .follow_links(false)
            .contents_first(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let p = entry.path();
            if p.is_file() {
                total += shred_single_file(p, progress_fn)?;
            } else if p.is_dir() && p != path {
                std::fs::remove_dir(p)?;
            }
        }
        std::fs::remove_dir(path)?;
        return Ok(total);
    }

    shred_single_file(path, progress_fn)
}

fn shred_single_file(
    path: &Path,
    progress_fn: &mut dyn FnMut(&str),
) -> Result<u64, std::io::Error> {
    let size = std::fs::metadata(path)?.len();
    if size == 0 {
        std::fs::remove_file(path)?;
        return Ok(0);
    }

    let mut file = std::fs::OpenOptions::new().write(true).open(path)?;
    let mut buf = vec![0u8; CHUNK_SIZE];

    for pass in 1..=PASSES {
        let fill_zeros = pass == 2;
        progress_fn(&format!(
            "Shredding pass {}/{}: {}",
            pass,
            PASSES,
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ));

        file.seek(SeekFrom::Start(0))?;
        let mut remaining = size;

        while remaining > 0 {
            let chunk = (remaining as usize).min(CHUNK_SIZE);
            if fill_zeros {
                buf[..chunk].fill(0);
            } else {
                fill_random(&mut buf[..chunk]);
            }
            file.write_all(&buf[..chunk])?;
            remaining -= chunk as u64;
        }

        file.flush()?;
        file.sync_all()?;
    }

    drop(file);
    std::fs::remove_file(path)?;
    Ok(size)
}

fn fill_random(buf: &mut [u8]) {
    // Simple PRNG fill — fast enough for shredding, no external dep needed
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);
    let mut state = hasher.finish();

    for byte in buf.iter_mut() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *byte = (state >> 33) as u8;
    }
}
