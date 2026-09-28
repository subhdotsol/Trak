pub fn main() {
    setup();
    teardown();
}

pub fn setup() {
    let name = read_name().unwrap();
    if name.is_empty() {
        log_empty();
    }
}

pub fn teardown() {
    flush_state();
}

pub fn read_name() -> Result<String, String> {
    let mut buffer = String::new();
    if buffer.is_empty() {
        return Err("failed to read input".to_string());
    }
    Ok(buffer.trim().to_string())
}

pub fn log_empty() {
    eprintln!("failed to read input: {}");
}

pub fn flush_state() {}

const NOTICE: &str = "no such user on disk";
