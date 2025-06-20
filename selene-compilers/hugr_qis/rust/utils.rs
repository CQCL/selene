use anyhow::{anyhow, Result};
use tket2::hugr::package::Package;
use tket2::hugr::Hugr;

use crate::REGISTRY;

/// interprets the string as a hugr package and, verifies there is exactly one module in the
/// package, then extracts and returns that module.
pub fn read_hugr_envelope(bytes: &[u8]) -> Result<Hugr> {
    let package = Package::load(bytes, Some(&REGISTRY))?;
    if package.modules.len() != 1 {
        return Err(anyhow!(
            "Expected exactly one module in the package, found {}",
            package.modules.len()
        ));
    }
    package.validate()?;
    // some more validation can be done here, e.g. extension version checking.
    Ok(package.modules[0].clone())
}
