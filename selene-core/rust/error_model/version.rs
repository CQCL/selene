use anyhow::{Result, anyhow};

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ErrorModelAPIVersion {
    /// Reserved for future use, must be 0.
    reserved: u8,
    /// Major version of the API.
    major: u8,
    /// Minor version of the API.
    minor: u8,
    /// Patch version of the API.
    patch: u8,
}

impl From<u64> for ErrorModelAPIVersion {
    fn from(value: u64) -> Self {
        Self {
            reserved: ((value >> 24) & 255) as u8,
            major: ((value >> 16) & 255) as u8,
            minor: ((value >> 8) & 255) as u8,
            patch: (value & 255) as u8,
        }
    }
}
impl From<ErrorModelAPIVersion> for u64 {
    fn from(value: ErrorModelAPIVersion) -> u64 {
        ((value.reserved as u64) << 24)
            | ((value.major as u64) << 16)
            | ((value.minor as u64) << 8)
            | (value.patch as u64)
    }
}

pub const CURRENT_API_VERSION: ErrorModelAPIVersion = ErrorModelAPIVersion {
    reserved: 0,
    major: 0,
    minor: 2,
    patch: 0,
};

// Changelog:
// 0.1.0: Initial version.
// 0.2.0: Replaced set_measurement_result with set_bool_result and set_u64_result in
//   ErrorModelSetResultInterface

impl ErrorModelAPIVersion {
    pub fn validate(&self) -> Result<()> {
        // Note: this is a naive check at the moment, as we have not introduced a breaking
        // change since versioning was introduced. This logic should evolve as and when
        // changes occur.
        //
        // As such, this is mostly a sketch of what the logic may look like.
        //
        // Reserved must be 0. We may want to attribute meaning to this one day.
        if self.reserved != 0 {
            return Err(anyhow!(
                "API version reserved field must be 0, got {}",
                self.reserved
            ));
        }
        // If the major version is different, the plugin is almost definitely incompatible.
        if self.major != CURRENT_API_VERSION.major {
            return Err(anyhow!(
                "API major version must be the same as Selene's API major version ({}), got {}",
                CURRENT_API_VERSION.major,
                self.major
            ));
        }
        // If the minor version is different, the plugin might be incompatible. This function
        // should be updated to reflect the actual changes in the API. For now we reject.
        if self.minor != CURRENT_API_VERSION.minor {
            return Err(anyhow!(
                "API minor version must be the same as Selene's API minor version ({}), got {}",
                CURRENT_API_VERSION.minor,
                self.minor
            ));
        }
        // The patch version might be bumped for changes that are optional and additive.
        // For example, if we wish to allow error models to have a selene_error_model_foo() function,
        // but it is not mandatory, we express the symbol as optional in plugin.rs, and the
        // behaviour of selene does not change with regards to the other functions (i.e. we do
        // not avoid calling shot_end() because selene_error_model_foo() has taken over that meaning),
        // then we can bump the patch self. Otherwise we should bump the minor self.
        Ok(())
    }
}
