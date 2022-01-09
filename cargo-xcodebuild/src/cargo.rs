use anyhow::Context as _;

pub fn run_cargo_subcommand(cmd: &cargo_subcommand::Subcommand) -> anyhow::Result<()> {
    run_cargo(cmd.cmd(), cmd.args(), None)?;

    Ok(())
}

pub fn run_cargo(cmd: &str, args: &[String], target: Option<&str>) -> anyhow::Result<()> {
    let mut command = std::process::Command::new("cargo");
    command.arg(cmd);
    if let Some(t) = target {
        command.arg("--target").arg(t);
    }
    command
        .args(args)
        .status()
        .with_context(|| format!("Failed to run cargo {} with args: {:?}", cmd, args,))?;
    Ok(())
}
