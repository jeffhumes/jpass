use rand::seq::SliceRandom;

const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{}<>?";

#[derive(Debug, Clone, Copy)]
pub struct PasswordOptions {
    pub length: usize,
    pub lowercase: bool,
    pub uppercase: bool,
    pub digits: bool,
    pub symbols: bool,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 20,
            lowercase: true,
            uppercase: true,
            digits: true,
            symbols: true,
        }
    }
}

/// Generate a random password from the requested character classes,
/// guaranteeing at least one character from each enabled class.
pub fn generate_password(opts: PasswordOptions) -> String {
    let mut pools: Vec<&[u8]> = Vec::new();
    if opts.lowercase {
        pools.push(LOWER);
    }
    if opts.uppercase {
        pools.push(UPPER);
    }
    if opts.digits {
        pools.push(DIGITS);
    }
    if opts.symbols {
        pools.push(SYMBOLS);
    }
    if pools.is_empty() {
        pools.push(LOWER);
    }

    let mut rng = rand::thread_rng();
    let length = opts.length.max(pools.len());

    // Guarantee one char per enabled class, then fill the rest randomly.
    let mut chars: Vec<u8> = pools
        .iter()
        .map(|pool| *pool.choose(&mut rng).unwrap())
        .collect();
    let all: Vec<u8> = pools.iter().flat_map(|p| p.iter().copied()).collect();
    while chars.len() < length {
        chars.push(*all.choose(&mut rng).unwrap());
    }
    chars.shuffle(&mut rng);

    String::from_utf8(chars).expect("password charset is ASCII")
}
