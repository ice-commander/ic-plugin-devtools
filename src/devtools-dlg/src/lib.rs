pub mod compute;

use devtools_core::TOOLS;
use ic_plugin_api::{
    check_host, needs_plugin_assets, HostCheck, IcBytes, IcHost, IC_ABI_VERSION, IC_ERR_HOST_TOO_OLD,
    IC_ERR_HOST_UNKNOWN, IC_ERR_INIT_FAILED, IC_SIDE_LEFT,
};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};

pub const VIEW_ID: &str = "devtools";
const ICON: &str = include_str!("../assets/toolbox.svg");

const LOCALES: &[(&str, &str)] = &[
    ("en", include_str!("../locales/en.json")),
    ("ru", include_str!("../locales/ru.json")),
    ("pl", include_str!("../locales/pl.json")),
    ("cs", include_str!("../locales/cs.json")),
    ("sk", include_str!("../locales/sk.json")),
    ("de", include_str!("../locales/de.json")),
    ("es", include_str!("../locales/es.json")),
    ("uk", include_str!("../locales/uk.json")),
    ("it", include_str!("../locales/it.json")),
    ("fr", include_str!("../locales/fr.json")),
    ("ro", include_str!("../locales/ro.json")),
    ("hu", include_str!("../locales/hu.json")),
    ("be", include_str!("../locales/be.json")),
    ("bg", include_str!("../locales/bg.json")),
    ("sr", include_str!("../locales/sr.json")),
];

static HOST: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static ANSWER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn hash_ids() -> Vec<&'static str> {
    TOOLS
        .iter()
        .filter(|spec| compute::is_hash(spec.id))
        .map(|spec| spec.id)
        .collect()
}

fn tool_options() -> Vec<Value> {
    TOOLS
        .iter()
        .map(|spec| {
            json!({
                "value": spec.id,
                "label": { "tr": format!("devtools.{}", spec.id), "en": spec.id }
            })
        })
        .collect()
}

pub fn document() -> Value {
    json!({
        "schema": 1,
        "fields": [
            { "bind": "tool", "type": "text", "default": TOOLS[0].id, "derive": "commit_only" },
            { "bind": "input", "type": "text" },
            { "bind": "output", "type": "text" },
            { "bind": "expected", "type": "text" }
        ],
        "form": {
            "t": "view",
            "surface": "dialog",
            "scroll": "vertical",
            "spacing": 12,
            "padding": 24,
            "width": 860,
            "height": 660,
            "children": [
                { "t": "group", "id": "picker", "children": [
                    { "t": "choice", "id": "tool", "bind": "tool", "chrome": "row",
                      "title": { "tr": "devtools.toolbar_tooltip", "en": "Developer Toolbox" },
                      "options": tool_options(),
                      "decode": { "case_insensitive": true, "unknown": TOOLS[0].id } } ] },

                { "t": "text", "id": "input_label", "role": "dim", "margin_top": 12,
                  "text": { "tr": "devtools.input", "en": "Input" } },
                { "t": "input", "id": "input", "bind": "input", "variant": "multiline",
                  "height": 150, "emit": "change", "debounce_ms": 120 },

                { "t": "row", "id": "controls", "spacing": 8, "children": [
                    { "t": "text", "id": "size", "role": "dim", "weight": 1,
                      "text": "{data.size}" },
                    { "t": "button", "id": "copy", "role": "flat",
                      "label": { "tr": "devtools.copy", "en": "Copy" },
                      "intent": { "do": "emit", "node": "copy" } } ] },

                { "t": "text", "id": "output_label", "role": "dim",
                  "text": { "tr": "devtools.output", "en": "Result" } },
                { "t": "input", "id": "output", "bind": "output", "variant": "multiline",
                  "height": 150, "read_only": true },

                { "t": "column", "id": "compare", "spacing": 8, "margin_top": 12,
                  "visible": { "one_of": { "value": "state.tool", "of": hash_ids() } },
                  "children": [
                    { "t": "text", "id": "expected_label", "role": "dim",
                      "text": { "tr": "devtools.expected", "en": "Compare with" } },
                    { "t": "input", "id": "expected", "bind": "expected", "chrome": "bare",
                      "emit": "change", "debounce_ms": 120,
                      "placeholder": { "tr": "devtools.expected_placeholder",
                                       "en": "paste a checksum to compare" } } ] },

                { "t": "text", "id": "status", "margin_top": 12,
                  "role": { "cases": [
                      { "when": { "truthy": "data.failed" }, "then": "error" } ],
                    "else": "dim" },
                  "text": "{data.status}" }
            ]
        }
    })
}

fn answer_with(source: &str) -> IcBytes {
    ANSWER.with(|slot| {
        *slot.borrow_mut() = source.as_bytes().to_vec();
        let held = slot.borrow();
        IcBytes {
            data: held.as_ptr(),
            len: held.len() as u64,
        }
    })
}

fn spell_size(bytes: usize) -> String {
    format!("{bytes} B")
}

