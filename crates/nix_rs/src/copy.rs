use crate::command::{CommandError, NixCmd};

/// Runs `nix copy` in Rust
pub async fn run_nix_copy(
    cmd: &NixCmd,
    host: &str,
    omnix_input: &str,
    path: &str,
    extra_args: &[String],
) -> Result<(), CommandError> {
    let remote_address = format!("ssh://{}", host);

    // base arguments for nix copy command
    let mut args = vec!["copy", "--to", &remote_address, omnix_input, path];

    // Filter and add only the --override-input flags and their values
    let override_inputs: Vec<&str> = extra_args
        .iter()
        .enumerate()
        .filter_map(|(i, arg)| {
            if arg == "--override-input" && i + 2 < extra_args.len() {
                Some(&extra_args[i..i + 3])
            } else {
                None
            }
        })
        .flatten()
        .map(AsRef::as_ref)
        .collect();

    args.extend(override_inputs);

    // Run the command with all arguments
    cmd.run_with_args(&args).await?;

    Ok(())
}
