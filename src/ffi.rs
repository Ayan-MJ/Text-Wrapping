use std::ptr;

use crate::editor_wire::{editor_query_json, editor_snapshot_from_json, editor_snapshot_json};
use crate::rich_wire::{font_registration_json, shape_paragraph_json};
use crate::{
    layout_json, normalized_placement_for_drag, normalized_placement_for_drag_in_mode,
    CanonicalShaper, EditorGeometrySnapshot, LayoutUnit, Rect, WritingMode,
};

// v5 adds paragraph alignment and indentation to the layout request. Both
// default, so a request that never mentions them is byte-identical to a v4 one
// -- but a request that DOES carry them is rejected by a v4 engine rather than
// quietly mislaid, which is exactly what a version number is for.
// v6 makes exclusion margins LOGICAL on the wire (`marginBefore`/`marginAfter`
// replace `marginTop`/`marginBottom`) and resolves them to physical sides
// inside the engine. That resolution cannot live in a platform adapter: with
// `baseDirection: auto` the adapter does not yet know whether a paragraph is
// ltr or rtl, because the engine is what runs the bidi algorithm.
pub const ABI_VERSION: u32 = 6;

#[repr(C)]
pub struct LDFlowBuffer {
    pub data: *mut u8,
    pub len: usize,
}

#[repr(C)]
pub struct LDFlowEditorSnapshot {
    inner: EditorGeometrySnapshot,
}

/// Returns the stable host ABI version.
#[no_mangle]
pub extern "C" fn ld_flow_abi_version() -> u32 {
    ABI_VERSION
}

/// Allocates a zero-initialized byte buffer in engine memory.
#[no_mangle]
pub extern "C" fn ld_flow_bytes_alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return ptr::null_mut();
    }
    let boxed = vec![0_u8; len].into_boxed_slice();
    Box::into_raw(boxed) as *mut u8
}

/// Releases a byte buffer returned by this library.
///
/// # Safety
/// `data` and `len` must describe one currently allocated buffer returned by
/// `ld_flow_bytes_alloc` or `ld_flow_layout_json` and must be freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn ld_flow_bytes_free(data: *mut u8, len: usize) {
    if data.is_null() || len == 0 {
        return;
    }
    // SAFETY: required by the public contract above.
    unsafe {
        drop(Box::from_raw(ptr::slice_from_raw_parts_mut(data, len)));
    }
}

/// Runs flow layout from the v1 JSON request and returns a v1 JSON envelope.
/// Returns 0 on a valid ABI call; validation failures are inside the envelope.
///
/// # Safety
/// `input` must point to `input_len` readable bytes and `output` must point to a
/// writable `LDFlowBuffer`. The caller owns the returned buffer and must free it.
#[no_mangle]
pub unsafe extern "C" fn ld_flow_layout_json(
    input: *const u8,
    input_len: usize,
    output: *mut LDFlowBuffer,
) -> i32 {
    if input.is_null() || input_len == 0 || output.is_null() {
        return 1;
    }
    // SAFETY: pointers were validated for null; remaining requirements belong
    // to the caller as documented by this exported ABI.
    let input = unsafe { std::slice::from_raw_parts(input, input_len) };
    write_output(layout_json(input), output)
}

#[no_mangle]
pub extern "C" fn ld_flow_shaper_create() -> *mut CanonicalShaper {
    Box::into_raw(Box::new(CanonicalShaper::new()))
}

/// Releases a canonical shaper and all registered font bytes.
///
/// # Safety
/// shaper must be null or a live pointer returned by ld_flow_shaper_create,
/// and each non-null pointer must be destroyed exactly once.
#[no_mangle]
pub unsafe extern "C" fn ld_flow_shaper_destroy(shaper: *mut CanonicalShaper) {
    if !shaper.is_null() {
        // SAFETY: required by the public contract above.
        unsafe {
            drop(Box::from_raw(shaper));
        }
    }
}