pub fn reply_for(event: &Value) -> Value {
    let values = &event["values"];
    let tool = values["tool"].as_str().unwrap_or(TOOLS[0].id);
    let input = values["input"].as_str().unwrap_or_default();
    let expected = values["expected"].as_str().unwrap_or_default();
    let bytes = input.as_bytes();

    if event["type"] == json!("activate") && event["node"] == json!("copy") {
        let held = values["output"].as_str().unwrap_or_default().to_string();
        if held.is_empty() {
            return json!({});
        }
        return json!({
            "clipboard": held,
            "set": { "data.status": { "tr": "devtools.copied", "en": "Copied" }, "data.failed": false }
        });
    }

    let mut set = serde_json::Map::new();
    set.insert("data.size".to_string(), json!(spell_size(bytes.len())));
    match compute::run(tool, bytes) {
        Ok(result) => {
            let status = match compute::verdict(tool, bytes, expected) {
                Some(true) => json!({ "tr": "devtools.sum_matches", "en": "matches" }),
                Some(false) => json!({ "tr": "devtools.sum_differs", "en": "does not match" }),
                None => json!(""),
            };
            let failed = compute::verdict(tool, bytes, expected) == Some(false);
            set.insert("data.status".to_string(), status);
            set.insert("data.failed".to_string(), json!(failed));
            json!({ "put": { "output": result }, "set": Value::Object(set) })
        }
        Err(reason) => {
            set.insert("data.status".to_string(), json!(reason));
            set.insert("data.failed".to_string(), json!(true));
            json!({ "put": { "output": "" }, "set": Value::Object(set) })
        }
    }
}

extern "C" fn describe(_ctx: *const u8, _ctx_len: u64, _user_data: *mut c_void) -> IcBytes {
    answer_with(&document().to_string())
}

extern "C" fn on_event(event: *const u8, len: u64, _user_data: *mut c_void) -> IcBytes {
    if event.is_null() || len == 0 {
        return answer_with("{}");
    }
    let raw = unsafe { std::slice::from_raw_parts(event, len as usize) };
    let parsed: Value = serde_json::from_slice(raw).unwrap_or(Value::Null);
    answer_with(&reply_for(&parsed).to_string())
}

extern "C" fn on_clicked(_user_data: *mut c_void, _parent: *mut c_void) {
    let host = HOST.load(Ordering::Relaxed) as *const IcHost;
    if host.is_null() {
        return;
    }
    let Ok(id) = CString::new(VIEW_ID) else {
        return;
    };
    unsafe {
        ((*host).open_view)(id.as_ptr(), std::ptr::null(), 0);
    }
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../version.rs"));

ic_plugin_api::declare_about!(
    "ic-devtools-dlg",
    "Developer Toolbox",
    plugins_version!(),
    "Hashes, encodings and formatting, in one window"
);

#[cfg_attr(feature = "export-abi", no_mangle)]
pub extern "C" fn ic_plugin_init(host: *const IcHost, _kind: *const c_char) -> c_int {
    match check_host(host, IC_ABI_VERSION, needs_plugin_assets()) {
        HostCheck::Ok => {}
        HostCheck::WrongMagic => return IC_ERR_HOST_UNKNOWN,
        HostCheck::TooOld { .. } | HostCheck::Truncated { .. } => return IC_ERR_HOST_TOO_OLD,
    }
    HOST.store(host as usize, Ordering::Relaxed);
    for (language, catalogue) in LOCALES {
        let Ok(tag) = CString::new(*language) else {
            continue;
        };
        unsafe {
            ((*host).register_locales)(tag.as_ptr(), catalogue.as_ptr(), catalogue.len() as u64);
        }
    }
    let (Ok(id), Ok(svg), Ok(tooltip)) = (
        CString::new(VIEW_ID),
        CString::new(ICON),
        CString::new("devtools.window_title"),
    ) else {
        return IC_ERR_INIT_FAILED;
    };
    let table = ic_plugin_api::IcViewVTable {
        struct_size: std::mem::size_of::<ic_plugin_api::IcViewVTable>() as u32,
        describe,
        on_event: Some(on_event),
        closed: None,
    };
    let registered = unsafe {
        ((*host).register_view)(id.as_ptr(), tooltip.as_ptr(), &table, std::ptr::null_mut())
    };
    if registered != ic_plugin_api::IC_OK {
        return registered;
    }
    let Ok(label) = CString::new("") else {
        return IC_ERR_INIT_FAILED;
    };
    unsafe {
        ((*host).add_header_button)(
            id.as_ptr(),
            svg.as_ptr(),
            label.as_ptr(),
            tooltip.as_ptr(),
            IC_SIDE_LEFT,
            11,
            on_clicked,
            std::ptr::null_mut(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plugin_refuses_a_host_it_does_not_recognise() {
        assert_eq!(
            ic_plugin_init(std::ptr::null(), std::ptr::null()),
            IC_ERR_HOST_UNKNOWN
        );
    }
}
