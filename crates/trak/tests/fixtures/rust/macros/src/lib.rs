macro_rules! make_pair {
    ($a:expr, $b:expr) => {
        ($a, $b)
    };
}

fn use_macro() -> (u8, u8) {
    make_pair!(1, 2)
}