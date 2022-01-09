use anyhow::Context as _;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub enum SelectedDevice {
    Device(md::MobileDevice),
    Simulator { udid: String },
}

#[derive(Debug, Clone, Copy)]
pub enum BuildType {
    Debug,
    Release,
}
pub struct Xcodebuild {
    manifest_path: PathBuf,
    target_dir: PathBuf,
    project_dir: Option<PathBuf>,
    src_dir: PathBuf,
    toml_content: String,
    app_name: Option<String>,
    bundle_prefix: Option<String>,
    device: Option<SelectedDevice>,
}

impl Xcodebuild {
    pub fn new(manifest_path: &Path, target_dir: &Path) -> anyhow::Result<Self> {
        let toml_content = std::fs::read_to_string(manifest_path)?;
        let target_dir = target_dir.join("xcodegen");
        let src_dir = target_dir.join("src");
        log::trace!("cargo-xcodebuild target dir: {:?}", target_dir);
        Ok(Self {
            manifest_path: manifest_path.to_path_buf(),
            target_dir,
            project_dir: None,
            src_dir,
            toml_content,
            app_name: None,
            bundle_prefix: None,
            device: None,
        })
    }

    pub fn check(&self, args: &[String]) -> anyhow::Result<()> {
        let toml =
            toml::from_str::<crate::toml::Root>(&self.toml_content)?.validate_build_targets()?;

        let build_targets = toml
            .package
            .metadata
            .unwrap_or_default()
            .ios
            .unwrap_or_default()
            .build_targets
            .unwrap_or_default();

        for target in build_targets {
            crate::cargo::run_cargo("check", args, Some(target.to_rustc_target()))?;
        }

        Ok(())
    }

    pub fn build(&mut self, args: &[String], build_type: BuildType) -> anyhow::Result<()> {
        log::info!("Building project");
        Self::check_xcodegen()?;

        let toml = toml::from_str::<crate::toml::Root>(&self.toml_content)?
            .validate_crate_type()?
            .validate_build_targets()?;

        let build_targets = toml.ios_metadata()?.build_targets.unwrap_or_default();

        for target in build_targets {
            log::info!("Build project for target: {}", target.to_rustc_target());
            crate::cargo::run_cargo("build", args, Some(target.to_rustc_target()))?;
        }
        let teams = crate::teams::find_development_teams();
        let team = teams.get(0);
        log::info!("Selected signing team: {:?}", team);
        let app_name = crate::xcodegen::Project::target_name(&toml);
        let project = crate::xcodegen::Project::from_toml(&self.manifest_path, toml, team)?;

        let project_dir = self.target_dir.join(&app_name);

        self.prepare_target_dir(&project_dir)?;
        self.generate_xcode_project(&project, &project_dir)?;

        self.project_dir = Some(project_dir);

        let bundle_prefix = project.options.bundle_id_prefix;

        let selected_device = self.find_device()?;
        self.xcode_build_project(&selected_device, build_type, &app_name)?;

        self.app_name = Some(app_name);
        self.bundle_prefix = Some(bundle_prefix);
        self.device = Some(selected_device);

        Ok(())
    }

    pub fn run(&self, build_type: BuildType) -> anyhow::Result<()> {
        log::info!("Running project");

        let bundle_id_prefix = self.bundle_prefix.as_ref().unwrap();
        let app_name = self.app_name.as_ref().unwrap();
        let selected_device = self.device.as_ref().unwrap();

        let app_path_str = Self::build_app_path(selected_device, build_type, app_name);
        let full_app_name = format!("{}.{}", bundle_id_prefix, app_name.replace("_", "-"));

        log::debug!("{} path: {}", full_app_name, app_path_str);

        match *selected_device {
            SelectedDevice::Device(ref md) => {
                log::info!(
                    "Installing app `{}` {} on connected device {}",
                    full_app_name,
                    app_path_str,
                    md.identifier
                );

                let project_dir = self.project_dir.as_ref().unwrap();
                md.install_app(&project_dir.join(app_path_str))?;
                println!(
                    "{}.{} is installed to device {}. Please run it.",
                    bundle_id_prefix, app_name, md.identifier
                );
            }
            SelectedDevice::Simulator { ref udid } => {
                self.install_app_to_simulator(&app_path_str, udid)?;
                Self::run_app_with_simulator(udid, &full_app_name)?;
            }
        }

        Ok(())
    }

