use property_simulation_random_engine::RandomInput;
use property_simulation_slice_catalogue::SliceResult;

/// A Slice calculation that does not consume random input.
pub trait DeterministicSlice {
    type Error: std::error::Error + 'static;

    fn calculate(&self) -> Result<SliceResult, Self::Error>;
}

/// A Slice simulation that consumes an explicit random input source.
pub trait StochasticSlice {
    type Error: std::error::Error + 'static;

    fn simulate(&self, random_input: &mut dyn RandomInput) -> Result<SliceResult, Self::Error>;
}
