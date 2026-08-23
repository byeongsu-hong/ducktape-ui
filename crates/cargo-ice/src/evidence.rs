use serde::Deserialize;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

pub(super) const CAPTURE_SCHEMA_VERSION: u64 = 2;
pub(super) const REVIEW_SCHEMA_VERSION: u64 = 2;
pub(super) const REVIEW_ARTIFACT_KIND: &str = "ice_review_bundle";
pub(super) const CAPTURE_DIFF_ARTIFACT_KIND: &str = "ice_capture_diff";

const MAX_CAPTURE_PIXELS: u64 = 16_777_216;

pub(super) struct ValidatedCaptureManifest {
    pub(super) png: PathBuf,
}

pub(super) fn validate_capture_manifest(
    path: &Path,
    document: &Value,
) -> Result<ValidatedCaptureManifest, String> {
    let manifest = CaptureManifestV2::deserialize(document).map_err(|error| {
        format!(
            "{} is not a capture manifest v{CAPTURE_SCHEMA_VERSION}: {error}",
            path.display()
        )
    })?;
    manifest.validate(path)
}

#[derive(Deserialize)]
struct RequiredNullable<T>(Option<T>);

#[derive(Deserialize)]
struct CaptureManifestV2 {
    schema_version: u64,
    name: String,
    png: String,
    capture_source: CaptureSource,
    viewport: LogicalSize,
    physical_size: PhysicalSize,
    scale_factor: f64,
    configured_theme: RequiredNullable<ThemeMode>,
    resolved_theme: ResolvedTheme,
    system_theme: ThemeMode,
    locale: RequiredNullable<String>,
    platform: Platform,
    reduced_motion: RequiredNullable<bool>,
    window: WindowState,
    clock: ClockContract,
    targets: Vec<TargetManifest>,
}

