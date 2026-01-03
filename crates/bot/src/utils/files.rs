use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

pub fn download_file(url: &str, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::blocking::get(url)?;
    let bytes = response.bytes()?;
    let mut file = File::create(dest)?;
    file.write_all(&bytes)?;
    Ok(())
}

pub fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}

pub fn move_files_from_subdir(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<_> = fs::read_dir(dir)?.collect();
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() && path.file_name().map(|n| n.to_string_lossy().contains("xmrig")).unwrap_or(false) {
            let subdir_entries: Vec<_> = fs::read_dir(&path)?.collect();
            
            for subentry in subdir_entries {
                let subentry = subentry?;
                let src = subentry.path();
                let filename = src.file_name().unwrap();
                let dst = dir.join(filename);
                
                if dst.exists() {
                    if dst.is_dir() {
                        fs::remove_dir_all(&dst)?;
                    } else {
                        fs::remove_file(&dst)?;
                    }
                }
                fs::rename(&src, &dst)?;
            }
            let _ = fs::remove_dir(&path);
            break;
        }
    }
    Ok(())
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let path = entry.path();
        let name = path.file_name().ok_or(io::Error::new(io::ErrorKind::Other, "No filename"))?;
        let dst_path = dst.join(name);

        if ty.is_dir() {
            copy_dir_recursive(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)?;
        }
        
        #[cfg(windows)]
        {
            // Re-apply hidden attributes to copied files if needed, 
            // but we will do a blanket hide later.
        }
    }
    Ok(())
}
