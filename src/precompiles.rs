use crate::runtime::{Context, ExitError, ExitSucceed};
use primitive_types::H160;

#[cfg(feature = "std")]
use std::vec::Vec;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

pub fn precompiles(
    _address: H160,
    _input: &[u8],
    _context: &Context,
) -> Option<Result<(ExitSucceed, Vec<u8>), ExitError>> {
    None
}
