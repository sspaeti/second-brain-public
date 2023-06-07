use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

pub fn convert_to_lower_case(file_path: &Path) -> io::Result<()> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let content = content.to_lowercase();

    let mut file = File::create(file_path)?;
    file.write_all(content.as_bytes())?;

    Ok(())
}
