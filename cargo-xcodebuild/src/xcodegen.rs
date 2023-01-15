use anyhow::Context;
use serde::Serialize;
use std::collections::HashMap;

const INHERITED: &str = "$(INHERITED)";

#[derive(Debug, Serialize)]
pub struct Project {
    pub name: String,
    pub configs: HashMap<String, String>,
    pub settings: HashMap<String, serde_yaml::Value>,
    pub options: Options,
    pub targets: HashMap<String, Target>,
}

impl Project {
    pub fn target_name(toml: &crate::toml::Root) -> String {
        toml.lib
            .clone()
            .unwrap_or_default()
            .name
            .unwrap_or_else(|| toml.package.name.clone())
            .replace("-", "_")
    }

    pub fn from_toml(
        manifest_path: &std::path::Path,
        toml: crate::toml::Root,
        team: Option<&crate::teams::Team>,
    ) -> anyhow::Result<Self> {
        log::debug!("Creating xcodegen project from Cargo.toml");

        let ios_metadata = toml.ios_metadata()?;
        let target_name = Self::target_name(&toml);

        let configs = HashMap::from([
            ("release".to_string(), TargetConfig::release()),
            ("debug".to_string(), TargetConfig::debug()),
        ]);
        let settings = HashMap::from([
            (
                "base".to_string(),
                serde_yaml::to_value(TargetSetting::base(&target_name, "../src/"))?,
            ),
            ("configs".to_string(), serde_yaml::to_value(configs)?),
        ]);
        let dependencies = {
            let user_deps = ios_metadata.dependencies.unwrap_or_default();
            if user_deps.is_empty() {
                Vec::new()
            } else {
                let mut deps = Vec::with_capacity(user_deps.len());
                for d in user_deps {
                    deps.push(Dependency::sdk(d));
                }

                deps
            }
        };
        let deployment_target = ios_metadata
            .deployment_target
            .unwrap_or_else(|| "12".into());

        let mut sources_map = vec![serde_yaml::Value::String("../src/".into())];
        let toml_assets = ios_metadata.assets.unwrap_or_default();
        let project_folder = manifest_path.parent().unwrap();
        for asset in toml_assets {
            let full_path = project_folder.join(&asset);
            let source_path = SourcesPath::assets_folder(full_path.to_str().unwrap(), &asset)?;
            let source = serde_yaml::to_value(source_path)?;
            sources_map.push(source);
        }

        let targets = HashMap::from([(
            target_name.clone(),
            Target {
                product_type: ProductType::Application,
                platform: Platform::Ios,
                deployment_target,
                sources: serde_yaml::to_value(sources_map)?,
                settings,
                dependencies,
                info: Default::default(),
                scheme: Default::default(),
            },
        )]);

        let project_settings = {
            let user_code_sign_identity = ios_metadata.code_sign_identity;
            let user_team_id = ios_metadata.development_team;

            let code_sign_identity = user_code_sign_identity.unwrap_or_else(|| {
                if team.is_some() {
                    "iPhone Developer".into()
                } else {
                    "".into()
                }
            });

            let team_id = user_team_id.unwrap_or_else(|| {
                if let Some(team) = team {
                    team.organization_unit.clone()
                } else {
                    "".into()
                }
            });

            if code_sign_identity.is_empty() && team_id.is_empty() {
                // No team, no user info. Disable code sign at all.
                log::debug!("Code sign is disabled");
                HashMap::from([
                    (
                        "CODE_SIGN_IDENTITY".to_string(),
                        serde_yaml::Value::String("".into()),
                    ),
                    (
                        "CODE_SIGNING_REQUIRED".to_string(),
                        serde_yaml::from_str("NO")?,
                    ),
                    (
                        "CODE_SIGN_ENTITLEMENTS".to_string(),
                        serde_yaml::Value::String("".into()),
                    ),
                    (
                        "CODE_SIGNING_ALLOWED".to_string(),
                        serde_yaml::from_str("NO")?,
                    ),
                ])
            } else {
                log::debug!(
                    "Code sign with identity `{}` and team id `{}`.",
                    code_sign_identity,
                    team_id
                );
                HashMap::from([
                    (
                        "CODE_SIGN_STYLE".to_string(),
                        serde_yaml::Value::String("Automatic".into()),
                    ),
                    (
                        "CODE_SIGN_IDENTITY".to_string(),
                        serde_yaml::Value::String(code_sign_identity),
                    ),
                    (
                        "DEVELOPMENT_TEAM".to_string(),
                        serde_yaml::Value::String(team_id),
                    ),
                ])
            }
        };

        let project_configs = HashMap::from([
            ("Debug".to_string(), "debug".to_string()),
            ("Release".to_string(), "release".to_string()),
        ]);
        let options = Options {
            bundle_id_prefix: ios_metadata
                .bundle_id_prefix
                .unwrap_or_else(|| "com.rust".into()),
        };
        let project = Project {
            name: target_name,
            configs: project_configs,
            settings: project_settings,
            options,
            targets,
        };

        Ok(project)
    }

