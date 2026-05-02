//! Minimal deterministic RNG (xorshift64). No global state — seed is explicit.
use solana_sdk::pubkey::Pubkey;

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Guard against seed=0 which xorshift can't recover from.
        let state = if seed == 0 { 0xDEAD_BEEF_CAFE_1234 } else { seed };
        Self { state }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    pub fn next_bytes<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0u8; N];
        let mut i = 0;
        while i < N {
            let val = self.next_u64().to_le_bytes();
            let take = (N - i).min(8);
            out[i..i + take].copy_from_slice(&val[..take]);
            i += take;
        }
        out
    }

    pub fn next_pubkey(&mut self) -> Pubkey {
        Pubkey::new_from_array(self.next_bytes::<32>())
    }

    /// Pick a value in `[0, n)`.
    pub fn next_usize_mod(&mut self, n: usize) -> usize {
        if n == 0 { return 0; }
        (self.next_u64() as usize) % n
    }
}
