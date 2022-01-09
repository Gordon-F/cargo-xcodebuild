use anyhow::Context as _;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Root {
    pub package: Package,
    pub lib: Option<Lib>,
}

impl Root {
    pub fn validate_crate_type(self) -> anyhow::Result<Self> {
        let crate_types = self
            .lib
            .clone()
            .unwrap_or_default()
            .crate_type
            .unwrap_or_default()
            .join(" ");
        if !crate_types.contains("staticlib") {
            anyhow::bail!(
                "Missing `staticlib` crate-type in `lib` section. Please check Cargo.toml."
            )
        }

        Ok(self)
    }

    pub fn validate_build_targets(self) -> anyhow::Result<Self> {
        let ios_metadata = self.ios_metadata()?;
        let targets = ios_metadata.build_targets.unwrap_or_default();
        if targets.is_empty() {
            anyhow::bail!("Missing `build_targets` in `package.metadata.ios` section. Please check Cargo.toml.")
        }

        Ok(self)
    }

    pub fn ios_metadata(&self) -> anyhow::Result<IosMetadata> {
        self.package
            .metadata
            .clone()
            .with_context(|| "Missing `metadata` section. Please check Cargo.toml.".to_string())?
            .ios
            .with_context(|| "Missing `ios` section. Please check Cargo.toml.".to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub metadata: Option<PackageMetadata>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Lib {
    pub name: Option<String>,
    #[serde(rename = "crate-type")]
    pub crate_type: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PackageMetadata {
    pub ios: Option<IosMetadata>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct IosMetadata {
    pub build_targets: Option<Vec<Target>>,
    pub deployment_target: Option<String>,
    pub dependencies: Option<Vec<String>>,
    //TODO: unused now
    pub supported_interface_orientations: Option<Vec<Orientation>>,
    pub bundle_id_prefix: Option<String>,
    pub code_sign_identity: Option<String>,
    pub development_team: Option<String>,
    pub device_id: Option<String>,
    pub device_type: Option<DeviceType>,
    pub assets: Option<Vec<String>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Device,
    Simulator,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum Orientation {
    #[serde(rename = "UIInterfaceOrientationUnknown")]
    Unknown,
    #[serde(rename = "UIInterfaceOrientationPortrait")]
    Portrait,
    #[serde(rename = "UIInterfaceOrientationPortraitUpsideDown")]
    PortraitUpsideDown,
    #[serde(rename = "UIInterfaceOrientationLandscapeLeft")]
    LandscapeLeft,
    #[serde(rename = "UIInterfaceOrientationLandscapeRight")]
    LandscapeRight,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum Target {
    #[serde(rename = "aarch64-apple-ios")]
    Arm,
    #[serde(rename = "aarch64-apple-ios-sim")]
    ArmSim,
    #[serde(rename = "x86_64-apple-ios")]
    Sim,
}

impl Target {
    pub fn to_rustc_target(self) -> &'static str {
        match self {
            Target::Arm => "aarch64-apple-ios",
            Target::ArmSim => "aarch64-apple-ios-sim",
            Target::Sim => "x86_64-apple-ios",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_toml() {
        let contents = r#"[package]
        name = "xcodebuild_examples"
        version = "0.1.0"
        edition = "2021"
        license = "MIT OR Apache-2.0"

        [lib]
        crate-type = ["staticlib", "cdylib"]

        [dependencies]


        [package.metadata.ios]
        build_targets = [ "aarch64-apple-ios", "aarch64-apple-ios-sim"]
        "#;
        let toml: Root = toml::from_str(&contents).unwrap();
        let ios_metadata = toml
            .package
            .metadata
            .unwrap_or_default()
            .ios
            .unwrap_or_default();

        assert_eq!(
            toml.lib.unwrap_or_default().crate_type.unwrap_or_default(),
            vec!["staticlib", "cdylib"]
        );
        assert_eq!(
            ios_metadata.build_targets.unwrap_or_default(),
            vec![Target::Arm, Target::ArmSim]
        );
    }
}
