use trak_core::Error;

use super::relevant::run_context;

pub fn run(message: &str) -> Result<(), Error> {
    run_context(message, "error message")
}
