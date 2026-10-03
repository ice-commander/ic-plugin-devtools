use devtools_core::{base64_tool, hash, json, Group, ToolSpec, TOOLS};

pub fn spec_of(id: &str) -> &'static ToolSpec {
    TOOLS.iter().find(|spec| spec.id == id).unwrap_or(&TOOLS[0])
}

pub fn is_hash(id: &str) -> bool {
    spec_of(id).group == Group::Hash
}

pub fn run(id: &str, bytes: &[u8]) -> Result<String, String> {
    let spec = spec_of(id);
    let text = || String::from_utf8_lossy(bytes).to_string();
    match spec.group {
        Group::Hash => {
            let algo = hash::Algo::from_id(spec.id).ok_or("unknown hash")?;
            Ok(hash::digest(algo, bytes))
        }
        Group::Encoding => match spec.id {
            "encoding.base64_encode" => {
                Ok(base64_tool::encode(base64_tool::Alphabet::Standard, bytes))
            }
            "encoding.hex_encode" => Ok(base64_tool::to_hex(bytes)),
            "encoding.base64_decode" => as_text(base64_tool::decode(&text())?),
            "encoding.hex_decode" => as_text(base64_tool::from_hex(&text())?),
            other => Err(format!("unknown encoder {other}")),
        },
        Group::Format => match spec.id {
            "format.json_pretty" => json::format(&text(), 2),
            "format.json_minify" => json::minify(&text()),
            other => Err(format!("unknown formatter {other}")),
        },
    }
}

fn as_text(raw: Vec<u8>) -> Result<String, String> {
    String::from_utf8(raw).map_err(|_| "not text".to_string())
}

pub fn verdict(id: &str, bytes: &[u8], expected: &str) -> Option<bool> {
    if expected.trim().is_empty() {
        return None;
    }
    let algo = hash::Algo::from_id(spec_of(id).id)?;
    Some(hash::matches(algo, bytes, expected.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_for(id: &str) -> &'static [u8] {
        match id {
            "encoding.base64_decode" => b"YWJj",
            "encoding.hex_decode" => b"616263",
            "format.json_pretty" | "format.json_minify" => br#"{"a":1}"#,
            _ => b"abc",
        }
    }

    #[test]
    fn every_declared_tool_runs_and_answers() {
        for spec in TOOLS {
            let outcome = run(spec.id, sample_for(spec.id));
            assert!(outcome.is_ok(), "{} failed: {outcome:?}", spec.id);
            assert!(
                !outcome.unwrap().is_empty(),
                "{} answered with nothing",
                spec.id
            );
        }
    }

    #[test]
    fn the_known_digests_are_the_known_digests() {
        assert_eq!(
            run("hash.md5", b"abc").expect("md5"),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            run("hash.sha256", b"abc").expect("sha256"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(run("encoding.base64_encode", b"abc").expect("b64"), "YWJj");
        assert_eq!(run("encoding.hex_encode", b"abc").expect("hex"), "616263");
    }

    #[test]
    fn a_decoder_reports_rubbish_rather_than_inventing_an_answer() {
        assert!(run("encoding.base64_decode", b"!!!not base64!!!").is_err());
        assert!(run("encoding.hex_decode", b"zz").is_err());
        assert!(run("format.json_pretty", b"{not json").is_err());
    }

    #[test]
    fn a_checksum_is_compared_only_when_one_was_offered() {
        assert_eq!(verdict("hash.md5", b"abc", "   "), None);
        assert_eq!(
            verdict("hash.md5", b"abc", "900150983CD24FB0D6963F7D28E17F72"),
            Some(true)
        );
        assert_eq!(verdict("hash.md5", b"abc", "deadbeef"), Some(false));
        assert_eq!(verdict("format.json_pretty", b"{}", "anything"), None);
    }

    #[test]
    fn only_the_checksum_tools_are_reported_as_hashes() {
        assert!(is_hash("hash.sha512"));
        assert!(is_hash("hash.crc32"));
        assert!(!is_hash("encoding.hex_encode"));
        assert!(!is_hash("format.json_pretty"));
    }

    #[test]
    fn an_unknown_tool_falls_back_to_the_first_one_instead_of_panicking() {
        assert_eq!(spec_of("nothing.at.all").id, TOOLS[0].id);
    }
}
