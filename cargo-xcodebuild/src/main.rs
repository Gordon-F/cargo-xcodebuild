use anyhow::Context;

mod cargo;
mod devices;
mod source;
mod teams;
mod toml;
mod xcodebuild;
mod xcodegen;

fn print_help() {
    println!(
        r#"cargo-xcodebuild
Helps cargo build and run apps for iOS

USAGE:
    cargo xcodebuild [SUBCOMMAND]

SUBCOMMAND:
    check, c     Checks that the current package builds without creating xcodeproject
    build, b     Compiles the current package and create xcodeproject
    run, r       Run a project on device or simulator
    generate, g  Generate xcodeproject without building it
    open, o      Open generated project with Xcode
    devices, d   List of booted simulator and connected devices
    teams, t     List of signing teams
    boot [id]    Boot a simulator with specific device id
"#
    );
}

#[cfg(not(target_os = "macos"))]
fn main() {
    compile_error!("Unfortunately, only MacOS is supported.");
}

#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    env_logger::try_init().with_context(|| "Failed to init env_logger".to_string())?;

    let args = std::env::args();
    let cmd = cargo_subcommand::Subcommand::new(args, "xcodebuild", |_, _| Ok(false));
    if cmd.is_err() {
        print_help();
        std::process::exit(0);
    }
    let cmd = cmd?;
    let mut xcode_build = xcodebuild::Xcodebuild::new(cmd.manifest(), cmd.target_dir())?;
    let build_type = match *cmd.profile() {
        cargo_subcommand::Profile::Dev => crate::xcodebuild::BuildType::Debug,
        cargo_subcommand::Profile::Release => crate::xcodebuild::BuildType::Release,
        cargo_subcommand::Profile::Custom(ref str) => {
            log::warn!("Custrom Profile: `{}`. Return debug build type", str);
            crate::xcodebuild::BuildType::Debug
        }
    };

    match cmd.cmd() {
        "check" | "c" => {
            xcode_build.check(cmd.args())?;
        }
        "build" | "b" => {
            xcode_build.build(cmd.args(), build_type)?;
        }
        "run" | "r" => {
            xcode_build.build(cmd.args(), build_type)?;
            xcode_build.run(build_type)?;
        }
        "generate" | "g" => {
            xcode_build.generate_project()?;
        }
        "devices" | "d" => {
            let simulators = xcodebuild::Xcodebuild::get_simulator_device_list()?;
            println!("Booted simulators: ");
            for device in simulators {
                if device.state == crate::devices::DeviceState::Booted {
                    println!("{:?}", device);
                }
            }

            let devices = md::get_device_list();
            println!("Connected devices: {}", devices.len());
            for device in devices {
                println!("{:?}", device);
            }
        }
        "teams" | "t" => {
            println!("Signing teams:");
            let teams = teams::find_development_teams();
            for t in teams {
                println!("{:?}", t);
            }
        }
        "boot" => {
            if let Some(arg) = cmd.args().get(0) {
                xcode_build.boot_simulator(arg)?;
            } else {
                println!("Simulator device id is required. List of avaliable devices:");
                let devices = xcodebuild::Xcodebuild::get_simulator_device_list()?;
                for d in devices {
                    println!("{:?}", d)
                }
                std::process::exit(1);
            }
        }
        "open" | "o" => xcode_build.open_xcode()?,
        "--help" => {
            if let Some(arg) = cmd.args().get(0) {
                match &**arg {
                    "build" | "b" | "check" | "c" | "run" | "r" | "test" | "t" | "doc" => {
                        cargo::run_cargo_subcommand(&cmd)?;
                    }
                    _ => print_help(),
                }
            } else {
                print_help();
            }
        }
        _ => print_help(),
    }

    Ok(())
}
