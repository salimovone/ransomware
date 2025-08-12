use std::path::Path;
use std::process::Command;
use std::{env, fs};

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
