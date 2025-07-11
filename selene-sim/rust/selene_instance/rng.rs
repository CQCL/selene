use crate::selene_instance::SeleneInstance;
use anyhow::{Result, anyhow};
use rand::Rng;
use rand_pcg::Pcg32;

impl SeleneInstance {
    pub fn random_seed(&mut self, seed: u64) {
        self.prng = Some(Pcg32::new(42, seed));
    }
    /// Produces a random 32-bit unsigned integer.
    pub fn random_u32(&mut self) -> Result<u32> {
        let Some(ref mut rng) = self.prng else {
            return Err(anyhow!("PRNG has not been seeded"));
        };
        Ok(rng.random())
    }
    /// Produces a random 32-bit float.
    pub fn random_f64(&mut self) -> Result<f64> {
        match self.random_u32() {
            Ok(r) => Ok(r as f64 / 2u64.pow(32) as f64),
            Err(e) => Err(e),
        }
    }
    /// Produces a bounded random 32-bit unsigned integer.
    pub fn random_u32_bounded(&mut self, bound: u32) -> Result<u32> {
        let Some(ref mut rng) = self.prng else {
            return Err(anyhow!("PRNG has not been seeded"));
        };
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let r = rng.random::<u32>();
            if r >= threshold {
                return Ok(r % bound);
            }
        }
    }
}
