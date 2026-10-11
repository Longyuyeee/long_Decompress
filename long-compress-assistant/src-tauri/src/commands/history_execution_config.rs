use serde_json::{Map, Value};

// Persist only the supported snapshot schema. A bad/unknown snapshot must not
// prevent saving the task's final error and logs.
pub(super) fn sanitize(value: Value, task_type: &str, workload: &str) -> Option<Value> {
    if value.get("schemaVersion")?.as_u64()? != 1 || value.to_string().len() > 16_384 { return None; }
    let kind = value.get("kind")?.as_str()?;
    let expected = match kind {
        "archive-extraction" => ("decompression", "archive"),
        "archive-compression" => ("compression", "archive"),
        "image-compression" => ("compression", "image"),
        "video-compression" => ("compression", "video"),
        "pdf-optimization" => ("compression", "pdf"),
        _ => return None,
    };
    if (task_type, workload) != expected { return None; }
    let mut output = Map::from_iter([
        ("schemaVersion".into(), Value::from(1)), ("kind".into(), Value::from(kind)),
    ]);
    let fields: &[(&str, char)] = match kind {
        "archive-extraction" => &[("outputDirectory", 's'), ("extractToSubfolder", 'b')],
        "archive-compression" => &[("outputDirectory", 's'), ("passwordRequired", 'b')],
        "video-compression" => &[("outputDirectory", 's')],
        "pdf-optimization" => &[("outputDirectory", 's'), ("mode", 's'), ("allowLargerOutput", 'b')],
        _ => &[],
    };
    output.extend(project(&value, fields)?);
    if kind == "archive-extraction" && output["outputDirectory"].as_str()?.trim().is_empty() { return None; }
    let settings_fields: &[(&str, char)] = match kind {
        "archive-compression" => &[("format", 's'), ("level", 'n'), ("filename", 's'), ("splitArchive", 'b'), ("splitSize", 's'), ("keepStructure", 'b'), ("verifyAfter", 'b'), ("createSolidArchive", 'b')],
        "image-compression" => &[("mode", 's'), ("quality", 'n'), ("resizeMode", 's'), ("maxWidth", 'n'), ("maxHeight", 'n'), ("outputFormat", 's'), ("preserveMetadata", 'b'), ("outputDirectory", 's'), ("conflictPolicy", 's')],
        "video-compression" => &[("preset", 's'), ("quality", 'n'), ("maxWidth", '?'), ("maxHeight", '?')],
        _ => &[],
    };
    if !settings_fields.is_empty() {
        let mut settings = project(value.get("settings")?, settings_fields)?;
        if kind == "archive-compression" {
            settings.insert("fastPack".into(), Value::Bool(false));
            if let Some(encoding) = value.get("settings")?.get("zipPasswordEncoding") {
                if !matches!(encoding.as_str(), Some("utf8" | "windows_local")) { return None; }
                settings.insert("zipPasswordEncoding".into(), encoding.clone());
            }
        }
        output.insert("settings".into(), Value::Object(settings));
    }
    Some(Value::Object(output))
}

fn project(value: &Value, fields: &[(&str, char)]) -> Option<Map<String, Value>> {
    fields.iter().map(|(key, ty)| {
        let field = value.get(key)?;
        let valid = match ty {
            's' => field.is_string(), 'b' => field.is_boolean(), 'n' => field.as_u64().is_some(),
            '?' => field.is_null() || field.as_u64().is_some(), _ => false,
        };
        valid.then(|| (key.to_string(), field.clone()))
    }).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn zip_encoding_snapshot_is_optional_validated_and_does_not_store_passwords() {
        let original = serde_json::json!({"schemaVersion":1,"kind":"archive-compression",
            "outputDirectory":"D:/out","passwordRequired":true,"settings":{
            "format":"zip","level":6,"filename":"archive","splitArchive":false,
            "splitSize":"100","keepStructure":true,"verifyAfter":true,"createSolidArchive":false,
            "password":"never persist"}});
        let old = super::sanitize(original.clone(), "compression", "archive").unwrap();
        assert!(old["settings"].get("zipPasswordEncoding").is_none());
        assert!(old["settings"].get("password").is_none());
        for mode in ["utf8", "windows_local"] {
            let mut current = original.clone();
            current["settings"]["zipPasswordEncoding"] = mode.into();
            let saved = super::sanitize(current, "compression", "archive").unwrap();
            assert_eq!(saved["settings"]["zipPasswordEncoding"], mode);
            assert!(saved["settings"].get("password").is_none());
        }
        let mut invalid = original;
        invalid["settings"]["zipPasswordEncoding"] = "unknown".into();
        assert!(super::sanitize(invalid, "compression", "archive").is_none());
    }
}
