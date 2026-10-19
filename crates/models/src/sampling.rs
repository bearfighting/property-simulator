/// Sampling is an injected concern; this minimal contract deliberately leaves
/// random-stream allocation to the M3 runner.
pub trait Sampler<D> {
    type Sample;
    fn sample(&mut self, distribution: &D) -> Self::Sample;
}
