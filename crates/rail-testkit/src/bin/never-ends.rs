//! Test helper: never ends on its own (tests the run time limit, NFR5.5).

#![forbid(unsafe_code)]

fn main() {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