/// Registers one immutable OpenType font face and returns a JSON envelope.
///
/// # Safety
/// All input pointers must reference their declared readable lengths, shaper
/// must be live, and output must point to writable LDFlowBuffer storage.
#[no_mangle]
pub unsafe extern "C" fn ld_flow_shaper_register_font(
    shaper: *mut CanonicalShaper,
    id: *const u8,
    id_len: usize,
    font_data: *const u8,
    font_len: usize,
    face_index: u32,
    output: *mut LDFlowBuffer,
) -> i32 {
    if shaper.is_null()
        || id.is_null()
        || id_len == 0
        || font_data.is_null()
        || font_len == 0
        || output.is_null()
    {
        return 1;
    }
    // SAFETY: remaining pointer validity is required by the public contract.
    let id_bytes = unsafe { std::slice::from_raw_parts(id, id_len) };
    let font_bytes = unsafe { std::slice::from_raw_parts(font_data, font_len) };
    let id = match std::str::from_utf8(id_bytes) {
        Ok(id) => id,
        Err(_) => return 1,
    };
    // SAFETY: shaper is required to be a unique live pointer for this call.
    let shaper = unsafe { &mut *shaper };
    let result = shaper.register_font(id, std::sync::Arc::<[u8]>::from(font_bytes), face_index);
    write_output(font_registration_json(result), output)
}

/// Shapes one rich paragraph using previously registered font bytes.
///
/// # Safety
/// shaper must be live, input must reference input_len readable bytes, and
/// output must point to writable LDFlowBuffer storage.
#[no_mangle]
pub unsafe extern "C" fn ld_flow_shaper_shape_json(
    shaper: *const CanonicalShaper,
    input: *const u8,
    input_len: usize,
    output: *mut LDFlowBuffer,
) -> i32 {
    if shaper.is_null() || input.is_null() || input_len == 0 || output.is_null() {
        return 1;
    }
    // SAFETY: remaining pointer validity is required by the public contract.
    let shaper = unsafe { &*shaper };
    let input = unsafe { std::slice::from_raw_parts(input, input_len) };
    write_output(shape_paragraph_json(shaper, input), output)
}

/// Shapes and lays out a rich document, returning both an immutable geometry
/// snapshot handle and its complete page-coordinate JSON representation.
/// Validation failures are returned in the JSON envelope with a null handle.
///
/// # Safety
/// shaper must be live; input must reference input_len readable bytes; snapshot
/// and output must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn ld_flow_editor_snapshot_create_json(
    shaper: *const CanonicalShaper,
    input: *const u8,
    input_len: usize,
    snapshot: *mut *mut LDFlowEditorSnapshot,
    output: *mut LDFlowBuffer,
) -> i32 {
    if shaper.is_null()
        || input.is_null()
        || input_len == 0
        || snapshot.is_null()
        || output.is_null()
    {
        return 1;
    }
    let shaper = unsafe { &*shaper };
    let input = unsafe { std::slice::from_raw_parts(input, input_len) };
    match editor_snapshot_from_json(shaper, input) {
        Ok(inner) => {
            let response = editor_snapshot_json(Ok(&inner));
            let handle = Box::into_raw(Box::new(LDFlowEditorSnapshot { inner }));
            unsafe { snapshot.write(handle) };
            write_output(response, output)
        }
        Err(error) => {
            unsafe { snapshot.write(ptr::null_mut()) };
            write_output(editor_snapshot_json(Err(error)), output)
        }
    }
}

/// Releases an immutable editor geometry snapshot.
///
/// # Safety
/// snapshot must be null or a live pointer returned by
/// ld_flow_editor_snapshot_create_json, destroyed exactly once.
#[no_mangle]
pub unsafe extern "C" fn ld_flow_editor_snapshot_destroy(snapshot: *mut LDFlowEditorSnapshot) {
    if !snapshot.is_null() {
        unsafe { drop(Box::from_raw(snapshot)) };
    }
}

/// Runs a caret, hit-test, selection, or movement query against one immutable
/// snapshot. Query failures are represented in the returned JSON envelope.
///
/// # Safety
/// snapshot must be live; input must reference input_len readable bytes; output
/// must point to writable LDFlowBuffer storage.
#[no_mangle]
pub unsafe extern "C" fn ld_flow_editor_query_json(
    snapshot: *const LDFlowEditorSnapshot,
    input: *const u8,
    input_len: usize,
    output: *mut LDFlowBuffer,
) -> i32 {
    if snapshot.is_null() || input.is_null() || input_len == 0 || output.is_null() {
        return 1;
    }
    let snapshot = unsafe { &*snapshot };
    let input = unsafe { std::slice::from_raw_parts(input, input_len) };
    write_output(editor_query_json(&snapshot.inner, input), output)
}

