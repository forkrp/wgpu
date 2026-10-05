// language: metal1.0
#include <metal_stdlib>
#include <simd/simd.h>

using metal::uint;

struct FragmentOutput {
    metal::float4 color;
    uint mask;
    char _pad2[12];
};

void helper(
) {
    return;
}

struct colorOutput {
    metal::float4 member [[color(0)]];
    uint _pipeline_sample_mask [[sample_mask]];
};
fragment colorOutput color(
) {
    return colorOutput { metal::float4(1.0), 2u };
}


struct maskOutput {
    uint member_1 [[sample_mask]];
};
fragment maskOutput mask(
) {
    return maskOutput { 3u & 2u };
}


struct combinedOutput {
    metal::float4 color [[color(0)]];
    uint mask [[sample_mask]];
};
fragment combinedOutput combined(
) {
    const auto _tmp = FragmentOutput {metal::float4(1.0), 3u};
    return combinedOutput { _tmp.color, _tmp.mask & 2u };
}


struct emptyOutput {
    uint _pipeline_sample_mask [[sample_mask]];
};
fragment emptyOutput empty(
) {
    return emptyOutput { 2u };
}


struct branchingInput {
};
struct branchingOutput {
    uint _pipeline_sample_mask [[sample_mask]];
};
fragment branchingOutput branching(
  bool front [[front_facing]]
) {
    helper();
    if (front) {
        return branchingOutput { 2u };
    } else {
        return branchingOutput { 2u };
    }
}