impl CaptureManifestV2 {
    fn validate(self, path: &Path) -> Result<ValidatedCaptureManifest, String> {
        if self.schema_version != CAPTURE_SCHEMA_VERSION {
            return Err(format!(
                "{} uses unsupported capture manifest schema version {}; expected {}",
                path.display(),
                self.schema_version,
                CAPTURE_SCHEMA_VERSION
            ));
        }
        if self.name.is_empty() {
            return Err(format!("{} field `name` must be non-empty", path.display()));
        }
        if path.file_stem().and_then(|stem| stem.to_str()) != Some(&self.name) {
            return Err(format!(
                "{} capture name {:?} does not match its filename",
                path.display(),
                self.name
            ));
        }
        self.capture_source.validate(path, "capture_source")?;
        self.viewport.validate(path, "viewport")?;
        self.physical_size.validate(path)?;
        finite_positive(path, "scale_factor", self.scale_factor)?;
        if self
            .locale
            .0
            .as_deref()
            .is_some_and(|locale| locale.is_empty())
        {
            return Err(format!(
                "{} field `locale` must be null or non-empty",
                path.display()
            ));
        }
        if self.resolved_theme.name.is_empty() {
            return Err(format!(
                "{} field `resolved_theme.name` must be non-empty",
                path.display()
            ));
        }
        let _ = (
            self.configured_theme.0,
            self.resolved_theme.mode,
            self.system_theme,
            self.platform,
            self.reduced_motion.0,
        );
        self.window.validate(path)?;
        self.clock.validate(path)?;
        for (index, target) in self.targets.into_iter().enumerate() {
            target.validate(path, index)?;
        }
        let png = resolve_manifest_png(path, &self.png)?;
        Ok(ValidatedCaptureManifest { png })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ThemeMode {
    None,
    Light,
    Dark,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Platform {
    Linux,
    Windows,
    Macos,
    Wasm,
}

#[derive(Deserialize)]
struct CaptureSource {
    path: String,
    line: u64,
    column: u64,
    statement: String,
}

impl CaptureSource {
    fn validate(&self, manifest_path: &Path, field: &str) -> Result<(), String> {
        validate_source(manifest_path, field, &self.path, self.line, self.column)?;
        if self.statement.is_empty() {
            return Err(format!(
                "{} field `{field}.statement` must be non-empty",
                manifest_path.display()
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct SourceOrigin {
    path: String,
    line: u64,
    column: u64,
}

impl SourceOrigin {
    fn validate(&self, manifest_path: &Path, field: &str) -> Result<(), String> {
        validate_source(manifest_path, field, &self.path, self.line, self.column)
    }
}

fn validate_source(
    manifest_path: &Path,
    field: &str,
    source_path: &str,
    line: u64,
    column: u64,
) -> Result<(), String> {
    if source_path.is_empty() || line == 0 || column == 0 {
        return Err(format!(
            "{} field `{field}` requires a non-empty path and positive line/column",
            manifest_path.display()
        ));
    }
    Ok(())
}

#[derive(Deserialize)]
struct LogicalSize {
    width: f64,
    height: f64,
}

impl LogicalSize {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        finite_positive(path, &format!("{field}.width"), self.width)?;
        finite_positive(path, &format!("{field}.height"), self.height)
    }
}

#[derive(Deserialize)]
struct PhysicalSize {
    width: u64,
    height: u64,
}

impl PhysicalSize {
    fn validate(&self, path: &Path) -> Result<(), String> {
        let pixels = self
            .width
            .checked_mul(self.height)
            .filter(|pixels| self.width > 0 && self.height > 0 && *pixels <= MAX_CAPTURE_PIXELS)
            .ok_or_else(|| {
                format!(
                    "{} field `physical_size` must contain positive dimensions totaling at most {MAX_CAPTURE_PIXELS} pixels",
                    path.display()
                )
            })?;
        let _ = pixels;
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolvedTheme {
    mode: ThemeMode,
    name: String,
}

#[derive(Deserialize)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        finite(path, &format!("{field}.x"), self.x)?;
        finite(path, &format!("{field}.y"), self.y)
    }
}

#[derive(Deserialize)]
struct WindowState {
    position: RequiredNullable<Point>,
    focused: bool,
}

impl WindowState {
    fn validate(&self, path: &Path) -> Result<(), String> {
        if let Some(position) = &self.position.0 {
            position.validate(path, "window.position")?;
        }
        let _ = self.focused;
        Ok(())
    }
}

#[derive(Deserialize)]
struct ClockContract {
    supports_virtual_redraw_advance: bool,
    iced_timer_futures_are_virtual: bool,
}

impl ClockContract {
    fn validate(&self, path: &Path) -> Result<(), String> {
        if !self.supports_virtual_redraw_advance || self.iced_timer_futures_are_virtual {
            return Err(format!(
                "{} field `clock` violates the capture v{CAPTURE_SCHEMA_VERSION} clock contract",
                path.display()
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct TargetManifest {
    id: String,
    kind: String,
    source: RequiredNullable<SourceOrigin>,
    geometry: TargetGeometry,
    visible: VisibleGeometry,
    content: OptionalRectangle,
    translation: OptionalVector,
    scroll: OptionalVector,
    value: RequiredNullable<String>,
    focused: bool,
    accessibility: RequiredNullable<Accessibility>,
    paint: Paint,
}

impl TargetManifest {
    fn validate(self, path: &Path, index: usize) -> Result<(), String> {
        let field = format!("targets[{index}]");
        if self.id.is_empty() || self.kind.is_empty() {
            return Err(format!(
                "{} field `{field}` requires non-empty `id` and `kind`",
                path.display()
            ));
        }
        if let Some(source) = &self.source.0 {
            source.validate(path, &format!("{field}.source"))?;
        }
        self.geometry.validate(path, &format!("{field}.geometry"))?;
        self.visible.validate(path, &format!("{field}.visible"))?;
        self.content.validate(path, &format!("{field}.content"))?;
        self.translation
            .validate(path, &format!("{field}.translation"))?;
        self.scroll.validate(path, &format!("{field}.scroll"))?;
        if let Some(accessibility) = self.accessibility.0 {
            accessibility.validate(path, &format!("{field}.accessibility"))?;
        }
        self.paint.validate(path, &format!("{field}.paint"))?;
        let _ = (self.value.0, self.focused);
        Ok(())
    }
}

#[derive(Deserialize)]
struct TargetGeometry {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    center_x: f64,
    center_y: f64,
    pixel_aligned: bool,
}

impl TargetGeometry {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        for (name, value) in [
            ("x", self.x),
            ("y", self.y),
            ("width", self.width),
            ("height", self.height),
            ("left", self.left),
            ("top", self.top),
            ("right", self.right),
            ("bottom", self.bottom),
            ("center_x", self.center_x),
            ("center_y", self.center_y),
        ] {
            finite(path, &format!("{field}.{name}"), value)?;
        }
        let _ = self.pixel_aligned;
        Ok(())
    }
}

#[derive(Deserialize)]
struct VisibleGeometry {
    present: bool,
    x: RequiredNullable<f64>,
    y: RequiredNullable<f64>,
    width: RequiredNullable<f64>,
    height: RequiredNullable<f64>,
}

impl VisibleGeometry {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        for (name, value) in [
            ("x", self.x.0),
            ("y", self.y.0),
            ("width", self.width.0),
            ("height", self.height.0),
        ] {
            if let Some(value) = value {
                finite(path, &format!("{field}.{name}"), value)?;
            }
        }
        let _ = self.present;
        Ok(())
    }
}

#[derive(Deserialize)]
struct OptionalRectangle {
    x: RequiredNullable<f64>,
    y: RequiredNullable<f64>,
    width: RequiredNullable<f64>,
    height: RequiredNullable<f64>,
}

impl OptionalRectangle {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        for (name, value) in [
            ("x", self.x.0),
            ("y", self.y.0),
            ("width", self.width.0),
            ("height", self.height.0),
        ] {
            if let Some(value) = value {
                finite(path, &format!("{field}.{name}"), value)?;
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct OptionalVector {
    x: RequiredNullable<f64>,
    y: RequiredNullable<f64>,
}

impl OptionalVector {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        if let Some(x) = self.x.0 {
            finite(path, &format!("{field}.x"), x)?;
        }
        if let Some(y) = self.y.0 {
            finite(path, &format!("{field}.y"), y)?;
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct Accessibility {
    role: String,
    name: RequiredNullable<String>,
    description: RequiredNullable<String>,
    value: RequiredNullable<String>,
    checked: RequiredNullable<bool>,
    expanded: RequiredNullable<bool>,
    disabled: bool,
    focused: bool,
    actions: AccessibilityActions,
}

impl Accessibility {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        if self.role.is_empty() {
            return Err(format!(
                "{} field `{field}.role` must be non-empty",
                path.display()
            ));
        }
        let _ = (
            &self.name.0,
            &self.description.0,
            &self.value.0,
            self.checked.0,
            self.expanded.0,
            self.disabled,
            self.focused,
            self.actions.click,
            self.actions.focus,
        );
        Ok(())
    }
}

#[derive(Deserialize)]
struct AccessibilityActions {
    click: bool,
    focus: bool,
}

#[derive(Deserialize)]
struct Paint {
    available: bool,
    unavailable_reason: RequiredNullable<String>,
    surfaces: Vec<Surface>,
    texts: Vec<TextPaint>,
    images: Vec<Rectangle>,
}

impl Paint {
    fn validate(self, path: &Path, field: &str) -> Result<(), String> {
        for (index, surface) in self.surfaces.iter().enumerate() {
            surface.validate(path, &format!("{field}.surfaces[{index}]"))?;
        }
        for (index, text) in self.texts.iter().enumerate() {
            text.validate(path, &format!("{field}.texts[{index}]"))?;
        }
        for (index, image) in self.images.iter().enumerate() {
            image.validate(path, &format!("{field}.images[{index}]"))?;
        }
        let _ = (self.available, self.unavailable_reason.0);
        Ok(())
    }
}

#[derive(Deserialize)]
struct Rectangle {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rectangle {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        for (name, value) in [
            ("x", self.x),
            ("y", self.y),
            ("width", self.width),
            ("height", self.height),
        ] {
            finite(path, &format!("{field}.{name}"), value)?;
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct Surface {
    background: Background,
    border: Border,
    shadow: Shadow,
}

impl Surface {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        self.background
            .validate(path, &format!("{field}.background"))?;
        self.border.validate(path, &format!("{field}.border"))?;
        self.shadow.validate(path, &format!("{field}.shadow"))
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum Background {
    #[serde(rename = "color")]
    Color { color: Color },
    #[serde(rename = "linear-gradient")]
    LinearGradient {
        angle_radians: f64,
        stops: Vec<GradientStop>,
    },
}

impl Background {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        match self {
            Self::Color { color } => color.validate(path, &format!("{field}.color")),
            Self::LinearGradient {
                angle_radians,
                stops,
            } => {
                finite(path, &format!("{field}.angle_radians"), *angle_radians)?;
                for (index, stop) in stops.iter().enumerate() {
                    stop.validate(path, &format!("{field}.stops[{index}]"))?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Deserialize)]
struct GradientStop {
    offset: f64,
    color: Color,
}

impl GradientStop {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        finite(path, &format!("{field}.offset"), self.offset)?;
        self.color.validate(path, &format!("{field}.color"))
    }
}

#[derive(Deserialize)]
struct Color {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

impl Color {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        for (name, value) in [("r", self.r), ("g", self.g), ("b", self.b), ("a", self.a)] {
            finite(path, &format!("{field}.{name}"), value)?;
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct Border {
    color: Color,
    width: f64,
    radius: Radius,
}

impl Border {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        self.color.validate(path, &format!("{field}.color"))?;
        finite(path, &format!("{field}.width"), self.width)?;
        self.radius.validate(path, &format!("{field}.radius"))
    }
}

#[derive(Deserialize)]
struct Radius {
    top_left: f64,
    top_right: f64,
    bottom_right: f64,
    bottom_left: f64,
}

impl Radius {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        for (name, value) in [
            ("top_left", self.top_left),
            ("top_right", self.top_right),
            ("bottom_right", self.bottom_right),
            ("bottom_left", self.bottom_left),
        ] {
            finite(path, &format!("{field}.{name}"), value)?;
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct Shadow {
    color: Color,
    offset_x: f64,
    offset_y: f64,
    blur_radius: f64,
}

impl Shadow {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        self.color.validate(path, &format!("{field}.color"))?;
        finite(path, &format!("{field}.offset_x"), self.offset_x)?;
        finite(path, &format!("{field}.offset_y"), self.offset_y)?;
        finite(path, &format!("{field}.blur_radius"), self.blur_radius)
    }
}

#[derive(Deserialize)]
struct TextPaint {
    content: RequiredNullable<String>,
    bounds: Rectangle,
    color: Color,
    size: RequiredNullable<f64>,
    font: RequiredNullable<Font>,
    line_height: RequiredNullable<LineHeight>,
    baseline: RequiredNullable<f64>,
}

impl TextPaint {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        self.bounds.validate(path, &format!("{field}.bounds"))?;
        self.color.validate(path, &format!("{field}.color"))?;
        if let Some(size) = self.size.0 {
            finite(path, &format!("{field}.size"), size)?;
        }
        if let Some(font) = &self.font.0 {
            font.validate(path, &format!("{field}.font"))?;
        }
        if let Some(line_height) = &self.line_height.0 {
            line_height.validate(path, &format!("{field}.line_height"))?;
        }
        if let Some(baseline) = self.baseline.0 {
            finite(path, &format!("{field}.baseline"), baseline)?;
        }
        let _ = &self.content.0;
        Ok(())
    }
}

#[derive(Deserialize)]
struct Font {
    family: FontFamily,
    weight: String,
    stretch: String,
    style: String,
}

impl Font {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        if self.family.name.is_empty()
            || self.family.kind.is_empty()
            || self.weight.is_empty()
            || self.stretch.is_empty()
            || self.style.is_empty()
        {
            return Err(format!(
                "{} field `{field}` contains an empty font contract value",
                path.display()
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct FontFamily {
    kind: String,
    name: String,
}

#[derive(Deserialize)]
struct LineHeight {
    kind: String,
    value: f64,
}

impl LineHeight {
    fn validate(&self, path: &Path, field: &str) -> Result<(), String> {
        if !matches!(self.kind.as_str(), "relative" | "absolute") {
            return Err(format!(
                "{} field `{field}.kind` must be `relative` or `absolute`",
                path.display()
            ));
        }
        finite(path, &format!("{field}.value"), self.value)
    }
}

fn finite(path: &Path, field: &str, value: f64) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!("{} field `{field}` must be finite", path.display()))
    }
}

fn finite_positive(path: &Path, field: &str, value: f64) -> Result<(), String> {
    finite(path, field, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(format!(
            "{} field `{field}` must be positive",
            path.display()
        ))
    }
}

fn resolve_manifest_png(manifest_path: &Path, value: &str) -> Result<PathBuf, String> {
    let relative = Path::new(value);
    if relative.components().count() != 1
        || !matches!(relative.components().next(), Some(Component::Normal(_)))
    {
        return Err(format!(
            "{} field `png` must be a sibling basename",
            manifest_path.display()
        ));
    }
    let directory = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let directory = directory.canonicalize().map_err(|error| {
        format!(
            "cannot resolve capture directory {}: {error}",
            directory.display()
        )
    })?;
    let png = directory.join(relative).canonicalize().map_err(|error| {
        format!(
            "cannot resolve capture PNG {}: {error}",
            directory.join(relative).display()
        )
    })?;
    if !png.starts_with(&directory) || !png.is_file() {
        return Err(format!(
            "capture PNG must be a sibling file of {}",
            manifest_path.display()
        ));
    }
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    fn valid_manifest() -> Value {
        json!({
            "schema_version": CAPTURE_SCHEMA_VERSION,
            "name": "ready",
            "png": "ready.png",
            "capture_source": {
                "path": "app.ice", "line": 1, "column": 1, "statement": "capture ready"
            },
            "viewport": { "width": 320.0, "height": 240.0 },
            "physical_size": { "width": 320, "height": 240 },
            "scale_factor": 1.0,
            "configured_theme": null,
            "resolved_theme": { "mode": "light", "name": "Light" },
            "system_theme": "none",
            "locale": null,
            "platform": "linux",
            "reduced_motion": null,
            "window": { "position": null, "focused": true },
            "clock": {
                "supports_virtual_redraw_advance": true,
                "iced_timer_futures_are_virtual": false
            },
            "targets": [],
        })
    }

    #[test]
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn allocation_contract_capture_manifest_deserialization_borrows_json() {
        let manifest = valid_manifest();

        const EXPECTED: (u64, u64) = (6, 123);
        let mut result = None;
        let _profiler = dhat::Profiler::builder().testing().build();
        let measured = crate::allocation::clean_window(EXPECTED.0, || {
            result = Some(validate_capture_manifest(
                Path::new("other.json"),
                &manifest,
            ));
        });

        assert!(
            result
                .expect("validated")
                .is_err_and(|error| error.contains("does not match its filename"))
        );
        assert_eq!(
            measured, EXPECTED,
            "manifest validation allocations: {measured:?}"
        );
        eprintln!(
            "capture manifest validation: {} heap blocks / {} bytes",
            measured.0, measured.1
        );
    }

    #[test]
    fn canonical_capture_validator_rejects_partial_and_mistyped_documents() {
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("ready.json");
        fs::write(fixture.path().join("ready.png"), []).unwrap();
        let manifest = valid_manifest();
        assert!(validate_capture_manifest(&path, &manifest).is_ok());

        for invalid in [
            json!({ "schema_version": 2, "png": "ready.png" }),
            {
                let mut value = manifest.clone();
                value["viewport"]["width"] = json!("320");
                value
            },
            {
                let mut value = manifest.clone();
                value.as_object_mut().unwrap().remove("capture_source");
                value
            },
            {
                let mut value = manifest.clone();
                value["clock"]["supports_virtual_redraw_advance"] = json!(false);
                value
            },
        ] {
            assert!(validate_capture_manifest(&path, &invalid).is_err());
        }
    }

    #[test]
    fn capture_png_must_be_an_existing_sibling_basename() {
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("ready.json");
        fs::write(fixture.path().join("ready.png"), []).unwrap();
        assert!(validate_capture_manifest(&path, &valid_manifest()).is_ok());

        for escaped in ["../ready.png", "nested/ready.png", "/ready.png"] {
            let mut manifest = valid_manifest();
            manifest["png"] = json!(escaped);
            assert!(validate_capture_manifest(&path, &manifest).is_err());
        }
    }
}
