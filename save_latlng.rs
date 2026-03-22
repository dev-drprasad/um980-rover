use std::fs::OpenOptions;
use std::io::Result;
use std::io::Write;

pub async fn save_latlng() {
    write_to_file("Sample data").await.unwrap();
}

async fn write_to_file(data: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file_path)?;

    file.write_all(data.as_bytes())?;

    println!("Successfully created and wrote to new file: {}", file_path);

    Ok(())
}
