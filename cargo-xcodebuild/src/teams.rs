// Inspired by https://github.com/BrainiumLLC/cargo-mobile/blob/master/src/apple/teams.rs

use anyhow::Context as _;
use std::collections::BTreeSet;
use x509_parser::prelude::*;

fn get_pem_list(name_substr: &str) -> anyhow::Result<std::process::Output> {
    let args = ["find-certificate", "-p", "-a", "-c", name_substr];
    let output = std::process::Command::new("security")
        .args(&args)
        .output()
        .with_context(|| format!("Failed to run security utility with args: {:?}", &args))?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        anyhow::bail!(
            "Failed to run security utility with args: {:?}.\n{:?}\n{:?}",
            &args,
            &stdout,
            &stderr,
        )
    }

    Ok(output)
}

fn get_pem_list_old_name_scheme() -> anyhow::Result<Vec<u8>> {
    Ok(get_pem_list("Developer:")
        .with_context(|| "Failed to get pem list with substring `Developer:`")?
        .stdout)
}

fn get_pem_list_new_name_scheme() -> anyhow::Result<Vec<u8>> {
    Ok(get_pem_list("Development:")
        .with_context(|| "Failed to get pem list with substring `Development:`")?
        .stdout)
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Team {
    pub common_name: String,
    pub organization: String,
    pub organization_unit: String,
}

impl Team {
    #[allow(clippy::unnecessary_unwrap)]
    pub fn from_pem(pem: &Pem) -> anyhow::Result<Option<Self>> {
        log::debug!("Creating team from pem cert");
        if let Ok(cert) = pem.parse_x509() {
            if !cert.validity().is_valid() {
                log::debug!("Invalid cert");
                return Ok(None);
            }

            log::debug!("Trying to get team info from x509 cert");
            let subj = cert.subject();
            let common_names = subj
                .iter_common_name()
                .map(|attr| attr.as_str())
                .collect::<Result<Vec<_>, X509Error>>()
                .with_context(|| "Failed to collect cert common name".to_string())?;
            let common_name = common_names.get(0);

            let organizations = subj
                .iter_organization()
                .map(|attr| attr.as_str())
                .collect::<Result<Vec<_>, X509Error>>()
                .with_context(|| "Failed to collect cert organization name".to_string())?;
            let organization = organizations.get(0);

            let organization_units = subj
                .iter_organizational_unit()
                .map(|attr| attr.as_str())
                .collect::<Result<Vec<_>, X509Error>>()
                .with_context(|| "Failed to collect cert organization unit".to_string())?;

            let organization_unit = organization_units.get(0);

            if common_name.is_some() && organization.is_some() && organization_unit.is_some() {
                log::debug!(
                    "Found cert {:?} with organization {:?}",
                    common_name.unwrap(),
                    organization.unwrap()
                );
                Ok(Some(Self {
                    common_name: common_name.unwrap().to_string(),
                    organization: organization.unwrap().to_string(),
                    organization_unit: organization_unit.unwrap().to_string(),
                }))
            } else {
                log::debug!("Failed to get team info");
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

pub fn find_development_teams() -> Vec<Team> {
    let pems = {
        let mut pems = vec![];

        if let Ok(new) = get_pem_list_new_name_scheme() {
            for pem in Pem::iter_from_buffer(&new).flatten() {
                pems.push(pem);
            }
        }

        if let Ok(old) = get_pem_list_old_name_scheme() {
            for pem in Pem::iter_from_buffer(&old).flatten() {
                pems.push(pem);
            }
        }

        pems
    };

    pems.into_iter()
        .flat_map(|cert| Team::from_pem(&cert))
        .flatten()
        // Silly way to sort this and ensure no dupes
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
