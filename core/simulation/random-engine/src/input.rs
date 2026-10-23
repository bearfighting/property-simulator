/// A source of random words consumed by domain calculation models.
pub trait RandomInput {
    type Error;

    fn next_u64(&mut self) -> Result<u64, Self::Error>;
}
