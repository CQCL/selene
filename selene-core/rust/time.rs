use std::ops;

#[derive(
    Default, Copy, Clone, Eq, PartialEq, Hash, Debug, derive_more::From, derive_more::Into,
)]
pub struct Instant(u64);

#[derive(
    Default,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Debug,
    derive_more::From,
    derive_more::Into,
    derive_more::Add,
    derive_more::AddAssign,
)]
/// Duration in nanoseconds
pub struct Duration(u64);

impl ops::Add<Duration> for Instant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl ops::AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        self.0 += rhs.0;
    }
}
