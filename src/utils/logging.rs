use std::fs::OpenOptions;
use std::io::Write;

#[allow(dead_code)]
pub fn debug_to_file(content: &str) {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("./output.txt")
        .unwrap();
    
    let content_with_newline = content.to_owned() + "\n";

    file.write_all(content_with_newline.as_bytes()).unwrap();
}