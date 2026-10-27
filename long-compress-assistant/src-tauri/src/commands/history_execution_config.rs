use serde_json::{Map, Value};

#[cfg(test)]
#[test]
fn directory1712_history_retains_optional_directory_counts_without_inventing_old_time_results(){
    let old=serde_json::json!({"schemaVersion":1,"kind":"archive-recovery","outputDirectory":"D:/out","recoveredFiles":0,"recoveredDirectories":2,"stopReason":"tail","revision":"a".repeat(64),"timePreservation":{"preservedFiles":0,"unpreservedFiles":0}});
    let clean=sanitize(old.clone(),"decompression","archive").unwrap();assert!(clean["timePreservation"].get("preservedDirectories").is_none());
    let mut current=old.clone();current["timePreservation"]["preservedDirectories"]=1.into();current["timePreservation"]["unpreservedDirectories"]=1.into();let clean=sanitize(current.clone(),"decompression","archive").unwrap();assert_eq!(clean["timePreservation"],current["timePreservation"]);
    for(preserved,unpreserved)in[(3,0),(-1,3),(1,0)]{let mut bad=current.clone();bad["timePreservation"]["preservedDirectories"]=preserved.into();bad["timePreservation"]["unpreservedDirectories"]=unpreserved.into();assert!(sanitize(bad,"decompression","archive").is_none());}
    current["timePreservation"].as_object_mut().unwrap().remove("unpreservedDirectories");assert!(sanitize(current,"decompression","archive").is_none());
}

#[cfg(test)]
#[test]
fn time1711_history_keeps_optional_time_totals_and_rejects_impossible_counts() {
    let old=serde_json::json!({"schemaVersion":1,"kind":"archive-recovery","outputDirectory":"D:/out","recoveredFiles":2,"stopReason":"tail","revision":"a".repeat(64)});
    assert!(sanitize(old.clone(),"decompression","archive").unwrap().get("timePreservation").is_none());
    let mut current=old.clone();current["timePreservation"]=serde_json::json!({"preservedFiles":1,"unpreservedFiles":1,"unrelated":"discard"});
    let clean=sanitize(current,"decompression","archive").unwrap();assert_eq!(clean["timePreservation"],serde_json::json!({"preservedFiles":1,"unpreservedFiles":1}));
    for counts in [serde_json::json!({"preservedFiles":3,"unpreservedFiles":0}),serde_json::json!({"preservedFiles":1,"unpreservedFiles":0}),serde_json::json!({"preservedFiles":-1,"unpreservedFiles":3}),serde_json::json!({"preservedFiles":1.5,"unpreservedFiles":0.5})] {
        let mut invalid=old.clone();invalid["timePreservation"]=counts;assert!(sanitize(invalid,"decompression","archive").is_none());
    }
}

#[cfg(test)]
#[test]
fn zpaq1537_final_history_preserves_guarded_selection_and_rejects_missing_identity(){
    let config=serde_json::json!({"schemaVersion":1,"kind":"archive-extraction","outputDirectory":"D:/out","extractToSubfolder":false,"zpaqSnapshot":{"version":1,"date":20261020130000i64},"zpaqCatalogId":"a".repeat(64),"selectedEntries":["资料/后来删除.txt"],"password":"not-persisted"});
    let saved=sanitize(config.clone(),"decompression","archive").unwrap();assert_eq!(saved["selectedEntries"],config["selectedEntries"]);assert_eq!(saved["zpaqCatalogId"],config["zpaqCatalogId"]);assert!(saved.get("password").is_none());
    let mut bad=config.clone();bad.as_object_mut().unwrap().remove("zpaqSnapshot");assert!(sanitize(bad,"decompression","archive").is_none());let mut bad=config;bad["selectedEntries"]=serde_json::json!([]);assert!(sanitize(bad,"decompression","archive").is_none());
}

