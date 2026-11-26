//! Reading files out of a Valve VPK (version 1 and 2) archive set: `<name>_dir.vpk` holds the
//! directory tree (and small files), `<name>_NNN.vpk` the data.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

struct Entry {
    preload: Vec<u8>,
    archive: u16,
    offset: u32,
    length: u32,
}

pub struct Vpk {
    dir: PathBuf,
    /// Where the data stored in the dir file starts.
    data_start: u64,
    entries: HashMap<String, Entry>,
}

fn u16_at(b: &[u8], i: &mut usize) -> Result<u16> {
    let v = b.get(*i..*i + 2).context("truncated VPK tree")?;
    *i += 2;
    Ok(u16::from_le_bytes([v[0], v[1]]))
}

fn u32_at(b: &[u8], i: &mut usize) -> Result<u32> {
    let v = b.get(*i..*i + 4).context("truncated VPK tree")?;
    *i += 4;
    Ok(u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
}

fn cstr(b: &[u8], i: &mut usize) -> Result<String> {
    let end = b[*i..].iter().position(|&c| c == 0).context("truncated VPK tree")?;
    let s = String::from_utf8_lossy(&b[*i..*i + end]).into_owned();
    *i += end + 1;
    Ok(s)
}

impl Vpk {
    /// Open `<name>_dir.vpk`.
    pub fn open(dir: &Path) -> Result<Self> {
        let bytes = std::fs::read(dir).with_context(|| format!("reading {}", dir.display()))?;
        let mut i = 0;
        ensure!(u32_at(&bytes, &mut i)? == 0x55AA_1234, "{} is not a VPK directory file", dir.display());
        let version = u32_at(&bytes, &mut i)?;
        let tree_size = u32_at(&bytes, &mut i)? as usize;
        let header = match version {
            1 => 12,
            2 => 28,
            v => bail!("VPK version {v} is not supported"),
        };
        i = header;
        let tree = bytes.get(header..header + tree_size).context("truncated VPK tree")?;
        let mut entries = HashMap::new();
        let mut t = 0;
        loop {
            let ext = cstr(tree, &mut t)?;
            if ext.is_empty() {
                break;
            }
            loop {
                let path = cstr(tree, &mut t)?;
                if path.is_empty() {
                    break;
                }
                loop {
                    let name = cstr(tree, &mut t)?;
                    if name.is_empty() {
                        break;
                    }
                    let _crc = u32_at(tree, &mut t)?;
                    let preload_len = u16_at(tree, &mut t)? as usize;
                    let archive = u16_at(tree, &mut t)?;
                    let offset = u32_at(tree, &mut t)?;
                    let length = u32_at(tree, &mut t)?;
                    ensure!(u16_at(tree, &mut t)? == 0xFFFF, "bad VPK entry terminator");
                    let preload = tree.get(t..t + preload_len).context("truncated VPK preload")?.to_vec();
                    t += preload_len;
                    let full = if path.trim().is_empty() { format!("{name}.{ext}") } else { format!("{path}/{name}.{ext}") };
                    entries.insert(full.to_ascii_lowercase(), Entry { preload, archive, offset, length });
                }
            }
        }
        let _ = i;
        Ok(Self { dir: dir.to_owned(), data_start: (header + tree_size) as u64, entries })
    }

    /// A file's bytes by its path inside the archive (`panorama/images/icons/icon_cart.vsvg_c`).
    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        let e = self.entries.get(&name.to_ascii_lowercase()).with_context(|| format!("{name} is not in the VPK"))?;
        let mut out = e.preload.clone();
        if e.length > 0 {
            let (file, start) = if e.archive == 0x7FFF {
                (self.dir.clone(), self.data_start + e.offset as u64)
            } else {
                let stem = self.dir.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix("_dir.vpk")).context("VPK name does not end in _dir.vpk")?;
                (self.dir.with_file_name(format!("{stem}_{:03}.vpk", e.archive)), e.offset as u64)
            };
            let mut f = std::fs::File::open(&file).with_context(|| format!("opening {}", file.display()))?;
            f.seek(SeekFrom::Start(start))?;
            let mut data = vec![0; e.length as usize];
            f.read_exact(&mut data)?;
            out.extend(data);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A v2 VPK with one file in the dir file and one in archive 000.
    #[test]
    fn reads_both_kinds_of_entries() {
        let dir = std::env::temp_dir().join(format!("vpk-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut tree = Vec::new();
        let entry = |tree: &mut Vec<u8>, name: &str, preload: &[u8], archive: u16, offset: u32, length: u32| {
            tree.extend(name.as_bytes());
            tree.push(0);
            tree.extend(0u32.to_le_bytes());
            tree.extend((preload.len() as u16).to_le_bytes());
            tree.extend(archive.to_le_bytes());
            tree.extend(offset.to_le_bytes());
            tree.extend(length.to_le_bytes());
            tree.extend(0xFFFFu16.to_le_bytes());
            tree.extend(preload);
        };
        tree.extend(b"txt\0icons\0");
        entry(&mut tree, "a", b"pre", 0x7FFF, 0, 4);
        entry(&mut tree, "b", b"", 0, 2, 3);
        tree.extend(b"\0\0\0");
        let mut file = Vec::new();
        for v in [0x55AA_1234u32, 2, tree.len() as u32, 4, 0, 0, 0] {
            file.extend(v.to_le_bytes());
        }
        file.extend(&tree);
        file.extend(b"DATA");
        std::fs::write(dir.join("pak01_dir.vpk"), &file).unwrap();
        std::fs::write(dir.join("pak01_000.vpk"), b"xxabc").unwrap();
        let v = Vpk::open(&dir.join("pak01_dir.vpk")).unwrap();
        assert_eq!(v.read("icons/a.txt").unwrap(), b"preDATA");
        assert_eq!(v.read("ICONS/B.TXT").unwrap(), b"abc");
        assert!(v.read("icons/c.txt").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
