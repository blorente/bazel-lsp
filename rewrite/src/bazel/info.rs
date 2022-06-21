use eyre::{eyre, Result};
use std::{collections::HashMap, path::PathBuf};

use super::bazel::BazelExecutable;

pub(crate) struct BazelInfo {
    repository_cache: PathBuf,
}

impl BazelInfo {
    pub(crate) fn new(bazel: &BazelExecutable, workspace_root: &PathBuf) -> Result<Self> {
        let output = bazel.call_bazel(vec!["info".to_string()], workspace_root)?;
        let all_info = BazelInfo::parse_bazel_info_output(&output)?;

        Ok(BazelInfo {
            repository_cache: PathBuf::from(BazelInfo::read_field("repository_cache", &all_info)?),
        })
    }

    fn read_field(name: &str, fields: &HashMap<String, String>) -> Result<String> {
        fields
            .get(name)
            .ok_or(eyre!(
                "Field {} not found in the fields of BazelInfo({:?})",
                name,
                fields.keys()
            ))
            .map(|s| s.to_string())
    }

    fn parse_bazel_info_output(output: &String) -> Result<HashMap<String, String>> {
        let mut map: HashMap<String, String> = HashMap::new();
        let parts: Vec<Vec<&str>> = output
            .split("\n")
            .map(|line: &str| line.split(":").collect())
            .collect();
        for part in parts {
            let field = part.get(0).ok_or(eyre!("Failed to parse field!"))?;
            let value = part.get(1).ok_or(eyre!("Failed to parse value!"))?;
            map.insert(field.to_string(), value.to_string());
        }
        Ok(map)
    }
}