// Persist only the supported snapshot schema. A bad/unknown snapshot must not
// prevent saving the task's final error and logs.
pub(super) fn sanitize(value: Value, task_type: &str, workload: &str) -> Option<Value> {
    if value.get("schemaVersion")?.as_u64()? != 1 || value.to_string().len() > if value.get("operation").and_then(Value::as_str)==Some("zpaq-update") || value.get("zpaqCatalogId").is_some() || value.get("librarySourceId").is_some() || value.get("comparisonSelection").is_some() || value.get("readerSelection").is_some() || value.get("zpaqRetrieval").is_some() {262_144} else {16_384} { return None; }
    let kind = value.get("kind")?.as_str()?;
    let expected = match kind {
        "archive-extraction" | "archive-recovery" => ("decompression", "archive"),
        "archive-compression" => ("compression", "archive"),
        "archive-conversion" | "archive-edit" => ("compression", "archive"),
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
        "archive-edit" => &[("outputPath", 's')],
        "archive-recovery" => &[("outputDirectory",'s')],
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
    if kind=="archive-recovery" {
        let selection:crate::services::lzop::recovery::HistorySelection=serde_json::from_value(value.clone()).ok()?;selection.validate().ok()?;
        for(key,v)in serde_json::to_value(selection).ok()?.as_object()? {output.insert(key.clone(),v.clone());}
    }
    if kind=="archive-extraction" {if let Some(selection)=value.get("zpaqRetrieval") {
        let selection:crate::services::zpaq_engine::retrieval::Selection=serde_json::from_value(selection.clone()).ok()?;selection.validate().ok()?;
        if output.get("zpaqSnapshot")!=Some(&serde_json::to_value(selection.snapshot).ok()?){return None}
        output.insert("zpaqRetrieval".into(),serde_json::to_value(selection).ok()?);
    }}
    if kind=="archive-extraction" {if let Some(selection)=value.get("readerSelection") {
        let selection:crate::services::image_reader::Selection=serde_json::from_value(selection.clone()).ok()?;
        selection.validate().ok()?;
        output.insert("readerSelection".into(),serde_json::to_value(selection).ok()?);
    }}
    if kind=="archive-extraction" {if let Some(selection)=value.get("comparisonSelection") {
        let selection:crate::services::source_comparison::Selection=serde_json::from_value(selection.clone()).ok()?;
        selection.validate().ok()?;
        output.insert("comparisonSelection".into(),serde_json::to_value(selection).ok()?);
    }}
    if kind=="archive-extraction" {if let Some(id)=value.get("zpaqCatalogId") {
        let text=id.as_str()?;if text.len()!=64||!text.bytes().all(|b|b.is_ascii_hexdigit())||!output.contains_key("zpaqSnapshot"){return None}
        let selected=value.get("selectedEntries")?.as_array()?;if selected.is_empty()||selected.len()>2000||!selected.iter().all(|p|p.as_str().is_some_and(|s|!s.is_empty()&&s.len()<=32768)){return None}
        output.insert("zpaqCatalogId".into(),id.clone());output.insert("selectedEntries".into(),Value::Array(selected.clone()));
    }}
    if kind=="archive-extraction" {if let Some(id)=value.get("librarySourceId") {
        if !id.as_str().is_some_and(|s|!s.is_empty()&&s.len()<=100){return None}
        let revision=value.get("libraryRevision")?;if !revision.as_str().is_some_and(|s|!s.is_empty()&&s.len()<=60000){return None}
        let selected=value.get("selectedEntries")?.as_array()?;if selected.is_empty()||selected.len()>2000||serde_json::to_string(selected).ok()?.len()>60000||!selected.iter().all(|p|p.as_str().is_some_and(|s|!s.is_empty()&&s.len()<=32768)){return None}
        let keep=value.get("libraryKeepStructure")?;keep.as_bool()?;
        let conflict=value.get("libraryConflictPolicy")?;if !matches!(conflict.as_str(),Some("rename"|"skip")){return None}
        for(key,v)in [("librarySourceId",id),("libraryRevision",revision),("selectedEntries",value.get("selectedEntries")?),("libraryKeepStructure",keep),("libraryConflictPolicy",conflict)]{output.insert(key.into(),v.clone());}
    }}
    if kind == "archive-conversion" {
        if let Some(required) = value.get("sourcePasswordRequired") {
            required.as_bool()?;
            output.insert("sourcePasswordRequired".into(), required.clone());
        }
        if let Some(operation)=value.get("operation") {
            if operation.as_str()!=Some("zpaq-update") || value.get("format")?.as_str()? != "zpaq" || value.get("sourceZpaqSnapshot").is_some() { return None; }
            let sources=value.get("updateSources")?.as_array()?;
            if sources.is_empty() || sources.len()>1000 || !sources.iter().all(|p|p.as_str().is_some_and(|s|!s.is_empty()&&s.len()<=32768)) {return None}
            let prefix=value.get("archivePrefix").and_then(Value::as_str).unwrap_or("");if prefix.len()>32768{return None}
            output.insert("operation".into(),operation.clone());output.insert("updateSources".into(),Value::Array(sources.clone()));output.insert("archivePrefix".into(),prefix.into());
        }
        if let Some(filter)=value.get("sevenZipFilter") {
            crate::services::seven_zip_method::SevenZipFilter::from_setting(filter.as_str()?)?;
            output.insert("sevenZipFilter".into(),filter.clone());
        }
        if let Some(distance)=value.get("sevenZipDeltaDistance") {
            if !(1..=256).contains(&distance.as_u64()?){return None}
            output.insert("sevenZipDeltaDistance".into(),distance.clone());
        }
        if let Some(method)=value.get("sevenZipMethod") {
            crate::services::seven_zip_method::SevenZipMethod::from_setting(method.as_str()?)?;
            output.insert("sevenZipMethod".into(),method.clone());
        }
        if let Some(method) = value.get("wimMethod") {
            if !matches!(method.as_str(), Some("store" | "xpress" | "lzx" | "lzms" | "solid")) { return None; }
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
        for (key, default, allowed) in [("zipMethod", "deflate", &["deflate", "zstd", "bzip2", "ppmd", "lzma", "store"][..]), ("zipEncryption", "aes256", &["aes256", "zipcrypto"][..]),
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
            if let Some(filter)=value.get("settings")?.get("sevenZipFilter") {
                crate::services::seven_zip_method::SevenZipFilter::from_setting(filter.as_str()?)?;
                settings.insert("sevenZipFilter".into(),filter.clone());
            }
            if let Some(distance)=value.get("settings")?.get("sevenZipDeltaDistance") {
                if !(1..=256).contains(&distance.as_u64()?){return None}
                settings.insert("sevenZipDeltaDistance".into(),distance.clone());
            }
            if let Some(method)=value.get("settings")?.get("sevenZipMethod") {
                crate::services::seven_zip_method::SevenZipMethod::from_setting(method.as_str()?)?;
                settings.insert("sevenZipMethod".into(),method.clone());
            }
            if let Some(method) = value.get("settings")?.get("wimMethod") {
                if !matches!(method.as_str(), Some("store" | "xpress" | "lzx" | "lzms" | "solid")) { return None; }
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
                if !matches!(method.as_str(), Some("deflate" | "zstd" | "bzip2" | "ppmd" | "lzma" | "store")) { return None; }
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
    fn recovery1542_history_preserves_only_boolean_password_requirements() {
        for required in [true,false] {
            let value=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/out.zip","format":"zip","level":6,"splitSize":0,"encryptFilenames":false,"passwordRequired":true,"sourcePasswordRequired":required,"sourcePassword":"never-save-source","password":"never-save-target"});
            let saved=super::sanitize(value.clone(),"compression","archive").unwrap();
            assert_eq!(saved["sourcePasswordRequired"],required);assert!(saved.get("sourcePassword").is_none());assert!(saved.get("password").is_none());
            let mut invalid=value;invalid["sourcePasswordRequired"]=serde_json::json!("true");assert!(super::sanitize(invalid,"compression","archive").is_none());
        }
    }
    #[test]
    fn zpaq1536_history_preserves_update_inputs_and_never_the_password() {
        let value=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","operation":"zpaq-update","updateSources":["D:/资料"],"archivePrefix":"备份","outputPath":"D:/new.zpaq","format":"zpaq","level":6,"splitSize":65536,"encryptFilenames":false,"passwordRequired":true,"sourcePassword":"do not persist"});
        let saved=super::sanitize(value,"compression","archive").unwrap();assert_eq!(saved["operation"],"zpaq-update");assert_eq!(saved["updateSources"][0],"D:/资料");assert_eq!(saved["archivePrefix"],"备份");assert!(saved.get("sourcePassword").is_none());
    }
    #[test]
    fn zip1535_history_preserves_ppmd_in_creation_and_conversion() {
        let conversion=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","format":"zip","level":0,"outputPath":"D:/out.zip","splitSize":65536,"encryptFilenames":false,"passwordRequired":true,"zipMethod":"ppmd","password":"secret"});
        let saved=super::sanitize(conversion,"compression","archive").unwrap();assert_eq!(saved["zipMethod"],"ppmd");assert!(saved.get("password").is_none());
        let compression=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"zip","level":0,"filename":"new","splitArchive":true,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":false,"zipMethod":"ppmd"}});
        assert_eq!(super::sanitize(compression,"compression","archive").unwrap()["settings"]["zipMethod"],"ppmd");
    }
    #[test]
    fn zip1538_history_preserves_lzma_in_creation_and_conversion() {
        let conversion=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","format":"zip","level":0,"outputPath":"D:/out.zip","splitSize":65536,"encryptFilenames":false,"passwordRequired":true,"zipMethod":"lzma","password":"secret"});
        let saved=super::sanitize(conversion,"compression","archive").unwrap();assert_eq!(saved["zipMethod"],"lzma");assert!(saved.get("password").is_none());
        let compression=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"zip","level":0,"filename":"new","splitArchive":true,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":false,"zipMethod":"lzma"}});
        assert_eq!(super::sanitize(compression,"compression","archive").unwrap()["settings"]["zipMethod"],"lzma");
    }


    #[test]
    fn zip1534_history_preserves_bzip2_in_creation_and_conversion() {
        let conversion=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","format":"zip","level":0,"outputPath":"D:/out.zip","splitSize":65536,"encryptFilenames":false,"passwordRequired":true,"zipMethod":"bzip2","password":"secret"});
        let saved=super::sanitize(conversion,"compression","archive").unwrap();assert_eq!(saved["zipMethod"],"bzip2");assert!(saved.get("password").is_none());
        let compression=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"zip","level":0,"filename":"new","splitArchive":true,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":false,"zipMethod":"bzip2"}});
        assert_eq!(super::sanitize(compression,"compression","archive").unwrap()["settings"]["zipMethod"],"bzip2");
    }

    #[test]
    fn wim1539_history_retains_method_without_secrets() {
        let config=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","format":"esd","level":6,"outputPath":"D:/new.esd","splitSize":65536,"encryptFilenames":false,"passwordRequired":false,"wimMethod":"solid","password":"do not save"});
        let saved=super::sanitize(config.clone(),"compression","archive").unwrap();assert_eq!(saved["wimMethod"],"solid");assert!(saved.get("password").is_none());
        let mut bad=config;bad["wimMethod"]="unknown".into();assert!(super::sanitize(bad,"compression","archive").is_none());
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

#[cfg(test)]
#[test]
fn zip_edit1544_history_identifies_results_without_replaying_edits(){
    let value=serde_json::json!({"schemaVersion":1,"kind":"archive-edit","outputPath":"D:/new.zip","changes":["private/source"],"password":"secret"});
    let result=sanitize(value,"compression","archive").unwrap();assert_eq!(result,serde_json::json!({"schemaVersion":1,"kind":"archive-edit","outputPath":"D:/new.zip"}));
}

#[cfg(test)]
#[test]
fn library1549_history_keeps_exact_scope_and_output_policy_without_credentials(){
 let config=serde_json::json!({"schemaVersion":1,"kind":"archive-extraction","outputDirectory":"D:/out","extractToSubfolder":false,"librarySourceId":"a","libraryRevision":"rev","selectedEntries":["资料/合同.txt"],"libraryKeepStructure":false,"libraryConflictPolicy":"skip","password":"do-not-store"});
 let saved=sanitize(config.clone(),"decompression","archive").unwrap();assert_eq!(saved["selectedEntries"],config["selectedEntries"]);assert_eq!(saved["libraryKeepStructure"],false);assert!(saved.get("password").is_none());
 for key in ["libraryRevision","selectedEntries","libraryConflictPolicy"]{let mut bad=config.clone();bad.as_object_mut().unwrap().remove(key);assert!(sanitize(bad,"decompression","archive").is_none());}
}

#[cfg(test)]
#[test]
fn comparison1554_history_keeps_exact_scope_and_drops_extra_fields(){
 let value=serde_json::json!({"schemaVersion":1,"kind":"archive-extraction","outputDirectory":"D:/out","extractToSubfolder":false,"comparisonSelection":{"source":{"path":"C:/source.zip","kind":"zip","revision":"a".repeat(64)},"paths":["nested/合同.txt"],"keepStructure":false,"conflictPolicy":"skip","password":"must-not-persist"}});
 let clean=sanitize(value.clone(),"decompression","archive").unwrap();assert_eq!(clean["comparisonSelection"]["paths"],value["comparisonSelection"]["paths"]);assert!(!clean.to_string().contains("must-not-persist"));let mut invalid=value;invalid["comparisonSelection"]["paths"]=serde_json::json!([]);assert!(sanitize(invalid,"decompression","archive").is_none());
}

#[cfg(test)]
#[test]
fn comparison1555_history_keeps_encoding_and_encryption_marker_without_passwords(){
 let value=serde_json::json!({"schemaVersion":1,"kind":"archive-extraction","outputDirectory":"D:/out","extractToSubfolder":false,"comparisonSelection":{"source":{"path":"C:/group.zip","kind":"zip","revision":"a".repeat(64),"encrypted":true,"password":"source-secret"},"paths":["one.txt"],"keepStructure":true,"conflictPolicy":"rename","zipPasswordEncoding":"cp936","password":"selection-secret"}});
 let result=sanitize(value.clone(),"decompression","archive").unwrap();assert_eq!(result["comparisonSelection"]["zipPasswordEncoding"],"cp936");assert_eq!(result["comparisonSelection"]["source"]["encrypted"],true);assert!(!result.to_string().contains("secret"));let mut bad=value;bad["comparisonSelection"]["zipPasswordEncoding"]="invalid".into();assert!(sanitize(bad,"decompression","archive").is_none());
}

#[cfg(test)]
#[test]
fn reader1558_history_preserves_large_exact_scope_and_excludes_credentials(){
 let paths:Vec<_>=(0..200).map(|n|format!("pages/{n:04}-{}.png","x".repeat(110))).collect();
 let value=serde_json::json!({"schemaVersion":1,"kind":"archive-extraction","outputDirectory":"D:/out","extractToSubfolder":false,"readerSelection":{"revision":"a".repeat(64),"paths":paths,"keepStructure":false,"conflictPolicy":"skip","password":"selection-secret"},"password":"source-secret","zipPasswordEncoding":"cp936"});
 assert!(value.to_string().len()>16384);let result=sanitize(value.clone(),"decompression","archive").unwrap();assert_eq!(result["readerSelection"]["paths"],value["readerSelection"]["paths"]);assert_eq!(result["readerSelection"]["keepStructure"],false);assert_eq!(result["zipPasswordEncoding"],"cp936");assert!(!result.to_string().contains("secret"));
}
#[cfg(test)]
#[test]
fn reader1558_history_rejects_empty_duplicate_and_unbound_ranges(){
 let value=serde_json::json!({"schemaVersion":1,"kind":"archive-extraction","outputDirectory":"D:/out","extractToSubfolder":false,"readerSelection":{"revision":"a".repeat(64),"paths":["pages/1.png"],"keepStructure":true,"conflictPolicy":"rename"}});
 for paths in [serde_json::json!([]),serde_json::json!(["a.png","a.png"])]{let mut invalid=value.clone();invalid["readerSelection"]["paths"]=paths;assert!(sanitize(invalid,"decompression","archive").is_none());}
 let mut invalid=value;invalid["readerSelection"]["revision"]="".into();assert!(sanitize(invalid,"decompression","archive").is_none());
}

#[test]
fn zpaq164_history_retains_subset_intent_without_credentials(){
 let selection=serde_json::json!({"snapshot":{"version":3,"date":20261020150000i64},"revision":"a".repeat(64),"paths":["docs/kept.txt"],"password":"not-saved"});
 let value=serde_json::json!({"schemaVersion":1,"kind":"archive-extraction","outputDirectory":"D:/out","extractToSubfolder":false,"zpaqSnapshot":selection["snapshot"],"zpaqRetrieval":selection,"password":"not-saved"});
 let saved=sanitize(value.clone(),"decompression","archive").unwrap();assert_eq!(saved["zpaqRetrieval"]["paths"],value["zpaqRetrieval"]["paths"]);assert!(!saved.to_string().contains("not-saved"));let mut bad=value;bad["zpaqRetrieval"]["paths"]=serde_json::json!([]);assert!(sanitize(bad,"decompression","archive").is_none());
}


#[cfg(test)]#[test]
fn sevenz1646_history_creation_and_conversion_retain_method_reject_unknown(){
 for method in ["lzma2","lzma","bzip2"]{
  let creation=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"7z","level":1,"filename":"","splitArchive":false,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":true,"sevenZipMethod":method}});
  let converted=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/out.7z","format":"7z","level":1,"splitSize":0,"encryptFilenames":false,"passwordRequired":false,"sevenZipMethod":method});
  for (value,kind) in [(creation,"compression"),(converted,"conversion")]{let saved=sanitize(value.clone(),"compression","archive").unwrap();assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipMethod"]}else{&saved["sevenZipMethod"]},method);let mut bad=value;if kind=="compression"{bad["settings"]["sevenZipMethod"]="bad".into()}else{bad["sevenZipMethod"]="bad".into()}assert!(sanitize(bad,"compression","archive").is_none());}
 }
}

#[cfg(test)]#[test]
fn sevenz1647_history_creation_and_conversion_retain_method_reject_unknown(){
 for method in ["deflate"]{
  let creation=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"7z","level":1,"filename":"","splitArchive":false,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":true,"sevenZipMethod":method}});
  let converted=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/out.7z","format":"7z","level":1,"splitSize":0,"encryptFilenames":false,"passwordRequired":false,"sevenZipMethod":method});
  for (value,kind) in [(creation,"compression"),(converted,"conversion")]{let saved=sanitize(value.clone(),"compression","archive").unwrap();assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipMethod"]}else{&saved["sevenZipMethod"]},method);let mut bad=value;if kind=="compression"{bad["settings"]["sevenZipMethod"]="bad".into()}else{bad["sevenZipMethod"]="bad".into()}assert!(sanitize(bad,"compression","archive").is_none());}
 }
}

#[cfg(test)]#[test]
fn sevenz1648_history_creation_and_conversion_retain_method_reject_unknown(){
 for method in ["ppmd"]{
  let creation=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"7z","level":1,"filename":"","splitArchive":false,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":true,"sevenZipMethod":method}});
  let converted=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/out.7z","format":"7z","level":1,"splitSize":0,"encryptFilenames":false,"passwordRequired":false,"sevenZipMethod":method});
  for (value,kind) in [(creation,"compression"),(converted,"conversion")]{let saved=sanitize(value.clone(),"compression","archive").unwrap();assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipMethod"]}else{&saved["sevenZipMethod"]},method);let mut bad=value;if kind=="compression"{bad["settings"]["sevenZipMethod"]="bad".into()}else{bad["sevenZipMethod"]="bad".into()}assert!(sanitize(bad,"compression","archive").is_none());}
 }
}

#[cfg(test)]#[test]
fn sevenz1649_history_creation_and_conversion_retain_method_reject_unknown(){
 for method in ["copy"]{
  let creation=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"7z","level":1,"filename":"","splitArchive":false,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":true,"sevenZipMethod":method}});
  let converted=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/out.7z","format":"7z","level":1,"splitSize":0,"encryptFilenames":false,"passwordRequired":false,"sevenZipMethod":method});
  for (value,kind) in [(creation,"compression"),(converted,"conversion")]{let saved=sanitize(value.clone(),"compression","archive").unwrap();assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipMethod"]}else{&saved["sevenZipMethod"]},method);let mut bad=value;if kind=="compression"{bad["settings"]["sevenZipMethod"]="bad".into()}else{bad["sevenZipMethod"]="bad".into()}assert!(sanitize(bad,"compression","archive").is_none());}
 }
}

#[cfg(test)]#[test]
fn zip1650_history_creation_and_conversion_retain_method_reject_unknown(){
 for method in ["store"]{
  let creation=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"zip","level":1,"filename":"","splitArchive":false,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":true,"zipMethod":method}});
  let converted=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/out.zip","format":"zip","level":1,"splitSize":0,"encryptFilenames":false,"passwordRequired":false,"zipMethod":method});
  for (value,kind) in [(creation,"compression"),(converted,"conversion")]{let saved=sanitize(value.clone(),"compression","archive").unwrap();assert_eq!(if kind=="compression"{&saved["settings"]["zipMethod"]}else{&saved["zipMethod"]},method);let mut bad=value;if kind=="compression"{bad["settings"]["zipMethod"]="bad".into()}else{bad["zipMethod"]="bad".into()}assert!(sanitize(bad,"compression","archive").is_none());}
 }
}

#[cfg(test)]#[test]
fn sevenz1651_history_creation_and_conversion_retain_method_reject_unknown(){
 for method in ["lzma2"]{
  let creation=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"7z","level":1,"filename":"","splitArchive":false,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":true,"sevenZipMethod":method,"sevenZipFilter":"delta","sevenZipDeltaDistance":256}});
  let converted=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/out.7z","format":"7z","level":1,"splitSize":0,"encryptFilenames":false,"passwordRequired":false,"sevenZipMethod":method,"sevenZipFilter":"delta","sevenZipDeltaDistance":256});
  for (value,kind) in [(creation,"compression"),(converted,"conversion")]{let saved=sanitize(value.clone(),"compression","archive").unwrap();assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipMethod"]}else{&saved["sevenZipMethod"]},method);assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipFilter"]}else{&saved["sevenZipFilter"]},"delta");let mut bad=value;if kind=="compression"{bad["settings"]["sevenZipMethod"]="bad".into()}else{bad["sevenZipMethod"]="bad".into()}assert!(sanitize(bad,"compression","archive").is_none());}
 }
}

#[cfg(test)]#[test]
fn sevenz1652_history_creation_and_conversion_retain_method_reject_unknown(){
 for method in ["lzma2"]{
  let creation=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"7z","level":1,"filename":"","splitArchive":false,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":true,"sevenZipMethod":method,"sevenZipFilter":"bcj_x86","sevenZipDeltaDistance":256}});
  let converted=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/out.7z","format":"7z","level":1,"splitSize":0,"encryptFilenames":false,"passwordRequired":false,"sevenZipMethod":method,"sevenZipFilter":"bcj_x86","sevenZipDeltaDistance":256});
  for (value,kind) in [(creation,"compression"),(converted,"conversion")]{let saved=sanitize(value.clone(),"compression","archive").unwrap();assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipMethod"]}else{&saved["sevenZipMethod"]},method);assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipFilter"]}else{&saved["sevenZipFilter"]},"bcj_x86");let mut bad=value;if kind=="compression"{bad["settings"]["sevenZipFilter"]="bad".into()}else{bad["sevenZipFilter"]="bad".into()}assert!(sanitize(bad,"compression","archive").is_none());}
 }
}

#[cfg(test)]#[test]
fn sevenz1653_history_creation_and_conversion_retain_method_reject_unknown(){
 for method in ["lzma2"]{
  let creation=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"7z","level":1,"filename":"","splitArchive":false,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":true,"sevenZipMethod":method,"sevenZipFilter":"bcj_arm32","sevenZipDeltaDistance":256}});
  let converted=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/out.7z","format":"7z","level":1,"splitSize":0,"encryptFilenames":false,"passwordRequired":false,"sevenZipMethod":method,"sevenZipFilter":"bcj_arm32","sevenZipDeltaDistance":256});
  for (value,kind) in [(creation,"compression"),(converted,"conversion")]{let saved=sanitize(value.clone(),"compression","archive").unwrap();assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipMethod"]}else{&saved["sevenZipMethod"]},method);assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipFilter"]}else{&saved["sevenZipFilter"]},"bcj_arm32");let mut bad=value;if kind=="compression"{bad["settings"]["sevenZipFilter"]="bad".into()}else{bad["sevenZipFilter"]="bad".into()}assert!(sanitize(bad,"compression","archive").is_none());}
 }
}

#[cfg(test)]#[test]
fn sevenz1654_history_creation_and_conversion_retain_method_reject_unknown(){
 for method in ["lzma2"]{
  let creation=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"7z","level":1,"filename":"","splitArchive":false,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":true,"sevenZipMethod":method,"sevenZipFilter":"bcj_arm_thumb","sevenZipDeltaDistance":256}});
  let converted=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/out.7z","format":"7z","level":1,"splitSize":0,"encryptFilenames":false,"passwordRequired":false,"sevenZipMethod":method,"sevenZipFilter":"bcj_arm_thumb","sevenZipDeltaDistance":256});
  for (value,kind) in [(creation,"compression"),(converted,"conversion")]{let saved=sanitize(value.clone(),"compression","archive").unwrap();assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipMethod"]}else{&saved["sevenZipMethod"]},method);assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipFilter"]}else{&saved["sevenZipFilter"]},"bcj_arm_thumb");let mut bad=value;if kind=="compression"{bad["settings"]["sevenZipFilter"]="bad".into()}else{bad["sevenZipFilter"]="bad".into()}assert!(sanitize(bad,"compression","archive").is_none());}
 }
}

