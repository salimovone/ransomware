use std::process::{Command, Child};
use std::thread;
use std::time::Duration;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, BufReader, ErrorKind};

pub fn manage_clones(exe_path: &str, base_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let max_instances = 3;
    let max_restart_attempts = 3;
    let mut children: Vec<Option<Child>> = Vec::with_capacity(max_instances);
    let mut restart_counts = vec![0usize; max_instances];

    // Vektorni None bilan to'ldirish
    for _ in 0..max_instances {
        children.push(None);
    }

    // Har bir instance uchun server va clone ni boshlash
    for i in 0..max_instances {
        let port = base_port + i as u16;
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
        println!("[*] Listening for clone {} on port {}", i, port);

        // Clone ni ishga tushirish, portni argument sifatida yuborish
        let child = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(format!(
                "Start-Process -FilePath '{}' -ArgumentList '-clone', '{}'",
                exe_path, i
            ))
            .spawn()
            .map_err(|e| Box::new(std::io::Error::new(ErrorKind::Other, format!("Spawn clone instance {} error: {}", i, e))))?;
        children[i] = Some(child);
        println!("[*] Clone instance {} started: {:?}", i, children[i].as_ref().unwrap().id());

        // Client ulanishini kutish
        let (stream, _) = listener.accept()?;
        println!("[*] Clone {} connected on port {}", i, port);

        // Heartbeat kuzatish thread
        let exe_path_clone = exe_path.to_string();
        let mut stream_reader = BufReader::new(stream);
        let mut local_restart_count = restart_counts[i];
        thread::spawn(move || {
            let mut buf = [0; 6]; // "alive\n" = 6 bayt
            loop {
                match stream_reader.read(&mut buf) {
                    Ok(0) => {
                        println!("[-] Connection lost for clone {}. Restarting...", i);
                        if local_restart_count < max_restart_attempts {
                            if let Err(e) = restart_clone(&exe_path_clone, i, port) {
                                eprintln!("[-] Failed to restart clone {}: {}", i, e);
                            }
                            local_restart_count += 1;
                        } else {
                            println!("[-] Max restarts reached for clone {}. Stopping.", i);
                            break;
                        }
                    }
                    Ok(n) if n > 0 => {
                        println!("[*] Received heartbeat from clone {}: {:?}", i, String::from_utf8_lossy(&buf[..n]));
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("[-] Error reading heartbeat from clone {}: {}. Restarting...", i, e);
                        if local_restart_count < max_restart_attempts {
                            if let Err(e) = restart_clone(&exe_path_clone, i, port) {
                                eprintln!("[-] Failed to restart clone {}: {}", i, e);
                            }
                            local_restart_count += 1;
                        } else {
                            println!("[-] Max restarts reached for clone {}. Stopping.", i);
                            break;
                        }
                    }
                }
                thread::sleep(Duration::from_secs(6));
            }
            // Thread tugaganda restart_counts ni yangilash uchun println ishlatamiz
            println!("[*] Clone {} final restart count: {}", i, local_restart_count);
        });
        // restart_counts ni yangilash kerak emas, chunki local_restart_count ishlatilmoqda
    }

    // Asosiy thread ni ushlab turish
    loop {
        thread::sleep(Duration::from_secs(10));
        let all_stopped = restart_counts.iter().all(|&count| count >= max_restart_attempts);
        if all_stopped {
            println!("[*] All clones stopped. Exiting manager.");
            break;
        }
    }

    Ok(())
}

fn restart_clone(exe_path: &str, i: usize, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(format!(
            "Start-Process -FilePath '{}' -ArgumentList '-clone', '{}'",
            exe_path, i
        ))
        .spawn()
        .map_err(|e| Box::new(std::io::Error::new(ErrorKind::Other, format!("Restart clone instance {} error: {}", i, e))))?;
    println!("[*] Clone instance {} restarted.", i);
    Ok(())
}