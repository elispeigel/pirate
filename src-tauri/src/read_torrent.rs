use std::fs::File;
use std::io::Read;

#[tauri::command]
fn read_torrent_file(path: String) -> Result<Vec<u8>, String> {
    let mut file = match File::open("/Users/elispeigel/code/pirate/puppy.torrent") {
        Ok(f) => {
            println!("Successfully opened file at path '{}'", path);
            f
        },
        Err(err) => {
            eprintln!("Failed to open file at path '{}': {}", path, err);
            return Err("Failed to open file".to_string());
        },
    };

    let mut contents = Vec::new();
    match file.read_to_end(&mut contents) {
        Ok(_) => {
            for byte in &contents {
                println!("{}", byte);
            }
            Ok(contents)
        },
        Err(_) => Err("Failed to read file".to_string()),
    }
}