#[cfg(test)]#[test]
fn sevenz1655_history_creation_and_conversion_retain_method_reject_unknown(){
 for method in ["lzma2"]{
  let creation=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"7z","level":1,"filename":"","splitArchive":false,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":true,"sevenZipMethod":method,"sevenZipFilter":"bcj_arm64","sevenZipDeltaDistance":256}});
  let converted=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/out.7z","format":"7z","level":1,"splitSize":0,"encryptFilenames":false,"passwordRequired":false,"sevenZipMethod":method,"sevenZipFilter":"bcj_arm64","sevenZipDeltaDistance":256});
  for (value,kind) in [(creation,"compression"),(converted,"conversion")]{let saved=sanitize(value.clone(),"compression","archive").unwrap();assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipMethod"]}else{&saved["sevenZipMethod"]},method);assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipFilter"]}else{&saved["sevenZipFilter"]},"bcj_arm64");let mut bad=value;if kind=="compression"{bad["settings"]["sevenZipFilter"]="bad".into()}else{bad["sevenZipFilter"]="bad".into()}assert!(sanitize(bad,"compression","archive").is_none());}
 }
}

#[cfg(test)]#[test]
fn sevenz1656_history_creation_and_conversion_retain_method_reject_unknown(){
 for method in ["lzma2"]{
  let creation=serde_json::json!({"schemaVersion":1,"kind":"archive-compression","outputDirectory":"D:/out","passwordRequired":true,"settings":{"format":"7z","level":1,"filename":"","splitArchive":false,"splitSize":"1","keepStructure":true,"verifyAfter":true,"createSolidArchive":true,"sevenZipMethod":method,"sevenZipFilter":"bcj_riscv","sevenZipDeltaDistance":256}});
  let converted=serde_json::json!({"schemaVersion":1,"kind":"archive-conversion","outputPath":"D:/out.7z","format":"7z","level":1,"splitSize":0,"encryptFilenames":false,"passwordRequired":false,"sevenZipMethod":method,"sevenZipFilter":"bcj_riscv","sevenZipDeltaDistance":256});
  for (value,kind) in [(creation,"compression"),(converted,"conversion")]{let saved=sanitize(value.clone(),"compression","archive").unwrap();assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipMethod"]}else{&saved["sevenZipMethod"]},method);assert_eq!(if kind=="compression"{&saved["settings"]["sevenZipFilter"]}else{&saved["sevenZipFilter"]},"bcj_riscv");let mut bad=value;if kind=="compression"{bad["settings"]["sevenZipFilter"]="bad".into()}else{bad["sevenZipFilter"]="bad".into()}assert!(sanitize(bad,"compression","archive").is_none());}
 }
}

#[cfg(test)]
mod lzop_recovery_history_174 {
 #[test] fn lzop174_history_retains_partial_scope_without_normal_extraction_semantics(){
  let value=serde_json::json!({"schemaVersion":1,"kind":"archive-recovery","outputDirectory":"D:/result","recoveredFiles":2,"stopReason":"后续截断","revision":"a".repeat(64),"password":"discard"});
  let clean=super::sanitize(value.clone(),"decompression","archive").unwrap();assert_eq!(clean["kind"],"archive-recovery");assert_eq!(clean["recoveredFiles"],2);assert!(clean.get("password").is_none());let mut invalid=value;invalid["recoveredFiles"]=(-1).into();assert!(super::sanitize(invalid,"decompression","archive").is_none());
 }
}
