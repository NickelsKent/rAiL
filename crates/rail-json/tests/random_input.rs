//! Seeded random-input test (NFR4.3): no input may panic the parser, and any
//! input it accepts must round-trip through the writer.

use rail_json::{parse, to_string};

/// xorshift64*: a tiny deterministic generator, so no test crate is needed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const SEED: u64 = 0x5241_494c_4a53_4f4e; // "RAILJSON"
const CASES: usize = 20_000;
const ALPHABET: &[u8] = b"{}[]:,\"\\/ \t\n\r0123456789-+.eEtruefalsnbu\x00\xc3\xa9\xff";
const SEEDS: &[&str] = &[
    r#"{"jsonrpc":"2.0","id":1,"method":"check.run","params":{"module":"a/b.rlc"}}"#,
    r#"[1,-2.5e10,true,false,null,"é\n",{"k":[{}]}]"#,
];

fn check(input: &[u8]) {
    if let Ok(value) = parse(input) {
        let written = to_string(&value);
        let again = parse(written.as_bytes()).expect("written JSON parses");
        assert_eq!(value, again, "round trip of {input:?}");
    }
}

#[test]
fn random_bytes_and_mutations_never_panic() {
    eprintln!("random_input seed = {SEED:#018x}, cases = {CASES}");
    let mut rng = Rng(SEED);
    for _ in 0..CASES {
        let mut input: Vec<u8> = if rng.below(2) == 0 {
            let len = rng.below(64);
            (0..len)
                .map(|_| ALPHABET[rng.below(ALPHABET.len())])
                .collect()
        } else {
            SEEDS[rng.below(SEEDS.len())].as_bytes().to_vec()
        };
        for _ in 0..rng.below(4) {
            if input.is_empty() {
                break;
            }
            let at = rng.below(input.len());
            match rng.below(3) {
                0 => input[at] = ALPHABET[rng.below(ALPHABET.len())],
                1 => {
                    input.remove(at);
                }
                _ => input.insert(at, ALPHABET[rng.below(ALPHABET.len())]),
            }
        }
        check(&input);
    }
}
