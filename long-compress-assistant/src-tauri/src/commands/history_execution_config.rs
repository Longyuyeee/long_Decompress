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
    let input_encoding_key = match kind {
        "archive-extraction" => Some("zipPasswordEncoding"),
        "archive-conversion" => Some("sourceZipPasswordEncoding"),
        _ => None,
    };
    if let Some(key) = input_encoding_key {
        if let Some(selected) = value.get(key).filter(|selected| !selected.is_null()) {
            let encoding = crate::services::zip_password::ZipPasswordEncoding::from_setting(selected.as_str()?)?;
            output.insert(key.into(), encoding.as_str().into());
        }
    }
    let snapshot_key = match kind { "archive-extraction" => Some("zpaqSnapshot"), "archive-conversion" => Some("sourceZpaqSnapshot"), _ => None };
    if let Some(key) = snapshot_key {
        if let Some(value) = value.get(key).filter(|v| !v.is_null()) {
            let snapshot: crate::models::compression::ZpaqSnapshot = serde_json::from_value(value.clone()).ok()?;
            if snapshot.version == 0 || snapshot.version > 9_999_999 || !(19000101000000..=29991231235959).contains(&snapshot.date) { return None; }
            output.insert(key.into(), serde_json::to_value(snapshot).ok()?);
        }
    }
    if kind == "archive-conversion" {
        if let Some(method) = value.get("wimMethod") {
            if !matches!(method.as_str(), Some("store" | "xpress" | "lzx" | "lzms")) { return None; }
            output.insert("wimMethod".into(), method.clone());
        }
        if let Some(count) = value.get("rarRecoveryVolumes") {
            if !count.as_u64().is_some_and(|n| n <= 10) { return None; }
            output.insert("rarRecoveryVolumes".into(), count.clone());
        }
        if let Some(percent) = value.get("rarRecoveryPercent") {
            if !matches!(percent.as_u64(), Some(0 | 3 | 5 | 10)) { return None; }
            output.insert("rarRecoveryPercent".into(), percent.clone());
        }
        for (key, default, allowed) in [("zipMethod", "deflate", &["deflate", "zstd", "bzip2", "ppmd"][..]), ("zipEncryption", "aes256", &["aes256", "zipcrypto"][..]),
            ("zipSplitMode", "numeric", &["numeric", "standard"][..])] {
            let selected = value.get(key).and_then(Value::as_str).unwrap_or(default);
            if !allowed.contains(&selected) { return None; }
            output.insert(key.into(), selected.into());
        }
        let encoding = match value.get("zipPasswordEncoding") {
            Some(selected) => crate::services::zip_password::ZipPasswordEncoding::from_setting(selected.as_str()?)?,
            None => Default::default(),
        };
        output.insert("zipPasswordEncoding".into(), encoding.as_str().into());
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
            if let Some(method) = value.get("settings")?.get("wimMethod") {
                if !matches!(method.as_str(), Some("store" | "xpress" | "lzx" | "lzms")) { return None; }
                settings.insert("wimMethod".into(), method.clone());
            }
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
            if let Some(method) = value.get("settings")?.get("zipMethod") {
                if !matches!(method.as_str(), Some("deflate" | "zstd" | "bzip2" | "ppmd")) { return None; }
                settings.insert("zipMethod".into(), method.clone());
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
                crate::services::zip_password::ZipPasswordEncoding::from_setting(encoding.as_str()?)?;
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
    fn zip1535_history_preserves_ppmd_in_creation_and_conversion() {
        let conversion=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","format":"zip","level":0,"outputPath":"D:/out.zip","splitSize":65536,"encryptFilenames":false,"passwordRequired":true,"zipMethod":"ppmd","password":"secret"});
        let saved=super::sanitize(conversion,"compression","archive").unwrap();assert_eq!(saved["zipMethod"],"ppmd");assert!(saved.get("password").is_none());
        let compression=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"zip","level":0,"filename":"new","splitArchive":true,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":false,"zipMethod":"ppmd"}});
        assert_eq!(super::sanitize(compression,"compression","archive").unwrap()["settings"]["zipMethod"],"ppmd");
    }


    #[test]
    fn zip1534_history_preserves_bzip2_in_creation_and_conversion() {
        let conversion=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","format":"zip","level":0,"outputPath":"D:/out.zip","splitSize":65536,"encryptFilenames":false,"passwordRequired":true,"zipMethod":"bzip2","password":"secret"});
        let saved=super::sanitize(conversion,"compression","archive").unwrap();assert_eq!(saved["zipMethod"],"bzip2");assert!(saved.get("password").is_none());
        let compression=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"zip","level":0,"filename":"new","splitArchive":true,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":false,"zipMethod":"bzip2"}});
        assert_eq!(super::sanitize(compression,"compression","archive").unwrap()["settings"]["zipMethod"],"bzip2");
    }

    #[test]
    fn wim1533_history_retains_method_without_secrets() {
        let config=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","format":"wim","level":6,"outputPath":"D:/new.wim","splitSize":65536,"encryptFilenames":false,"passwordRequired":false,"wimMethod":"lzms","password":"do not save"});
        let saved=super::sanitize(config.clone(),"compression","archive").unwrap();assert_eq!(saved["wimMethod"],"lzms");assert!(saved.get("password").is_none());
        let mut bad=config;bad["wimMethod"]="unknown".into();assert!(super::sanitize(bad,"compression","archive").is_none());
    }

    #[test]
    fn wim1532_history_retains_method_without_secrets() {
        let config=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","format":"wim","level":6,"outputPath":"D:/new.wim","splitSize":65536,"encryptFilenames":false,"passwordRequired":false,"wimMethod":"lzx","password":"do not save"});
        let saved=super::sanitize(config.clone(),"compression","archive").unwrap();assert_eq!(saved["wimMethod"],"lzx");assert!(saved.get("password").is_none());
        let mut bad=config;bad["wimMethod"]="unknown".into();assert!(super::sanitize(bad,"compression","archive").is_none());
    }

    #[test]
    fn wim1531_history_retains_method_without_secrets() {
        let config=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","format":"wim","level":6,"outputPath":"D:/new.wim","splitSize":65536,"encryptFilenames":false,"passwordRequired":false,"wimMethod":"xpress","password":"do not save"});
        let saved=super::sanitize(config.clone(),"compression","archive").unwrap();assert_eq!(saved["wimMethod"],"xpress");assert!(saved.get("password").is_none());
        let mut bad=config;bad["wimMethod"]="unknown".into();assert!(super::sanitize(bad,"compression","archive").is_none());
    }

    #[test]
    fn zpaq_1530_contract_history_preserves_exact_snapshot_and_rejects_invalid_identity() {
        for (kind,task,key) in [("archive-extraction","decompression","zpaqSnapshot"),("archive-conversion","compression","sourceZpaqSnapshot")] {
            let mut input=if task=="decompression" {serde_json::json!({"schemaVersion":1,"kind":kind,"outputDirectory":"D:/out","extractToSubfolder":false})} else {serde_json::json!({"schemaVersion":1,"kind":kind,"outputPath":"D:/out.zip","format":"zip","level":6,"splitSize":0,"encryptFilenames":false,"passwordRequired":false})};
            input[key]=serde_json::json!({"version":1,"date":20261020130000i64});input["password"]=serde_json::json!("private");let saved=super::sanitize(input.clone(),task,"archive").unwrap();assert_eq!(saved[key],input[key]);assert!(!saved.to_string().contains("private"));
            input[key]["version"]=serde_json::json!(0);assert!(super::sanitize(input,task,"archive").is_none());
        }
    }
    #[test]
    fn input_zip_encoding_snapshots_keep_auto_and_explicit_modes_without_passwords() {
        for (input, task_type, key) in [
            (serde_json::json!({"schemaVersion":1,"kind":"archive-extraction","outputDirectory":"D:/out","extractToSubfolder":true,"password":"not-persisted"}), "decompression", "zipPasswordEncoding"),
            (serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/new.zip","format":"zip","level":6,"splitSize":0,"encryptFilenames":false,"passwordRequired":true,"sourcePassword":"not-persisted"}), "compression", "sourceZipPasswordEncoding"),
        ] {
            assert!(super::sanitize(input.clone(), task_type, "archive").unwrap().get(key).is_none());
            let mut automatic = input.clone(); automatic[key] = serde_json::Value::Null;
            assert!(super::sanitize(automatic, task_type, "archive").unwrap().get(key).is_none());
            for mode in ["utf8", "windows_local", "cp932", "cp936", "cp950"] {
                let mut configured = input.clone(); configured[key] = mode.into();
                let saved = super::sanitize(configured, task_type, "archive").unwrap();
                assert_eq!(saved[key], mode);
                assert!(!saved.to_string().contains("not-persisted"));
                if task_type == "compression" { assert_eq!(saved["zipPasswordEncoding"], "utf8"); }
            }
            for invalid in [serde_json::json!("unknown"), serde_json::json!(true), serde_json::json!(932)] {
                let mut configured = input.clone(); configured[key] = invalid;
                assert!(super::sanitize(configured, task_type, "archive").is_none());
            }
        }
    }

    #[test]
    fn conversion_password_encoding_keeps_fixed_pages_and_rejects_invalid_values() {
        let input = serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/new.zip",
            "format":"zip","level":6,"splitSize":0,"encryptFilenames":false,"passwordRequired":true,
            "password":"not-to-be-persisted","sourcePassword":"not-to-be-persisted"});
        assert_eq!(super::sanitize(input.clone(), "compression", "archive").unwrap()["zipPasswordEncoding"], "utf8");
        for mode in ["utf8", "windows_local", "cp932", "cp936", "cp950"] {
            let mut value = input.clone();
            value["zipPasswordEncoding"] = mode.into();
            let saved = super::sanitize(value, "compression", "archive").unwrap();
            assert_eq!(saved["zipPasswordEncoding"], mode);
            assert!(!saved.to_string().contains("not-to-be-persisted"));
        }
        for invalid in [serde_json::json!("unknown"), serde_json::json!(true), serde_json::json!(null), serde_json::json!(932)] {
            let mut value = input.clone();
            value["zipPasswordEncoding"] = invalid;
            assert!(super::sanitize(value, "compression", "archive").is_none());
        }
    }

    #[test]
    fn zip_output_1414_history_keeps_method_for_compression_and_conversion() {
        let mut conversion = serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/new.zip",
            "format":"zip","level":6,"splitSize":1048576,"encryptFilenames":false,"passwordRequired":true,"zipMethod":"zstd"});
        assert_eq!(super::sanitize(conversion.clone(), "compression", "archive").unwrap()["zipMethod"], "zstd");
        conversion["zipMethod"] = "unknown".into(); assert!(super::sanitize(conversion, "compression", "archive").is_none());
        let mut compression = serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,
            "settings":{"format":"zip","level":6,"filename":"new","splitArchive":true,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":false,"zipMethod":"zstd"}});
        assert_eq!(super::sanitize(compression.clone(), "compression", "archive").unwrap()["settings"]["zipMethod"], "zstd");
        compression["settings"]["zipMethod"] = "unknown".into(); assert!(super::sanitize(compression, "compression", "archive").is_none());
    }
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
        for mode in ["utf8", "windows_local", "cp932", "cp936", "cp950"] {
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
