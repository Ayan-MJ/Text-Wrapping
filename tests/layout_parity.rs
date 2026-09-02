//! Cross-platform layout parity.
//!
//! Reads the golden file the LastDraft repo freezes
//! (`src/lib/__fixtures__/scribe-layout-golden.json`) and asserts this engine
//! reproduces its geometry exactly, from the same requests and the same font
//! bytes — through the SAME C ABI the browser calls, not through an internal
//! shortcut.
//!
//! Web and native run this same crate, so identical input gives identical
//! output almost by construction. The value is that the golden pins the REQUEST
//! as well: a Swift or Kotlin adapter that builds a request even slightly
//! differently fails against this same file, and that is where a real
//! divergence would come from.
//!
//! The repo is resolved from `LASTDRAFT_REPO`, defaulting to the usual
//! checkout. The test skips with a clear message when it is not there, because
//! this crate must still build on a machine without it.

use std::path::PathBuf;

use lastdraft_flow::ffi::{
    ld_flow_abi_version, ld_flow_bytes_free, ld_flow_editor_snapshot_create_json,
    ld_flow_editor_snapshot_destroy, ld_flow_shaper_create, ld_flow_shaper_destroy,
    ld_flow_shaper_register_font, LDFlowBuffer, LDFlowEditorSnapshot,
};
use serde_json::Value;

fn take_json(buffer: LDFlowBuffer) -> Value {
    assert!(!buffer.data.is_null(), "ABI returned no JSON");
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) }.to_vec();
    unsafe { ld_flow_bytes_free(buffer.data, buffer.len) };
    serde_json::from_slice(&bytes).expect("ABI returned invalid JSON")
}

fn empty() -> LDFlowBuffer {
    LDFlowBuffer {
        data: std::ptr::null_mut(),
        len: 0,
    }
}

fn repo_root() -> Option<PathBuf> {
    let root = std::env::var("LASTDRAFT_REPO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("dev/lastdraft")
        });
    root.join("src/lib/__fixtures__/scribe-layout-golden.json")
        .exists()
        .then_some(root)
}

#[test]
fn engine_reproduces_the_frozen_layout_geometry() {
    let Some(root) = repo_root() else {
        eprintln!("skipping parity: set LASTDRAFT_REPO to the lastdraft checkout");
        return;
    };
    let golden: Value = serde_json::from_slice(
        &std::fs::read(root.join("src/lib/__fixtures__/scribe-layout-golden.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(
        golden["engineAbi"].as_u64().unwrap() as u32,
        ld_flow_abi_version(),
        "the golden was produced by a different ABI"
    );

    let cases = golden["cases"].as_array().expect("golden has cases");
    assert!(!cases.is_empty(), "golden has no cases");

    for case in cases {
        let name = case["name"].as_str().unwrap();
        let shaper = ld_flow_shaper_create();
        assert!(!shaper.is_null(), "{name}: no shaper");

        for font in case["fonts"].as_array().unwrap() {
            let id = font["id"].as_str().unwrap();
            let path = root.join(font["file"].as_str().unwrap());
            let bytes = std::fs::read(&path)
                .unwrap_or_else(|error| panic!("{name}: {}: {error}", path.display()));

            let mut output = empty();
            let status = unsafe {
                ld_flow_shaper_register_font(
                    shaper,
                    id.as_ptr(),
                    id.len(),
                    bytes.as_ptr(),
                    bytes.len(),
                    0,
                    &mut output,
                )
            };
            let response = take_json(output);
            assert_eq!(status, 0, "{name}: {id}: {response}");
            assert!(response["error"].is_null(), "{name}: {id}: {response}");
            // The golden pins the exact bytes each face was laid out with, so a
            // font that changed underneath it fails here rather than silently
            // moving every line break in every letter written against it.
            assert_eq!(
                response["font"]["sha256"].as_str().unwrap(),
                font["sha256"].as_str().unwrap(),
                "{name}: {id} is not the font the golden was made with"
            );
        }

        let request = serde_json::to_vec(&case["request"]).unwrap();
        let mut snapshot: *mut LDFlowEditorSnapshot = std::ptr::null_mut();
        let mut output = empty();
        let status = unsafe {
            ld_flow_editor_snapshot_create_json(
                shaper,
                request.as_ptr(),
                request.len(),
                &mut snapshot,
                &mut output,
            )
        };
        let response = take_json(output);
        assert_eq!(status, 0, "{name}: {response}");
        assert!(response["error"].is_null(), "{name}: {response}");
        assert_eq!(
            response["snapshot"]["layout"], case["geometry"],
            "{name}: geometry diverged from the golden"
        );

        if !snapshot.is_null() {
            unsafe { ld_flow_editor_snapshot_destroy(snapshot) };
        }
        unsafe { ld_flow_shaper_destroy(shaper) };
    }
}