    pub fn write_to(&self, dir: &std::path::Path) -> anyhow::Result<()> {
        let file = std::fs::File::create(dir.join("project.yml"))
            .with_context(|| format!("Failed to create project.yml in {:?}", dir))?;
        let w = std::io::BufWriter::new(file);
        serde_yaml::to_writer(w, &self)?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct Options {
    #[serde(rename(serialize = "bundleIdPrefix"))]
    pub bundle_id_prefix: String,
}

#[derive(Debug, Serialize)]
pub struct Target {
    #[serde(rename(serialize = "type"))]
    pub product_type: ProductType,
    pub platform: Platform,
    #[serde(rename(serialize = "deploymentTarget"))]
    pub deployment_target: String,
    pub sources: serde_yaml::Value,
    pub settings: HashMap<String, serde_yaml::Value>,
    pub dependencies: Vec<Dependency>,
    pub info: Plist,
    pub scheme: TargetScheme,
}

#[derive(Debug, Serialize)]
pub struct Dependency {
    #[serde(flatten)]
    pub dependency_type: DependencyType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed: Option<bool>,
}

impl Dependency {
    pub fn sdk(name: String) -> Self {
        Dependency {
            dependency_type: DependencyType::Sdk(name),
            embed: None,
        }
    }

    #[allow(dead_code)]
    pub fn target(name: String, embed: bool) -> Self {
        Dependency {
            dependency_type: DependencyType::Target(name),
            embed: Some(embed),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Sdk(String),
    #[allow(dead_code)]
    Target(String),
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct TargetSetting {
    #[serde(serialize_with = "bool_to_word")]
    enable_bitcode: bool,
    clang_cxx_language_standard: String,
    clang_cxx_library: String,
    other_ldflags: Vec<String>,
    header_search_paths: Vec<String>,
}

impl TargetSetting {
    pub fn base(lib_name: &str, header_path: &str) -> Self {
        Self {
            enable_bitcode: false,
            header_search_paths: vec![INHERITED.into(), header_path.into()],
            clang_cxx_language_standard: "c++11".into(),
            clang_cxx_library: "libc++".into(),
            other_ldflags: vec![
                INHERITED.into(),
                "-lc++abi".into(),
                format!("-l{}", lib_name),
            ],
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct TargetConfig {
    #[serde(rename = "LIBRARY_SEARCH_PATHS[sdk=iphoneos*]")]
    library_search_paths_arm: Vec<String>,
    #[serde(rename = "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=arm64]")]
    library_search_paths_arm_sim: Vec<String>,
    #[serde(rename = "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=x86_64]")]
    library_search_paths_x86: Vec<String>,
}

impl TargetConfig {
    pub fn release() -> Self {
        Self {
            library_search_paths_arm: vec![
                INHERITED.into(),
                "../../aarch64-apple-ios/release".into(),
            ],
            library_search_paths_arm_sim: vec![
                INHERITED.into(),
                "../../aarch64-apple-ios-sim/release".into(),
            ],
            library_search_paths_x86: vec![
                INHERITED.into(),
                "../../x86_64-apple-ios/release".into(),
            ],
        }
    }

    pub fn debug() -> Self {
        Self {
            library_search_paths_arm: vec![
                INHERITED.into(),
                "../../aarch64-apple-ios/debug".into(),
            ],
            library_search_paths_arm_sim: vec![
                INHERITED.into(),
                "../../aarch64-apple-ios-sim/debug".into(),
            ],
            library_search_paths_x86: vec![INHERITED.into(), "../../x86_64-apple-ios/debug".into()],
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SourcesPath {
    pub path: String,
    #[serde(rename = "buildPhase")]
    pub build_phase: HashMap<String, serde_yaml::Value>,
}

impl SourcesPath {
    pub fn assets_folder(full_path: &str, dest_path: &str) -> anyhow::Result<Self> {
        let mut copy_files: HashMap<serde_yaml::Value, serde_yaml::Value> = HashMap::new();
        copy_files.insert(
            serde_yaml::Value::String("destination".to_string()),
            serde_yaml::Value::String("resources".to_string()),
        );
        copy_files.insert(
            serde_yaml::Value::String("subpath".to_string()),
            serde_yaml::Value::String(dest_path.to_string()),
        );
        let build_phase =
            HashMap::from([("copyFiles".to_string(), serde_yaml::to_value(copy_files)?)]);

        Ok(Self {
            path: full_path.to_string(),
            build_phase,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Plist {
    path: String,
    properties: HashMap<String, serde_yaml::Value>,
}

impl Default for Plist {
    fn default() -> Self {
        Self {
            path: "../src/Info.plist".into(),
            properties: HashMap::from([(
                "UILaunchStoryboardName".to_string(),
                serde_yaml::Value::String("LaunchScreen".into()),
            )]),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TargetScheme {
    #[serde(rename(serialize = "environmentVariables"))]
    pub environment_variables: Vec<EnvironmentVariable>,
}

impl Default for TargetScheme {
    fn default() -> Self {
        Self {
            environment_variables: vec![
                EnvironmentVariable {
                    variable: "RUST_BACKTRACE".into(),
                    value: "full".into(),
                    is_enabled: true,
                },
                EnvironmentVariable {
                    variable: "RUST_LOG".into(),
                    value: "info".into(),
                    is_enabled: true,
                },
            ],
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EnvironmentVariable {
    pub variable: String,
    pub value: String,
    #[serde(rename(serialize = "isEnabled"))]
    pub is_enabled: bool,
}

#[derive(Debug, Serialize)]
pub enum ProductType {
    #[serde(rename(serialize = "application"))]
    Application,
    #[serde(rename(serialize = ""))]
    #[allow(dead_code)]
    None,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub enum Platform {
    #[serde(rename = "iOS")]
    Ios,
    MacOS,
    TvOS,
    WatchOS,
}

fn bool_to_word<S>(value: &bool, ser: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    if *value {
        ser.serialize_str("YES")
    } else {
        ser.serialize_str("NO")
    }
}
