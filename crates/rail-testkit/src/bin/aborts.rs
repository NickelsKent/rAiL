//! Test helper: ends by a signal (SIGABRT; tests signal reporting, NFR9.9).

#![forbid(unsafe_code)]

fn main() {
    std::process::abort();
}
