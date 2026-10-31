pub fn main() {
    save();
}

pub fn save() {
    report_full("/tmp");
}

pub fn report_full(path: &str) {
    eprintln!("disk is full: {}", path);
}
