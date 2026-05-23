use crate::prelude::*;

#[derive(Debug)]
pub struct JsonShhhProvider {
    entries: Vec<ShhhFsEntry>,
}

impl JsonShhhProvider {
    pub fn from_options(options: &str) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(options).context("failed to parse json provider options")?;
        let object = value
            .as_object()
            .ok_or_else(|| anyhow!("json provider options must be a JSON object"))?;

        let mut entries = Vec::with_capacity(object.len());

        for (name, value) in object {
            if name.is_empty() || name == "." || name == ".." || name.contains('/') {
                return Err(anyhow!(
                    "JSON provider keys must be flat file names, got {:?}",
                    name
                ));
            }

            let contents = match value {
                serde_json::Value::String(value) => value.as_bytes().to_vec(),
                value => serde_json::to_string_pretty(value)?.into_bytes(),
            };

            entries.push(ShhhFsEntry {
                name: name.clone(),
                contents,
            });
        }

        entries.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(Self { entries })
    }
}

impl ShhhFsProvider for JsonShhhProvider {
    fn entries(&self) -> &[ShhhFsEntry] {
        &self.entries
    }
}
