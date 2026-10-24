/// An explicit, infallible source of random words consumed by domain calculations.
pub trait RandomInput {
    fn next_u64(&mut self) -> u64;
}
