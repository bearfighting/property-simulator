use property_simulation_month::Month;
use property_simulation_random_engine::{
    RandomRootKey, RandomStream, RandomStreamIdentity, derive_stream,
};

pub(super) fn random_stream(
    root: &RandomRootKey,
    purpose: &[u8],
    month: Month,
    instance_month: Option<Month>,
) -> RandomStream {
    let mut bytes = b"property-simulation/monthly-slice/v1\0".to_vec();
    bytes.extend_from_slice(&(purpose.len() as u32).to_le_bytes());
    bytes.extend_from_slice(purpose);
    bytes.extend_from_slice(&month.number().to_le_bytes());
    if let Some(instance_month) = instance_month {
        bytes.extend_from_slice(&instance_month.number().to_le_bytes());
    }
    derive_stream(root, &RandomStreamIdentity::from_bytes(bytes))
}
