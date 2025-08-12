use std::process::Command;
pub fn hide_app(app: &String) {
    let output = Command::new("attrib")
        .args(&["+h", app])
        .output()
        .expect("Buyruq bajarilmadi");

    // stdout chiqarish
    println!("{}", String::from_utf8_lossy(&output.stdout));

    // Agar xatolar bo'lsa, stderr chiqarish
    if !output.stderr.is_empty() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }
}

pub fn run_as_admin(app_path: &str) {
    let ps_command = format!(
        "Start-Process -FilePath '{}' -ArgumentList '-first' -Verb RunAs",
        app_path
    );

    let status = Command::new("powershell")
        .arg("-Command")
        .arg(ps_command)
        .status()
        .expect("PowerShell komanda ishlamadi");

    println!("Chiqish kodi: {:?}", status.code());
}
