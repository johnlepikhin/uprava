use anyhow::{bail, Result};

pub fn json_de_kv<DATA>(
    map: &std::collections::BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<DATA>
where
    for<'de> DATA: serde::Deserialize<'de>,
{
    println!("Getting {:?}", key);
    match map.get(key) {
        None => bail!("Field {:?} not found", key),
        Some(value) => {
            let r = serde_json::value::from_value(value.clone())?;
            Ok(r)
        }
    }
}

pub fn json_de_kv_opt<DATA>(
    map: &std::collections::BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Option<DATA>>
where
    for<'de> DATA: serde::Deserialize<'de>,
{
    match map.get(key) {
        None => Ok(None),
        Some(value) => {
            let r = serde_json::value::from_value(value.clone())?;
            Ok(r)
        }
    }
}
