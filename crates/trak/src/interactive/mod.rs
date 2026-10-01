//! The interactive session: `trak` with no subcommand.

// Reachable only from `run`, which arrives with the loop.
#[allow(dead_code)]
mod session;
#[cfg(test)]
mod testing;
