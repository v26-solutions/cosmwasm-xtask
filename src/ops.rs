use xshell::{cmd, Shell};

use crate::Error;

/// Build and optimize all contract crates in `<workspace-root>/contracts` using the `cosmwasm/workspace-optimizer` docker image.
/// Artifacts are placed in `<workspace-root>/artifacts` by default, this can be overridden by setting the `COSMWASM_ARTIFACTS_DIR` environment variable.
///
/// # Errors
///
/// This function will return an error if:
/// - Creating the artifacts directory if it does not exist fails
/// - Running the docker command fails
pub fn dist_workspace(sh: &Shell) -> Result<(), Error> {
    let cwd = sh.current_dir().canonicalize()?;

    let cwd_path = cwd.as_path();

    let cwd_name = cwd.file_stem().unwrap();

    let artifacts_dir =
        std::env::var("COSMWASM_ARTIFACTS_DIR").unwrap_or_else(|_| "artifacts".to_owned());

    if !sh.path_exists(&artifacts_dir) {
        cmd!(sh, "mkdir {artifacts_dir}").run()?;
    }

    cmd!(
        sh,
        "docker run --rm -v {cwd_path}:/code
          --mount type=volume,source={cwd_name}_cache,target=/code/target
          --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry
          cosmwasm/workspace-optimizer:0.14.0"
    )
    .run()?;

    Ok(())
}
