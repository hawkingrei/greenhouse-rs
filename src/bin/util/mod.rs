pub(crate) mod signal_handler;

/// Returns the tikv version information.
pub fn greenhouse_version_info() -> String {
    let fallback = "Unknown (env var does not exist when building)";
    format!(
        "\nRelease Version:   {}\
         \nGit Commit Hash:   {}\
         \nGit Commit Branch: {}\
         \nUTC Build Time:    {}\
         \nRust Version:      {}",
        env!("CARGO_PKG_VERSION"),
        option_env!("GREENHOUSE_BUILD_GIT_HASH").unwrap_or(fallback),
        option_env!("GREENHOUSE_BUILD_GIT_BRANCH").unwrap_or(fallback),
        option_env!("GREENHOUSE_BUILD_TIME").unwrap_or(fallback),
        option_env!("GREENHOUSE_BUILD_RUSTC_VERSION").unwrap_or(fallback),
    )
}
