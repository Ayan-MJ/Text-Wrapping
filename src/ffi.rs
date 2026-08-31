use std::ptr;

use crate::{layout_json, normalized_placement_for_drag, LayoutUnit, Rect};

pub const ABI_VERSION: u32 = 1;

#[repr(C)]
pub struct LDFlowBuffer {
    pub data: *mut u8,
    pub len: usize,
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
    let result = layout_json(input).into_boxed_slice();
    let len = result.len();
    let data = Box::into_raw(result) as *mut u8;
    // SAFETY: output is required to reference writable storage.
    unsafe {
        output.write(LDFlowBuffer { data, len });
    }
    0
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
