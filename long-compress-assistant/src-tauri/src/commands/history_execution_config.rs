use serde_json::{Map, Value};

// Persist only the supported snapshot schema. A bad/unknown snapshot must not
// prevent saving the task's final error and logs.
pub(super) fn sanitize(value: Value, task_type: &str, workload: &str) -> Option<Value> {
    if value.get("schemaVersion")?.as_u64()? != 1 || value.to_string().len() > 16_384 { return None; }
    let kind = value.get("kind")?.as_str()?;
    let expected = match kind {
        "archive-extraction" => ("decompression", "archive"),
        "archive-compression" => ("compression", "archive"),
        "archive-conversion" => ("compression", "archive"),
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
        "archive-conversion" => &[("outputPath", 's'), ("format", 's'), ("level", 'n'), ("splitSize", 'n'), ("encryptFilenames", 'b'), ("passwordRequired", 'b')],
        "video-compression" => &[("outputDirectory", 's')],
        "pdf-optimization" => &[("outputDirectory", 's'), ("mode", 's'), ("allowLargerOutput", 'b')],
        _ => &[],
    };
    output.extend(project(&value, fields)?);
    if kind == "archive-conversion" {
        if let Some(count) = value.get("rarRecoveryVolumes") {
            if !count.as_u64().is_some_and(|n| n <= 10) { return None; }
            output.insert("rarRecoveryVolumes".into(), count.clone());
        }
        if let Some(percent) = value.get("rarRecoveryPercent") {
            if !matches!(percent.as_u64(), Some(0 | 3 | 5 | 10)) { return None; }
            output.insert("rarRecoveryPercent".into(), percent.clone());
        }
        for (key, default, allowed) in [("zipEncryption", "aes256", ["aes256", "zipcrypto"]),
            ("zipPasswordEncoding", "utf8", ["utf8", "windows_local"]),
            ("zipSplitMode", "numeric", ["numeric", "standard"])] {
            let selected = value.get(key).and_then(Value::as_str).unwrap_or(default);
            if !allowed.contains(&selected) { return None; }
            output.insert(key.into(), selected.into());
        }
    }
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
            if let Some(count) = value.get("settings")?.get("rarRecoveryVolumes") {
                if !count.as_u64().is_some_and(|n| n <= 10) { return None; }
                settings.insert("rarRecoveryVolumes".into(), count.clone());
            }
            if let Some(percent) = value.get("settings")?.get("rarRecoveryPercent") {
                if !matches!(percent.as_u64(), Some(0 | 3 | 5 | 10)) { return None; }
                settings.insert("rarRecoveryPercent".into(), percent.clone());
            }
            if let Some(mode) = value.get("settings")?.get("zipSplitMode") {
                if !matches!(mode.as_str(), Some("numeric" | "standard")) { return None; }
                settings.insert("zipSplitMode".into(), mode.clone());
            }
            if let Some(encryption) = value.get("settings")?.get("zipEncryption") {
                if !matches!(encryption.as_str(), Some("aes256" | "zipcrypto")) { return None; }
                settings.insert("zipEncryption".into(), encryption.clone());
            }
            settings.insert("fastPack".into(), Value::Bool(false));
            if let Some(encrypted_names) = value.get("settings")?.get("encryptFilenames") {
                if !encrypted_names.is_boolean() { return None; }
                if encrypted_names.as_bool()? && (!matches!(settings.get("format")?.as_str()?, "7z" | "rar")
                    || !value.get("passwordRequired")?.as_bool()?) { return None; }
                settings.insert("encryptFilenames".into(), encrypted_names.clone());
            }
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
    fn conversion_snapshot_keeps_its_identity_without_passwords() {
        let input = serde_json::json!({"schemaVersion": 1, "kind": "archive-conversion",
            "outputPath": "C:/new.7z", "format": "7z", "level": 6, "splitSize": 65536,
            "encryptFilenames": true, "passwordRequired": true,
            "password": "target-secret", "sourcePassword": "input-secret"});
        let saved = super::sanitize(input.clone(), "compression", "archive").unwrap();
        assert_eq!(saved["kind"], "archive-conversion");
        assert_eq!(saved["splitSize"], 65536);
        assert!(!saved.to_string().contains("secret"));
        assert_eq!(saved["zipSplitMode"], "numeric");
        let mut standard = input.clone();
        standard["zipSplitMode"] = "standard".into();
        assert_eq!(super::sanitize(standard.clone(), "compression", "archive").unwrap()["zipSplitMode"], "standard");
        standard["zipSplitMode"] = "invalid".into();
        assert!(super::sanitize(standard, "compression", "archive").is_none());
        assert!(super::sanitize(input, "decompression", "archive").is_none());
    }
    #[test]
    fn zip_encoding_snapshot_is_optional_validated_and_does_not_store_passwords() {
        let original = serde_json::json!({"schemaVersion":1,"kind":"archive-compression",
            "outputDirectory":"D:/out","passwordRequired":true,"settings":{
            "format":"zip","level":6,"filename":"archive","splitArchive":false,
            "splitSize":"100","keepStructure":true,"verifyAfter":true,"createSolidArchive":false,
            "password":"never persist"}});
        let old = super::sanitize(original.clone(), "compression", "archive").unwrap();
        for mode in ["aes256", "zipcrypto"] {
            let mut current = original.clone();
            current["settings"]["zipEncryption"] = mode.into();
            let saved = super::sanitize(current, "compression", "archive").unwrap();
            assert_eq!(saved["settings"]["zipEncryption"], mode);
        }
        assert!(old["settings"].get("zipPasswordEncoding").is_none());
        assert!(old["settings"].get("password").is_none());
        assert!(old["settings"].get("encryptFilenames").is_none());
        assert!(old["settings"].get("zipSplitMode").is_none());
        for mode in ["numeric", "standard"] {
            let mut current = original.clone();
            current["settings"]["zipSplitMode"] = mode.into();
            assert_eq!(super::sanitize(current, "compression", "archive").unwrap()["settings"]["zipSplitMode"], mode);
        }
        let mut invalid_mode = original.clone();
        invalid_mode["settings"]["zipSplitMode"] = "other".into();
        assert!(super::sanitize(invalid_mode, "compression", "archive").is_none());
        for enabled in [false, true] {
            let mut current = original.clone();
            current["settings"]["format"] = "7z".into();
            current["settings"]["encryptFilenames"] = enabled.into();
            let saved = super::sanitize(current.clone(), "compression", "archive").unwrap();
            assert_eq!(saved["settings"]["encryptFilenames"], enabled);
            assert!(saved["settings"].get("password").is_none());
            if enabled {
                current["passwordRequired"] = false.into();
                assert!(super::sanitize(current, "compression", "archive").is_none());
            }
        }
        for invalid in [serde_json::json!("true"), serde_json::json!(null), serde_json::json!(true)] {
            let mut current = original.clone();
            current["settings"]["encryptFilenames"] = invalid;
            assert!(super::sanitize(current, "compression", "archive").is_none());
        }
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
