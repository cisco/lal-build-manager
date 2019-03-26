use super::{LalResult, Lockfile, Manifest};
use crate::input;

bitflags! {
    /// Flags to indicate variants on the verification process.
    #[derive(Default)]
    pub struct Flags: u8 {
        /// A simple verify was added to aid the workflow of stashed components.
        /// Users can use `lal verify --simple` or `lal build -s` aka. `--simple-verify`,
        /// instead of having to use `lal build --force` when just using stashed components.
        /// This avoids problems with different environments going undetected.
        const SIMPLE = 0b01;
        /// Indicates whether verification should allow testing channels, which are only to be
        /// used by automated systems.
        const TESTING = 0b10;
    }
}

/// Verifies that `./INPUT` satisfies all strictness conditions.
///
/// This first verifies that there are no key mismatches between `defaultConfig` and
/// `configurations` in the manifest.
///
/// Once this is done, `INPUT` is analysed thoroughly via each components lockfiles.
/// Missing dependencies, or multiple versions dependend on implicitly are both
/// considered errors for verify, as are having custom versions in `./INPUT`.
///
/// This function is meant to be a helper for when we want official builds, but also
/// a way to tell developers that they are using things that differ from what jenkins
/// would use.
pub fn verify(m: &Manifest, env: &str, flags: Flags) -> LalResult<()> {
    // 1. Verify that the manifest is sane
    m.verify(flags)?;

    // 2. dependencies in `INPUT` match `manifest.json`.
    if m.dependencies.is_empty() && !input::present() {
        // special case where lal fetch is not required and so INPUT may not exist
        // nothing needs to be verified in this case, so allow missing INPUT
        return Ok(());
    }
    input::verify_dependencies_present(m)?;

    // get data for big verify steps
    let lf = Lockfile::default()
        .with_channel(m.channel.clone())
        .populate_from_input()?;

    // 3. verify the root level dependencies match the manifest
    let simple = flags.contains(Flags::SIMPLE);
    if !simple {
        input::verify_global_versions(&lf, m)?;
    }

    // 4. the dependency tree is flat, and deps use only global deps
    if !simple {
        input::verify_consistent_dependency_versions(&lf, m)?;
    }

    // 5. verify all components are built in the same environment
    input::verify_environment_consistency(&lf, env)?;

    // 6. the channel hierarchy is valid
    if !simple {
        input::verify_global_channels(&lf)?;
    }

    info!("Dependencies fully verified");
    Ok(())
}
