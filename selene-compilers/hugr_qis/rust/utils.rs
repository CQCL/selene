use crate::REGISTRY;
use anyhow::{Error, Result, anyhow};
use tket::hugr::Hugr;
use tket::hugr::envelope::get_generator;
use tket::hugr::package::Package;

/// interprets the string as a hugr package and, verifies there is exactly one module in the
/// package, then extracts and returns that module.
pub fn read_hugr_envelope(bytes: &[u8]) -> Result<Hugr> {
    let package = Package::load(bytes, Some(&REGISTRY))
        .map_err(|e| Error::new(e).context("Error loading HUGR package."))?;
    if package.modules.len() != 1 {
        return Err(anyhow!(
            "Expected exactly one module in the package, found {}",
            package.modules.len()
        ));
    }
    package.validate().map_err(|e| {
        let generator = get_generator(&package.modules);
        let any = Error::new(e);
        if let Some(generator) = generator {
            any.context(format!("in package with generator {generator}"))
        } else {
            any
        }
    })?;
    // some more validation can be done here, e.g. extension version checking.
    Ok(package.modules[0].clone())
}
