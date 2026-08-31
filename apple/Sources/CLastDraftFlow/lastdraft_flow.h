#ifndef LASTDRAFT_FLOW_H
#define LASTDRAFT_FLOW_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define LD_FLOW_ABI_VERSION 1
#define LD_FLOW_STATUS_OK 0
#define LD_FLOW_STATUS_INVALID_ARGUMENT 1

typedef struct LDFlowBuffer {
    uint8_t *data;
    size_t len;
} LDFlowBuffer;

uint32_t ld_flow_abi_version(void);
uint8_t *ld_flow_bytes_alloc(size_t len);
void ld_flow_bytes_free(uint8_t *data, size_t len);

int32_t ld_flow_layout_json(
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

#ifdef __cplusplus
}
#endif

#endif

