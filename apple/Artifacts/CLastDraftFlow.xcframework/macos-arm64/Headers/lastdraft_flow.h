#ifndef LASTDRAFT_FLOW_H
#define LASTDRAFT_FLOW_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define LD_FLOW_ABI_VERSION 4
#define LD_FLOW_STATUS_OK 0
#define LD_FLOW_STATUS_INVALID_ARGUMENT 1

typedef struct LDFlowBuffer {
    uint8_t *data;
    size_t len;
} LDFlowBuffer;

typedef struct LDFlowShaper LDFlowShaper;
typedef struct LDFlowEditorSnapshot LDFlowEditorSnapshot;

uint32_t ld_flow_abi_version(void);
uint8_t *ld_flow_bytes_alloc(size_t len);
void ld_flow_bytes_free(uint8_t *data, size_t len);

int32_t ld_flow_layout_json(
    const uint8_t *input,
    size_t input_len,
    LDFlowBuffer *output
);

LDFlowShaper *ld_flow_shaper_create(void);
void ld_flow_shaper_destroy(LDFlowShaper *shaper);

int32_t ld_flow_shaper_register_font(
    LDFlowShaper *shaper,
    const uint8_t *id,
    size_t id_len,
    const uint8_t *font_data,
    size_t font_len,
    uint32_t face_index,
    LDFlowBuffer *output
);

int32_t ld_flow_shaper_shape_json(
    const LDFlowShaper *shaper,
    const uint8_t *input,
    size_t input_len,
    LDFlowBuffer *output
);

int32_t ld_flow_editor_snapshot_create_json(
    const LDFlowShaper *shaper,
    const uint8_t *input,
    size_t input_len,
    LDFlowEditorSnapshot **snapshot,
    LDFlowBuffer *output
);

void ld_flow_editor_snapshot_destroy(LDFlowEditorSnapshot *snapshot);

int32_t ld_flow_editor_query_json(
    const LDFlowEditorSnapshot *snapshot,
    const uint8_t *input,
    size_t input_len,
    LDFlowBuffer *output
);

int32_t ld_flow_normalized_placement_for_drag(
    int32_t content_x_q26_6,
    int32_t content_width_q26_6,
    int32_t object_width_q26_6,
    int32_t anchor_reference_top_q26_6,
    int32_t line_height_q26_6,
    int32_t proposed_x_q26_6,
    int32_t proposed_y_q26_6,
    uint16_t *inline_u16,
    int32_t *block_offset1024
);

/*
 * ABI-v4 axis-aware drag conversion.
 * writing_mode: 0=horizontalTb, 1=verticalRl, 2=verticalLr.
 * The ABI-v3 horizontal function above remains supported.
 */
int32_t ld_flow_normalized_placement_for_drag_v4(
    int32_t content_x_q26_6,
    int32_t content_y_q26_6,
    int32_t content_width_q26_6,
    int32_t content_height_q26_6,
    int32_t object_width_q26_6,
    int32_t object_height_q26_6,
    int32_t anchor_reference_block_start_q26_6,
    int32_t line_height_q26_6,
    int32_t proposed_x_q26_6,
    int32_t proposed_y_q26_6,
    uint32_t writing_mode,
    uint16_t *inline_u16,
    int32_t *block_offset1024
);

#ifdef __cplusplus
}
#endif

#endif