    pub fn generate_project(&self) -> anyhow::Result<()> {
        log::info!("Generating xcodeproject");

        let toml =
            toml::from_str::<crate::toml::Root>(&self.toml_content)?.validate_crate_type()?;

        let teams = crate::teams::find_development_teams();
        let team = teams.get(0);
        log::info!("Selected signing team: {:?}", team);
        let app_name = crate::xcodegen::Project::target_name(&toml);
        let project = crate::xcodegen::Project::from_toml(&self.manifest_path, toml, team)?;

        let project_dir = self.target_dir.join(&app_name);
        self.prepare_target_dir(&project_dir)?;
        self.generate_xcode_project(&project, &project_dir)?;

        Ok(())
    }

    pub fn boot_simulator(&self, device_id: &str) -> anyhow::Result<()> {
        let output = Command::new("xcrun")
            .arg("simctl")
            .arg("boot")
            .arg(device_id)
            .output()
            .with_context(|| {
                format!(
                    "Failed to get output from command: xcrun simctl boot {}",
                    device_id
                )
            })?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            anyhow::bail!(
                "Failed to boot simulator with id: {}\n{:?}\n{:?}",
                &device_id,
                &stdout,
                &stderr,
            )
        }

        let output = Command::new("open")
            .arg("-a")
            .arg("Simulator.app")
            .output()
            .with_context(|| "Failed to open Simulator.app".to_string())?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            anyhow::bail!(
                "Failed to open Simulator.app: {}\n{:?}\n{:?}",
                &device_id,
                &stdout,
                &stderr,
            )
        }

        Ok(())
    }

    pub fn open_xcode(&self) -> anyhow::Result<()> {
        log::trace!("Opening xcode project");
        // First searching for provided manifest path. Then search Cargo.toml in cwd.
        let target_dir_str = self.target_dir.to_str().unwrap();
        let provided_toml = toml::from_str::<crate::toml::Root>(&self.toml_content)?;
        let provided_project_path_str = Self::get_xcode_project_path(target_dir_str, provided_toml);
        let provided_project_path = std::path::PathBuf::from(&provided_project_path_str);

        if provided_project_path.exists() && provided_project_path.is_dir() {
            Self::open_xcode_project(&provided_project_path_str)?;
        } else {
            log::debug!("xcodeproj not exists {:?}", provided_project_path_str);
            let cwd =
                std::env::current_dir().with_context(|| "Failed to get cwd when open xcode")?;
            log::trace!("Searching Cargo.toml in current directory: {:?}", cwd);
            let possible_manifest_path = cwd.join("Cargo.toml");
            if possible_manifest_path.exists() && possible_manifest_path.is_file() {
                let toml = toml::from_str::<crate::toml::Root>(&self.toml_content)?;
                let project_path_str = Self::get_xcode_project_path(target_dir_str, toml);
                let project_path = std::path::PathBuf::from(&project_path_str);
                if project_path.exists() && project_path.is_dir() {
                    Self::open_xcode_project(&project_path_str)?;
                } else {
                    log::debug!("xcodeproj not exists {:?}", project_path_str);
                    anyhow::bail!("Can't find xcodeproj. Build it first")
                }
            } else {
                anyhow::bail!("Can't find xcodeproj. Build it first")
            }
        }

        Ok(())
    }

    pub fn check_xcodegen() -> anyhow::Result<()> {
        log::debug!("Checking xcodegen version");
        let xcodegen_version_output = Command::new("xcodegen")
            .arg("version")
            .output()
            .with_context(|| "Failed to get output".to_string())?;

        if xcodegen_version_output.status.success() {
            let stdout = String::from_utf8_lossy(&xcodegen_version_output.stdout);
            if stdout.starts_with("Version:") {
                log::info!("Xcodegen {}", stdout);
            } else {
                anyhow::bail!("Xcodegen is not found")
            }
        } else {
            anyhow::bail!("Xcodegen is not found")
        }

        Ok(())
    }

    pub fn get_simulator_device_list() -> anyhow::Result<Vec<crate::devices::SimulatorDevice>> {
        let output = std::process::Command::new("xcrun")
            .arg("simctl")
            .arg("list")
            .arg("devices")
            .arg("iOS")
            .arg("--json")
            .output()
            .with_context(|| "Failed to get iOS simulators list".to_string())?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Failed to get booted simulators:\n{:?}\n{:?}",
                &stdout,
                &stderr
            )
        } else {
            let devices: crate::devices::SimulatorDevices = serde_json::from_str(&stdout)
                .with_context(|| {
                    format!(
                        "Failed to create typed device list from output:\n{}",
                        stdout
                    )
                })?;

            Ok(devices
                .ios()
                .with_context(|| "Failed to get ios device list")?)
        }
    }

    fn get_xcode_project_path(target_dir: &str, toml: crate::toml::Root) -> String {
        let app_name = toml
            .lib
            .unwrap_or_default()
            .name
            .unwrap_or(toml.package.name)
            .replace("-", "_");
        format!("{}/{}/{}.xcodeproj", target_dir, app_name, app_name)
    }

    fn open_xcode_project(project_path: &str) -> anyhow::Result<()> {
        log::trace!("Opening xcode project: {}", project_path);
        let output = Command::new("open")
            .arg(project_path)
            .output()
            .with_context(|| format!("Failed to open xcodeproject: {}", project_path))?;

        if !output.status.success() {
            anyhow::bail!("Failed to open xcodeproject: {}", project_path)
        }
        Ok(())
    }

    #[allow(clippy::unnecessary_unwrap)]
    fn find_device(&self) -> anyhow::Result<SelectedDevice> {
        log::debug!("Finding device");
        let toml =
            toml::from_str::<crate::toml::Root>(&self.toml_content)?.validate_build_targets()?;
        let toml_device_id = toml.ios_metadata().unwrap_or_default().device_id;
        let toml_device_type = toml.ios_metadata().unwrap_or_default().device_type;

        if toml_device_id.is_some() && toml_device_type.is_some() {
            let device_id = toml_device_id.unwrap();
            let is_simulator = toml_device_type.unwrap() == crate::toml::DeviceType::Simulator;
            log::info!("Device is specified in Cargo.toml {}", device_id);

            if is_simulator {
                Ok(SelectedDevice::Simulator { udid: device_id })
            } else if let Some(md) = md::get_device_list()
                .iter()
                .find(|e| e.identifier == device_id)
            {
                Ok(SelectedDevice::Device(md.clone()))
            } else {
                anyhow::bail!(
                    "Failed to find a connected device with specified id {}",
                    device_id
                )
            }
        } else {
            // Let's check connected device first, then simulators
            let connected_devices = md::get_device_list();
            if !connected_devices.is_empty() {
                Ok(SelectedDevice::Device(connected_devices[0].clone()))
            } else {
                log::info!("Failed to find connected device. Searching a booted simulator");
                let devices = Self::get_simulator_device_list().unwrap_or_default();
                if let Some(d) = devices
                    .iter()
                    .find(|d| d.state == crate::devices::DeviceState::Booted)
                {
                    Ok(SelectedDevice::Simulator {
                        udid: d.udid.clone(),
                    })
                } else {
                    anyhow::bail!("Failed to get device id")
                }
            }
        }
    }

    fn build_app_path(device: &SelectedDevice, build_type: BuildType, app_name: &str) -> String {
        let base_path = "build/Build/Products/";
        let configuration = match build_type {
            BuildType::Debug => "Debug",
            BuildType::Release => "Release",
        };
        let device_type = match *device {
            SelectedDevice::Device(_) => "iphoneos",
            SelectedDevice::Simulator { .. } => "iphonesimulator",
        };
        format!(
            "{}{}-{}/{}.app",
            base_path, configuration, device_type, app_name,
        )
    }

    fn xcode_build_project(
        &self,
        device: &SelectedDevice,
        build_type: BuildType,
        scheme_name: &str,
    ) -> anyhow::Result<()> {
        let (platform, device_id) = match *device {
            SelectedDevice::Simulator { ref udid } => ("iOS Simulator", udid),
            SelectedDevice::Device(ref md) => ("iOS", &md.identifier),
        };
        let configuration = match build_type {
            BuildType::Debug => "Debug",
            BuildType::Release => "Release",
        };
        let destination = format!("platform={},id={}", platform, device_id);
        log::info!(
            "Building {} scheme {} for destination: {}",
            configuration,
            scheme_name,
            destination,
        );
        let base_args = [
            "-derivedDataPath",
            "build",
            "-scheme",
            scheme_name,
            "-configuration",
            configuration,
            "-allowProvisioningUpdates",
        ];
        let additional_args = match *device {
            SelectedDevice::Device(ref md) => {
                let arch = if md.cpu_architecture == "arm64e" {
                    "arm64"
                } else {
                    &md.cpu_architecture
                };
                vec!["-sdk", "iphoneos", "-arch", arch]
            }
            SelectedDevice::Simulator { .. } => vec!["-destination", &destination],
        };

        let project_dir = self.project_dir.as_ref().unwrap();
        let mut command = std::process::Command::new("xcodebuild");
        command
            .current_dir(project_dir)
            .args(base_args)
            .args(additional_args);
        log::trace!("cwd: {:?}", project_dir);
        log::trace!("xcodebuild command: {:?}", command);
        let output = command
            .output()
            .with_context(|| "Failed to get xcodebuild output".to_string())?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            anyhow::bail!(
                "Failed to build project with xcodebuild:\n{:?}\n{:?}",
                stdout,
                stderr
            )
        };

        Ok(())
    }

    fn install_app_to_simulator(&self, app_path: &str, device_id: &str) -> anyhow::Result<()> {
        log::info!("Installing app {} on simulator {}", app_path, device_id);
        let project_dir = self.project_dir.as_ref().unwrap();
        let output = std::process::Command::new("xcrun")
            .current_dir(project_dir)
            .arg("simctl")
            .arg("install")
            .arg(device_id)
            .arg(app_path)
            .output()
            .with_context(|| "Failed to get xcrun output".to_string())?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            anyhow::bail!("Failed to install app:\n{:?}\n{:?}", stdout, stderr)
        };

        Ok(())
    }

    fn run_app_with_simulator(device_id: &str, full_app_name: &str) -> anyhow::Result<()> {
        log::info!("Running app {} on simulator {}", full_app_name, device_id);
        let output = std::process::Command::new("xcrun")
            .arg("simctl")
            .arg("launch")
            .arg(device_id)
            .arg(full_app_name)
            .output()
            .with_context(|| "Failed to get xcrun output".to_string())?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            anyhow::bail!("Failed to run app:\n{:?}\n{:?}", stdout, stderr)
        };

        Ok(())
    }

    fn prepare_target_dir(&self, project_dir: &Path) -> anyhow::Result<()> {
        log::debug!("Creating required target dirs");
        std::fs::create_dir_all(&self.src_dir)
            .with_context(|| format!("Failed to create all dir: {:?}", &self.src_dir))?;

        std::fs::create_dir_all(project_dir)
            .with_context(|| format!("Failed to create all dir: {:?}", project_dir))?;

        Ok(())
    }

    fn generate_xcode_project(
        &self,
        project: &crate::xcodegen::Project,
        project_dir: &Path,
    ) -> anyhow::Result<()> {
        log::debug!("Writing all files required for xcodegen");
        project.write_to(project_dir)?;
        std::fs::write(
            self.src_dir.join("bindings.h"),
            crate::source::DEFAULT_BINDING_HEADER,
        )
        .with_context(|| format!("Failed to write bindings.h: {:?}", &self.src_dir))?;
        std::fs::write(
            self.src_dir.join("main.m"),
            crate::source::DEFAULT_MAIN_FILE,
        )
        .with_context(|| format!("Failed to write main.m:{:?}", &self.src_dir))?;

        log::info!("Generating xcode project");
        let output = std::process::Command::new("xcodegen")
            .arg("--use-cache")
            .current_dir(project_dir)
            .output()
            .with_context(|| "Failed to get output".to_string())?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            anyhow::bail!(
                "Failed to generate xcode project:\n{:?}\n{:?}",
                stdout,
                stderr
            )
        };

        Ok(())
    }
}
