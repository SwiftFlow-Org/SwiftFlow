use naga::valid::{Capabilities, ValidationFlags, Validator};

fn validate(name: &str, source: &str) {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("{name} failed to parse:\n{}", e.emit_to_string(source)));

    Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name} failed validation:\n{e:?}"));
}

#[test]
fn shader_wgsl_is_valid() {
    validate("shader.wgsl", include_str!("../src/shader.wgsl"));
}

#[test]
fn text_shader_wgsl_is_valid() {
    validate("text_shader.wgsl", include_str!("../src/text_shader.wgsl"));
}

#[test]
fn glass_preview_wgsl_is_valid() {
    validate("glass_preview.wgsl", include_str!("../src/glass_preview.wgsl"));
}

#[test]
fn the_preview_shares_the_shipped_shaders_glass_constants() {
    fn glass_constants(source: &str) -> Vec<String> {
        source
            .lines()
            .map(str::trim)
            .filter(|line| {
                ["GLASS_", "RIM_", "FRESNEL_", "GLARE_", "SHADOW_"]
                    .iter()
                    .any(|family| line.starts_with(&format!("const {family}")))
            })

            .map(|line| line.split("//").next().unwrap_or(line))

            .map(|line| line.chars().filter(|c| !c.is_whitespace()).collect::<String>())
            .collect()
    }

    let shipped = glass_constants(include_str!("../src/shader.wgsl"));
    let preview = glass_constants(include_str!("../src/glass_preview.wgsl"));

    assert!(
        !shipped.is_empty(),
        "found no glass constants in shader.wgsl — this test stopped testing anything"
    );
    for constant in &shipped {
        assert!(
            preview.contains(constant),
            "the preview is missing or has changed `{constant}`, so it no longer \
             shows what the app does"
        );
    }
}
