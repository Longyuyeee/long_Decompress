use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Output;

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PreviewPayload {
    pub(super) bytes: Vec<u8>,
    pub(super) total_size: u64,
    pub(super) input_bytes: Option<u64>,
    #[serde(default)]
    pub(super) partial: bool,
}

impl PreviewPayload {
    // Decode the normal response directly. The legacy path handles errors and unusual responses.
    pub(super) fn decode(output: &Output) -> Option<Self> {
        super::archive_helper_output::decode_response(output)
    }

    pub(super) fn from_result(
        mut value: serde_json::Value,
        missing_size: &'static str,
    ) -> Result<Self> {
        let bytes = serde_json::from_value(
            value
                .get_mut("bytes")
                .map(serde_json::Value::take)
                .unwrap_or_default(),
        )?;
        let total_size = value["totalSize"].as_u64().context(missing_size)?;
        Ok(Self {
            bytes,
            total_size,
            input_bytes: value["inputBytes"].as_u64(),
            partial: value["partial"] == true,
        })
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::{os::windows::process::ExitStatusExt, process::ExitStatus};

    fn output(stdout: &str, exit_code: u32) -> Output {
        Output {
            status: ExitStatus::from_raw(exit_code),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    #[test]
    fn decodes_bounded_preview_protocol_without_a_json_tree() {
        for json in [
            r#"{"result":{"bytes":[0,127,255],"totalSize":3}}"#,
            r#"{"result":{"bytes":[0,127,255],"totalSize":8,"partial":true,"inputBytes":4}}"#,
        ] {
            let value: serde_json::Value = serde_json::from_str(json).unwrap();
            let expected =
                PreviewPayload::from_result(value["result"].clone(), "size missing").unwrap();
            assert_eq!(PreviewPayload::decode(&output(json, 0)), Some(expected));
            assert_eq!(PreviewPayload::decode(&output(json, 2)), None);
        }
    }

    #[test]
    fn defers_noncanonical_responses_to_existing_error_handling() {
        for json in [
            r#"[{"bytes":[1],"totalSize":1}]"#,
            r#"{"result":[[1],1,null,false]}"#,
            r#"{"result":{"bytes":[1],"totalSize":1},"error":"helper error"}"#,
            r#"{"result":{"bytes":[1],"totalSize":1},"result":{"bytes":[2],"totalSize":1}}"#,
            r#"{"result":{"bytes":[256],"totalSize":1}}"#,
            r#"{"result":{"bytes":[1],"totalSize":1,"partial":null}}"#,
            r#"{"result":{"bytes":[1]}}"#,
            r#"{"result":{"bytes":[1],"totalSize":1}} trailing"#,
        ] {
            assert_eq!(PreviewPayload::decode(&output(json, 0)), None, "{json}");
        }
    }
}