/// Converts a dragged origin into persisted anchor-local placement.
///
/// # Safety
/// `inline_u16` and `block_offset1024` must be valid writable pointers.
#[no_mangle]
pub unsafe extern "C" fn ld_flow_normalized_placement_for_drag(
    content_x_q26_6: i32,
    content_width_q26_6: i32,
    object_width_q26_6: i32,
    anchor_reference_top_q26_6: i32,
    line_height_q26_6: i32,
    proposed_x_q26_6: i32,
    proposed_y_q26_6: i32,
    inline_u16: *mut u16,
    block_offset1024: *mut i32,
) -> i32 {
    if inline_u16.is_null() || block_offset1024.is_null() {
        return 1;
    }
    let placement = normalized_placement_for_drag(
        Rect::new(
            LayoutUnit::from_raw(content_x_q26_6),
            LayoutUnit::ZERO,
            LayoutUnit::from_raw(content_width_q26_6),
            LayoutUnit::MAX,
        ),
        LayoutUnit::from_raw(object_width_q26_6),
        LayoutUnit::from_raw(anchor_reference_top_q26_6),
        LayoutUnit::from_raw(line_height_q26_6),
        LayoutUnit::from_raw(proposed_x_q26_6),
        LayoutUnit::from_raw(proposed_y_q26_6),
    );
    // SAFETY: output pointers were checked and are required to be writable.
    unsafe {
        inline_u16.write(placement.inline_position.raw());
        block_offset1024.write(placement.block_offset);
    }
    0
}

/// ABI-v4 axis-aware drag conversion.
///
/// writing_mode is 0=horizontalTb, 1=verticalRl, 2=verticalLr.
///
/// # Safety
/// `inline_u16` and `block_offset1024` must be valid writable pointers.
#[no_mangle]
pub unsafe extern "C" fn ld_flow_normalized_placement_for_drag_v4(
    content_x_q26_6: i32,
    content_y_q26_6: i32,
    content_width_q26_6: i32,
    content_height_q26_6: i32,
    object_width_q26_6: i32,
    object_height_q26_6: i32,
    anchor_reference_block_start_q26_6: i32,
    line_height_q26_6: i32,
    proposed_x_q26_6: i32,
    proposed_y_q26_6: i32,
    writing_mode: u32,
    inline_u16: *mut u16,
    block_offset1024: *mut i32,
) -> i32 {
    if inline_u16.is_null() || block_offset1024.is_null() {
        return 1;
    }
    let writing_mode = match writing_mode {
        0 => WritingMode::HorizontalTb,
        1 => WritingMode::VerticalRl,
        2 => WritingMode::VerticalLr,
        _ => return 1,
    };
    let placement = normalized_placement_for_drag_in_mode(
        Rect::new(
            LayoutUnit::from_raw(content_x_q26_6),
            LayoutUnit::from_raw(content_y_q26_6),
            LayoutUnit::from_raw(content_width_q26_6),
            LayoutUnit::from_raw(content_height_q26_6),
        ),
        LayoutUnit::from_raw(object_width_q26_6),
        LayoutUnit::from_raw(object_height_q26_6),
        LayoutUnit::from_raw(anchor_reference_block_start_q26_6),
        LayoutUnit::from_raw(line_height_q26_6),
        LayoutUnit::from_raw(proposed_x_q26_6),
        LayoutUnit::from_raw(proposed_y_q26_6),
        writing_mode,
    );
    unsafe {
        inline_u16.write(placement.inline_position.raw());
        block_offset1024.write(placement.block_offset);
    }
    0
}

fn write_output(bytes: Vec<u8>, output: *mut LDFlowBuffer) -> i32 {
    let result = bytes.into_boxed_slice();
    let len = result.len();
    let data = Box::into_raw(result) as *mut u8;
    // SAFETY: every caller checks output and requires writable storage.
    unsafe {
        output.write(LDFlowBuffer { data, len });
    }
    0
}
