struct FragmentOutput {
    @location(0) color: vec4f,
    @builtin(sample_mask) mask: u32,
}

@fragment fn color() -> @location(0) vec4f {
    return vec4f(1.0);
}

@fragment fn mask() -> @builtin(sample_mask) u32 {
    return 3u;
}

@fragment fn combined() -> FragmentOutput {
    return FragmentOutput(vec4f(1.0), 3u);
}

@fragment fn empty() {}

fn helper() {}

@fragment fn branching(@builtin(front_facing) front: bool) {
    helper();
    if front { return; }
}
