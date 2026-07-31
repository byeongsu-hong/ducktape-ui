use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use ui_lang_core::{
    ApiComponent, ApiComponentEvent, ApiComponentProp, ApiComponentSlot, ApiPropAccess, ApiRecipe,
    ApiSurface, ApiThemeContract,
};

const FINGERPRINT_SCHEMA_VERSION: u32 = 1;
const DIFF_SCHEMA_VERSION: u32 = 1;

pub(crate) fn valid_args(args: &[String]) -> bool {
    match args {
        [source] => source != "diff",
        [command, _, _] => command == "diff",
        [command, _, _, flag, format] => {
            command == "diff" && flag == "--format" && matches!(format.as_str(), "human" | "json")
        }
        _ => false,
    }
}

pub(crate) fn run(root: &Path, args: &[String]) -> Result<(), String> {
    match args {
        [source] if source != "diff" => emit_fingerprint(&root.join(source)),
        [command, baseline, current] if command == "diff" => diff_files(
            &root.join(baseline),
            &root.join(current),
            ReportFormat::Human,
        ),
        [command, baseline, current, flag, format]
            if command == "diff" && flag == "--format" =>
        {
            let format = match format.as_str() {
                "human" => ReportFormat::Human,
                "json" => ReportFormat::Json,
                _ => return Err("API diff format must be `human` or `json`".into()),
            };
            diff_files(&root.join(baseline), &root.join(current), format)
        }
        _ => Err(
            "usage: cargo ice api <root.ice> | cargo ice api diff <baseline.json> <current.json> [--format human|json]"
                .into(),
        ),
    }
}

fn emit_fingerprint(source: &Path) -> Result<(), String> {
    let api = ui_lang_core::analyze_api_file(source)
        .map_err(|error| error.render(&source.display().to_string()))?;
    let package = package_for_source(source)?;
    let document = FingerprintDocument::new(package, api)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn diff_files(baseline: &Path, current: &Path, format: ReportFormat) -> Result<(), String> {
    let baseline = read_fingerprint(baseline)?;
    let current = read_fingerprint(current)?;
    let report = diff(&baseline, &current);
    match format {
        ReportFormat::Human => print!("{}", report.render_human()),
        ReportFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
        ),
    }
    if report.summary.breaking > 0 {
        Err(format!(
            "Ice API diff contains {} breaking change(s)",
            report.summary.breaking
        ))
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ReportFormat {
    Human,
    Json,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApiPackage {
    name: String,
    version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FingerprintDocument {
    schema_version: u32,
    fingerprint: String,
    language_revision: String,
    package: ApiPackage,
    api: ApiSurface,
}

#[derive(Serialize)]
struct FingerprintPayload<'a> {
    schema_version: u32,
    language_revision: &'a str,
    package: &'a ApiPackage,
    api: &'a ApiSurface,
}

impl FingerprintDocument {
    fn new(package: ApiPackage, api: ApiSurface) -> Result<Self, String> {
        let mut document = Self {
            schema_version: FINGERPRINT_SCHEMA_VERSION,
            fingerprint: String::new(),
            language_revision: ui_lang_core::LANGUAGE_REVISION.into(),
            package,
            api,
        };
        document.fingerprint = document.expected_fingerprint()?;
        Ok(document)
    }

    fn expected_fingerprint(&self) -> Result<String, String> {
        let payload = FingerprintPayload {
            schema_version: self.schema_version,
            language_revision: &self.language_revision,
            package: &self.package,
            api: &self.api,
        };
        let bytes = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

fn read_fingerprint(path: &Path) -> Result<FingerprintDocument, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read API fingerprint `{}`: {error}", path.display()))?;
    let value = serde_json::from_str::<serde_json::Value>(&source).map_err(|error| {
        format!(
            "malformed API fingerprint `{}` at line {}, column {}: {error}",
            path.display(),
            error.line(),
            error.column()
        )
    })?;
    let version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            format!(
                "malformed API fingerprint `{}`: missing integer `schema_version`",
                path.display()
            )
        })?;
    if version != u64::from(FINGERPRINT_SCHEMA_VERSION) {
        return Err(format!(
            "unsupported API fingerprint schema version {version} in `{}`; expected {}",
            path.display(),
            FINGERPRINT_SCHEMA_VERSION
        ));
    }
    let document = serde_json::from_value::<FingerprintDocument>(value)
        .map_err(|error| format!("malformed API fingerprint `{}`: {error}", path.display()))?;
    validate_canonical_document(&document).map_err(|error| {
        format!(
            "non-canonical API fingerprint `{}`: {error}",
            path.display()
        )
    })?;
    let expected = document.expected_fingerprint()?;
    if document.fingerprint != expected {
        return Err(format!(
            "corrupt API fingerprint `{}`: embedded fingerprint does not match its canonical payload",
            path.display()
        ));
    }
    Ok(document)
}

fn validate_canonical_document(document: &FingerprintDocument) -> Result<(), String> {
    if document.language_revision.is_empty() {
        return Err("language revision must not be empty".into());
    }
    if document.package.name.is_empty() || document.package.version.is_empty() {
        return Err("package name and version must not be empty".into());
    }

    let api = &document.api;
    require_sorted_unique("api.components", &api.components, |item| item.name.as_str())?;
    for component in &api.components {
        let root = format!("api.components.{}", component.name);
        require_sorted_unique(&format!("{root}.props"), &component.props, |item| {
            item.name.as_str()
        })?;
        require_sorted_unique(&format!("{root}.events"), &component.events, |item| {
            item.name.as_str()
        })?;
        require_sorted_unique(&format!("{root}.slots"), &component.slots, |item| {
            item.name.as_str()
        })?;
        for prop in &component.props {
            if prop.required != prop.default.is_none() {
                return Err(format!(
                    "{root}.props.{} has inconsistent `required` and `default` facts",
                    prop.name
                ));
            }
        }
    }

    require_sorted_unique("api.recipes", &api.recipes, |item| item.name.as_str())?;
    if let Some(theme) = &api.theme {
        require_sorted_unique("api.theme.tokens", &theme.tokens, String::as_str)?;
    }
    require_sorted_unique("api.extern_types", &api.extern_types, |item| {
        item.name.as_str()
    })?;
    for item in &api.extern_types {
        require_sorted_unique(
            &format!("api.extern_types.{}.fields", item.name),
            &item.fields,
            |field| field.name.as_str(),
        )?;
    }
    require_sorted_unique("api.enums", &api.enums, |item| item.name.as_str())?;
    for item in &api.enums {
        require_sorted_unique(
            &format!("api.enums.{}.variants", item.name),
            &item.variants,
            |variant| variant.name.as_str(),
        )?;
    }
    require_sorted_unique("api.extern_functions", &api.extern_functions, |function| {
        function.name.as_str()
    })?;
    for function in &api.extern_functions {
        let mut params = BTreeSet::new();
        if function
            .params
            .iter()
            .any(|param| !params.insert(param.name.as_str()))
        {
            return Err(format!(
                "api.extern_functions.{}.params contains a duplicate name",
                function.name
            ));
        }
    }
    Ok(())
}

fn require_sorted_unique<T>(
    path: &str,
    values: &[T],
    name: impl Fn(&T) -> &str,
) -> Result<(), String> {
    if values
        .windows(2)
        .all(|pair| name(&pair[0]) < name(&pair[1]))
    {
        Ok(())
    } else {
        Err(format!("{path} must be strictly sorted and unique"))
    }
}

fn package_for_source(source: &Path) -> Result<ApiPackage, String> {
    let directory = source.parent().ok_or_else(|| {
        format!(
            "cannot resolve containing directory for API source `{}`",
            source.display()
        )
    })?;
    let directory = directory.canonicalize().map_err(|error| {
        format!(
            "cannot resolve API source directory `{}`: {error}",
            directory.display()
        )
    })?;
    for directory in directory.ancestors() {
        let manifest = directory.join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let source = fs::read_to_string(&manifest)
            .map_err(|error| format!("cannot read `{}`: {error}", manifest.display()))?;
        let document = source.parse::<toml_edit::DocumentMut>().map_err(|error| {
            format!(
                "cannot parse package manifest `{}`: {error}",
                manifest.display()
            )
        })?;
        let Some(package) = document
            .get("package")
            .and_then(toml_edit::Item::as_table_like)
        else {
            continue;
        };
        let name = package
            .get("name")
            .and_then(toml_edit::Item::as_str)
            .ok_or_else(|| {
                format!(
                    "package manifest `{}` has no package name",
                    manifest.display()
                )
            })?
            .to_owned();
        let version = if let Some(version) =
            package.get("version").and_then(toml_edit::Item::as_str)
        {
            version.to_owned()
        } else if package
            .get("version")
            .and_then(toml_edit::Item::as_table_like)
            .and_then(|version| version.get("workspace"))
            .and_then(toml_edit::Item::as_bool)
            == Some(true)
        {
            workspace_package_version(directory).ok_or_else(|| {
                format!(
                    "package `{name}` inherits its version, but no ancestor `[workspace.package]` version was found"
                )
            })?
        } else {
            return Err(format!(
                "package manifest `{}` has no package version",
                manifest.display()
            ));
        };
        return Ok(ApiPackage { name, version });
    }
    Err(format!(
        "API source `{}` is not inside a Cargo package",
        source.display()
    ))
}

fn workspace_package_version(start: &Path) -> Option<String> {
    for directory in start.ancestors() {
        let Ok(source) = fs::read_to_string(directory.join("Cargo.toml")) else {
            continue;
        };
        let Ok(document) = source.parse::<toml_edit::DocumentMut>() else {
            continue;
        };
        if let Some(version) = document
            .get("workspace")
            .and_then(toml_edit::Item::as_table_like)
            .and_then(|workspace| workspace.get("package"))
            .and_then(toml_edit::Item::as_table_like)
            .and_then(|package| package.get("version"))
            .and_then(toml_edit::Item::as_str)
        {
            return Some(version.to_owned());
        }
    }
    None
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct DiffReport {
    schema_version: u32,
    baseline_fingerprint: String,
    current_fingerprint: String,
    summary: DiffSummary,
    changes: Vec<ApiChange>,
}

impl DiffReport {
    fn render_human(&self) -> String {
        let mut output = format!(
            "Ice API diff: {} breaking, {} behavioral review, {} additive\n",
            self.summary.breaking, self.summary.behavioral_review, self.summary.additive
        );
        for change in &self.changes {
            output.push_str(&format!(
                "[{}] {}: {} ({})\n",
                change.classification.human_name(),
                change.path,
                change.message,
                change.code
            ));
        }
        output
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct DiffSummary {
    breaking: usize,
    behavioral_review: usize,
    additive: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ApiChange {
    classification: ChangeClassification,
    code: &'static str,
    path: String,
    message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum ChangeClassification {
    Breaking,
    BehavioralReview,
    Additive,
}

impl ChangeClassification {
    fn human_name(self) -> &'static str {
        match self {
            Self::Breaking => "BREAKING",
            Self::BehavioralReview => "BEHAVIORAL REVIEW",
            Self::Additive => "ADDITIVE",
        }
    }
}

fn diff(baseline: &FingerprintDocument, current: &FingerprintDocument) -> DiffReport {
    let mut changes = Vec::new();
    if baseline.package.name != current.package.name {
        push_change(
            &mut changes,
            ChangeClassification::Breaking,
            "package_changed",
            "package.name",
            format!(
                "package changed from `{}` to `{}`",
                baseline.package.name, current.package.name
            ),
        );
    }
    if baseline.language_revision != current.language_revision {
        push_change(
            &mut changes,
            ChangeClassification::Breaking,
            "language_revision_changed",
            "language_revision",
            format!(
                "language revision changed from `{}` to `{}`",
                baseline.language_revision, current.language_revision
            ),
        );
    }

    diff_components(
        &baseline.api.components,
        &current.api.components,
        &mut changes,
    );
    diff_recipes(&baseline.api.recipes, &current.api.recipes, &mut changes);
    diff_theme(
        baseline.api.theme.as_ref(),
        current.api.theme.as_ref(),
        &mut changes,
    );
    diff_named_contracts(
        "extern_type",
        &baseline.api.extern_types,
        &current.api.extern_types,
        |item| &item.name,
        &mut changes,
    );
    diff_named_contracts(
        "enum",
        &baseline.api.enums,
        &current.api.enums,
        |item| &item.name,
        &mut changes,
    );
    diff_named_contracts(
        "extern_function",
        &baseline.api.extern_functions,
        &current.api.extern_functions,
        |item| &item.name,
        &mut changes,
    );

    changes.sort_by(|left, right| {
        left.classification
            .cmp(&right.classification)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.code.cmp(right.code))
    });
    let summary = DiffSummary {
        breaking: changes
            .iter()
            .filter(|change| change.classification == ChangeClassification::Breaking)
            .count(),
        behavioral_review: changes
            .iter()
            .filter(|change| change.classification == ChangeClassification::BehavioralReview)
            .count(),
        additive: changes
            .iter()
            .filter(|change| change.classification == ChangeClassification::Additive)
            .count(),
    };
    DiffReport {
        schema_version: DIFF_SCHEMA_VERSION,
        baseline_fingerprint: baseline.fingerprint.clone(),
        current_fingerprint: current.fingerprint.clone(),
        summary,
        changes,
    }
}

fn diff_components(
    baseline: &[ApiComponent],
    current: &[ApiComponent],
    changes: &mut Vec<ApiChange>,
) {
    let baseline = named(baseline, |item| &item.name);
    let current = named(current, |item| &item.name);
    for name in names(&baseline, &current) {
        let path = format!("components.{name}");
        match (baseline.get(name), current.get(name)) {
            (Some(_), None) => push_change(
                changes,
                ChangeClassification::Breaking,
                "component_removed",
                path,
                "component was removed",
            ),
            (None, Some(_)) => push_change(
                changes,
                ChangeClassification::Additive,
                "component_added",
                path,
                "component was added",
            ),
            (Some(before), Some(after)) => diff_component(before, after, changes),
            (None, None) => unreachable!(),
        }
    }
}

fn diff_component(before: &ApiComponent, after: &ApiComponent, changes: &mut Vec<ApiChange>) {
    let root = format!("components.{}", before.name);
    if before.default_output != after.default_output {
        push_change(
            changes,
            ChangeClassification::Breaking,
            "component_output_changed",
            format!("{root}.default_output"),
            format!(
                "default output changed from `{}` to `{}`",
                before.default_output, after.default_output
            ),
        );
    }
    if before.lifetime != after.lifetime {
        push_change(
            changes,
            ChangeClassification::BehavioralReview,
            "component_lifetime_changed",
            format!("{root}.lifetime"),
            "component state lifetime changed",
        );
    }
    diff_props(&root, &before.props, &after.props, changes);
    diff_events(&root, &before.events, &after.events, changes);
    diff_slots(&root, &before.slots, &after.slots, changes);
}

fn diff_props(
    root: &str,
    before: &[ApiComponentProp],
    after: &[ApiComponentProp],
    changes: &mut Vec<ApiChange>,
) {
    let before = named(before, |item| &item.name);
    let after = named(after, |item| &item.name);
    for name in names(&before, &after) {
        let path = format!("{root}.props.{name}");
        match (before.get(name), after.get(name)) {
            (Some(_), None) => push_change(
                changes,
                ChangeClassification::Breaking,
                "prop_removed",
                path,
                "component prop was removed",
            ),
            (None, Some(prop)) => {
                let (classification, code, message) = if prop.required {
                    (
                        ChangeClassification::Breaking,
                        "required_prop_added",
                        "required component prop was added",
                    )
                } else {
                    (
                        ChangeClassification::Additive,
                        "default_prop_added",
                        "defaulted component prop was added",
                    )
                };
                push_change(changes, classification, code, path, message);
            }
            (Some(left), Some(right)) => {
                if left.ty != right.ty {
                    push_change(
                        changes,
                        ChangeClassification::Breaking,
                        "prop_type_changed",
                        format!("{path}.type"),
                        format!("prop type changed from `{}` to `{}`", left.ty, right.ty),
                    );
                }
                if left.access != right.access {
                    let direction = match (left.access, right.access) {
                        (ApiPropAccess::Read, ApiPropAccess::Bind) => "read to bind",
                        (ApiPropAccess::Bind, ApiPropAccess::Read) => "bind to read",
                        _ => unreachable!(),
                    };
                    push_change(
                        changes,
                        ChangeClassification::Breaking,
                        "prop_access_changed",
                        format!("{path}.access"),
                        format!("prop access changed from {direction}"),
                    );
                }
                match (left.required, right.required) {
                    (false, true) => push_change(
                        changes,
                        ChangeClassification::Breaking,
                        "prop_became_required",
                        format!("{path}.required"),
                        "defaulted prop became required",
                    ),
                    (true, false) => push_change(
                        changes,
                        ChangeClassification::Additive,
                        "prop_default_added",
                        format!("{path}.required"),
                        "required prop gained a default",
                    ),
                    _ => {}
                }
                if !left.required && !right.required && left.default != right.default {
                    push_change(
                        changes,
                        ChangeClassification::BehavioralReview,
                        "prop_default_changed",
                        format!("{path}.default"),
                        "prop default value changed",
                    );
                }
            }
            (None, None) => unreachable!(),
        }
    }
}

fn diff_events(
    root: &str,
    before: &[ApiComponentEvent],
    after: &[ApiComponentEvent],
    changes: &mut Vec<ApiChange>,
) {
    let before = named(before, |item| &item.name);
    let after = named(after, |item| &item.name);
    for name in names(&before, &after) {
        let path = format!("{root}.events.{name}");
        match (before.get(name), after.get(name)) {
            (Some(_), None) => push_change(
                changes,
                ChangeClassification::Breaking,
                "event_removed",
                path,
                "named event was removed",
            ),
            (None, Some(_)) => push_change(
                changes,
                ChangeClassification::Breaking,
                "event_added",
                path,
                "named event was added to a closed component contract",
            ),
            (Some(left), Some(right)) if left.payload != right.payload => push_change(
                changes,
                ChangeClassification::Breaking,
                "event_payload_changed",
                format!("{path}.payload"),
                "named event payload signature changed",
            ),
            _ => {}
        }
    }
}

fn diff_slots(
    root: &str,
    before: &[ApiComponentSlot],
    after: &[ApiComponentSlot],
    changes: &mut Vec<ApiChange>,
) {
    let before = named(before, |item| &item.name);
    let after = named(after, |item| &item.name);
    for name in names(&before, &after) {
        let path = format!("{root}.slots.{name}");
        match (before.get(name), after.get(name)) {
            (Some(_), None) => push_change(
                changes,
                ChangeClassification::Breaking,
                "slot_removed",
                path,
                "component slot was removed",
            ),
            (None, Some(slot)) => {
                let (classification, code, message) = if slot.required {
                    (
                        ChangeClassification::Breaking,
                        "required_slot_added",
                        "required component slot was added",
                    )
                } else {
                    (
                        ChangeClassification::Additive,
                        "optional_slot_added",
                        "optional component slot was added",
                    )
                };
                push_change(changes, classification, code, path, message);
            }
            (Some(left), Some(right)) if !left.required && right.required => push_change(
                changes,
                ChangeClassification::Breaking,
                "slot_became_required",
                format!("{path}.required"),
                "optional slot became required",
            ),
            (Some(left), Some(right)) if left.required && !right.required => push_change(
                changes,
                ChangeClassification::Additive,
                "slot_became_optional",
                format!("{path}.required"),
                "required slot became optional",
            ),
            _ => {}
        }
    }
}

fn diff_recipes(before: &[ApiRecipe], after: &[ApiRecipe], changes: &mut Vec<ApiChange>) {
    let before = named(before, |item| &item.name);
    let after = named(after, |item| &item.name);
    for name in names(&before, &after) {
        let path = format!("recipes.{name}");
        match (before.get(name), after.get(name)) {
            (Some(_), None) => push_change(
                changes,
                ChangeClassification::Breaking,
                "recipe_removed",
                path,
                "recipe was removed",
            ),
            (None, Some(_)) => push_change(
                changes,
                ChangeClassification::Additive,
                "recipe_added",
                path,
                "recipe was added",
            ),
            (Some(left), Some(right)) => {
                if left.target != right.target {
                    push_change(
                        changes,
                        ChangeClassification::Breaking,
                        "recipe_target_changed",
                        format!("{path}.target"),
                        "recipe target changed",
                    );
                }
                if left.base != right.base || left.flattened_utilities != right.flattened_utilities
                {
                    push_change(
                        changes,
                        ChangeClassification::BehavioralReview,
                        "recipe_semantics_changed",
                        format!("{path}.flattened_semantic_contract"),
                        "recipe inheritance or flattened utility semantics changed",
                    );
                }
            }
            (None, None) => unreachable!(),
        }
    }
}

fn diff_theme(
    before: Option<&ApiThemeContract>,
    after: Option<&ApiThemeContract>,
    changes: &mut Vec<ApiChange>,
) {
    match (before, after) {
        (None, None) => {}
        (Some(_), None) => push_change(
            changes,
            ChangeClassification::Breaking,
            "theme_contract_removed",
            "theme",
            "theme contract was removed",
        ),
        (None, Some(_)) => push_change(
            changes,
            ChangeClassification::Breaking,
            "theme_contract_added",
            "theme",
            "theme contract was added and requires complete consumer palettes",
        ),
        (Some(left), Some(right)) => {
            if left.name != right.name {
                push_change(
                    changes,
                    ChangeClassification::Breaking,
                    "theme_contract_renamed",
                    "theme.name",
                    format!(
                        "theme contract changed from `{}` to `{}`",
                        left.name, right.name
                    ),
                );
            }
            let left = left
                .tokens
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            let right = right
                .tokens
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            for token in left.difference(&right) {
                push_change(
                    changes,
                    ChangeClassification::Breaking,
                    "theme_token_removed",
                    format!("theme.tokens.{token}"),
                    "theme token was removed",
                );
            }
            for token in right.difference(&left) {
                push_change(
                    changes,
                    ChangeClassification::Breaking,
                    "theme_token_added",
                    format!("theme.tokens.{token}"),
                    "theme token was added and existing palettes are incomplete",
                );
            }
        }
    }
}

fn diff_named_contracts<'a, T: PartialEq>(
    kind: &'static str,
    before: &'a [T],
    after: &'a [T],
    name: impl Fn(&'a T) -> &'a str + Copy,
    changes: &mut Vec<ApiChange>,
) {
    let before = named(before, name);
    let after = named(after, name);
    for item_name in names(&before, &after) {
        let path = format!("{kind}s.{item_name}");
        match (before.get(item_name), after.get(item_name)) {
            (Some(_), None) => push_change(
                changes,
                ChangeClassification::Breaking,
                match kind {
                    "extern_type" => "extern_type_removed",
                    "enum" => "enum_removed",
                    "extern_function" => "extern_function_removed",
                    _ => unreachable!(),
                },
                path,
                format!("{} was removed", kind.replace('_', " ")),
            ),
            (None, Some(_)) => push_change(
                changes,
                ChangeClassification::Additive,
                match kind {
                    "extern_type" => "extern_type_added",
                    "enum" => "enum_added",
                    "extern_function" => "extern_function_added",
                    _ => unreachable!(),
                },
                path,
                format!("{} was added", kind.replace('_', " ")),
            ),
            (Some(left), Some(right)) if left != right => push_change(
                changes,
                ChangeClassification::Breaking,
                match kind {
                    "extern_type" => "extern_type_changed",
                    "enum" => "enum_changed",
                    "extern_function" => "extern_function_changed",
                    _ => unreachable!(),
                },
                path,
                format!("{} contract changed", kind.replace('_', " ")),
            ),
            _ => {}
        }
    }
}

fn named<'a, T>(values: &'a [T], name: impl Fn(&'a T) -> &'a str) -> BTreeMap<&'a str, &'a T> {
    values.iter().map(|value| (name(value), value)).collect()
}

fn names<'a, T>(left: &BTreeMap<&'a str, T>, right: &BTreeMap<&'a str, T>) -> BTreeSet<&'a str> {
    left.keys().chain(right.keys()).copied().collect()
}

fn push_change(
    changes: &mut Vec<ApiChange>,
    classification: ChangeClassification,
    code: &'static str,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    changes.push(ApiChange {
        classification,
        code,
        path: path.into(),
        message: message.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::{
        ApiPackage, ChangeClassification, FINGERPRINT_SCHEMA_VERSION, FingerprintDocument, diff,
        read_fingerprint,
    };
    use std::fs;
    use tempfile::TempDir;
    use ui_lang_core::{
        ApiExternFunction, ApiExternKind, ApiSurface, analyze, analyze_api_file, format_source,
    };

    fn package() -> ApiPackage {
        ApiPackage {
            name: "fixture".into(),
            version: "1.0.0".into(),
        }
    }

    fn fingerprint(source: &str) -> FingerprintDocument {
        let checked = analyze(source).unwrap();
        FingerprintDocument::new(package(), ApiSurface::from_checked(&checked)).unwrap()
    }

    fn source(component: &str, recipe_utility: &str, extra_token: &str) -> String {
        format!(
            r#"
app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
  accent
  {extra_token}
palette light for AppTheme
  bg #000000
  fg #ffffff
  primary #112233
  danger #ff0000
  accent #445566
  {extra_token} #778899
recipe panel for box
  {recipe_utility}
{component}
view
  space
"#
        )
    }

    #[test]
    fn formatting_and_declaration_order_do_not_change_the_fingerprint() {
        let first = source(
            "component Zebra()\n  space\ncomponent Alpha(value:str=\"x\")\n  space",
            "p-2",
            "extra",
        );
        let reordered = source(
            "component Alpha( value : str = \"x\" )\n    space\n\ncomponent Zebra()\n    space",
            "@p-2",
            "extra",
        );
        let formatted = format_source(&reordered).unwrap();
        let first = fingerprint(&first);
        let second = fingerprint(&formatted);
        assert_eq!(first.api, second.api);
        assert_eq!(first.fingerprint, second.fingerprint);
        assert!(diff(&first, &second).changes.is_empty());
    }

    #[test]
    fn classifies_breaking_behavioral_and_additive_changes() {
        let before = source(
            "component Card(title:str=\"Draft\")\n  col\n    slot Body?",
            "bg-bg",
            "extra",
        );
        let after = source(
            "component Card(bind title:str, count:i64=0)\n  col\n    slot Body\n    slot Footer?\ncomponent NewThing()\n  space",
            "bg-primary",
            "new_token",
        );
        let report = diff(&fingerprint(&before), &fingerprint(&after));
        let code = |expected| {
            report
                .changes
                .iter()
                .find(|change| change.code == expected)
                .unwrap()
        };
        assert_eq!(
            code("prop_access_changed").classification,
            ChangeClassification::Breaking
        );
        assert_eq!(
            code("prop_became_required").classification,
            ChangeClassification::Breaking
        );
        assert_eq!(
            code("default_prop_added").classification,
            ChangeClassification::Additive
        );
        assert_eq!(
            code("slot_became_required").classification,
            ChangeClassification::Breaking
        );
        assert_eq!(
            code("optional_slot_added").classification,
            ChangeClassification::Additive
        );
        assert_eq!(
            code("component_added").classification,
            ChangeClassification::Additive
        );
        assert_eq!(
            code("theme_token_added").classification,
            ChangeClassification::Breaking
        );
        assert_eq!(
            code("theme_token_removed").classification,
            ChangeClassification::Breaking
        );
        assert_eq!(
            code("recipe_semantics_changed").classification,
            ChangeClassification::BehavioralReview
        );
    }

    #[test]
    fn required_props_slots_component_removal_and_event_payloads_are_breaking() {
        let before = source(
            "component Removed()\n  space\ncomponent Card(title:str)\n  emits\n    changed(str)\n  col\n    slot Body?",
            "p-2",
            "extra",
        );
        let after = source(
            "component Card(title:str, count:i64)\n  emits\n    changed(i64)\n  col\n    slot Body?\n    slot Actions",
            "p-2",
            "extra",
        );
        let report = diff(&fingerprint(&before), &fingerprint(&after));
        for expected in [
            "component_removed",
            "required_prop_added",
            "event_payload_changed",
            "required_slot_added",
        ] {
            let change = report
                .changes
                .iter()
                .find(|change| change.code == expected)
                .unwrap_or_else(|| panic!("missing {expected}: {:?}", report.changes));
            assert_eq!(change.classification, ChangeClassification::Breaking);
        }
    }

    #[test]
    fn interface_graph_keeps_bare_and_nested_namespace_identities() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("plain.ice"),
            "component Plain()\n  space\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("inner.ice"),
            "component Inner()\n  space\nrecipe compact for text\n  text-sm\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("nested.ice"),
            "use \"inner.ice\" as deep\ncomponent Outer()\n  space\n",
        )
        .unwrap();
        let root = temp.path().join("api.ice");
        fs::write(
            &root,
            "use \"plain.ice\"\nuse \"plain.ice\" as mirror\nuse \"nested.ice\" as kit\ntheme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette light for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #112233\n  danger #ff0000\n",
        )
        .unwrap();

        let api = analyze_api_file(&root).unwrap();
        assert_eq!(
            api.components
                .iter()
                .map(|component| component.name.as_str())
                .collect::<Vec<_>>(),
            ["Plain", "kit::Outer", "kit::deep::Inner", "mirror::Plain"]
        );
        assert_eq!(api.recipes[0].name, "kit::deep::compact");
    }

    #[test]
    fn interface_analysis_does_not_hide_an_incomplete_application_root() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("api.ice");
        fs::write(&root, "app Broken\n").unwrap();
        let error = analyze_api_file(&root).unwrap_err();
        assert_eq!(error.code, "E008");
    }

    #[test]
    fn package_identity_resolves_workspace_inherited_versions() {
        let temp = TempDir::new().unwrap();
        let member = temp.path().join("member");
        fs::create_dir_all(member.join("src")).unwrap();
        fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"member\"]\n[workspace.package]\nversion = \"9.8.7\"\n",
        )
        .unwrap();
        fs::write(
            member.join("Cargo.toml"),
            "[package]\nname = \"inherited\"\nversion.workspace = true\n",
        )
        .unwrap();
        let source = member.join("src/api.ice");
        fs::write(&source, "").unwrap();

        let package = super::package_for_source(&source).unwrap();
        assert_eq!(package.name, "inherited");
        assert_eq!(package.version, "9.8.7");
    }

    #[test]
    fn rejects_unknown_schema_and_corrupt_payloads() {
        let temp = TempDir::new().unwrap();
        let source = source(
            "component Card(title:str=\"Draft\")\n  space",
            "p-2",
            "extra",
        );
        let document = fingerprint(&source);
        let path = temp.path().join("api.json");

        let mut value = serde_json::to_value(&document).unwrap();
        value["schema_version"] = serde_json::json!(FINGERPRINT_SCHEMA_VERSION + 1);
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(read_fingerprint(&path).unwrap_err().contains("unsupported"));

        let mut value = serde_json::to_value(&document).unwrap();
        value["api"]["components"][0]["name"] = serde_json::json!("Changed");
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(read_fingerprint(&path).unwrap_err().contains("corrupt"));

        let mut value = serde_json::to_value(&document).unwrap();
        value["api"]["components"][0]["props"][0]["default"]["unexpected"] =
            serde_json::json!("tampered");
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(read_fingerprint(&path).unwrap_err().contains("malformed"));

        let mut duplicate = document.clone();
        duplicate
            .api
            .components
            .push(duplicate.api.components[0].clone());
        duplicate.fingerprint = duplicate.expected_fingerprint().unwrap();
        fs::write(&path, serde_json::to_vec(&duplicate).unwrap()).unwrap();
        assert!(
            read_fingerprint(&path)
                .unwrap_err()
                .contains("non-canonical")
        );

        let mut inconsistent = document.clone();
        inconsistent.api.components[0].props[0].required = true;
        inconsistent.fingerprint = inconsistent.expected_fingerprint().unwrap();
        fs::write(&path, serde_json::to_vec(&inconsistent).unwrap()).unwrap();
        assert!(
            read_fingerprint(&path)
                .unwrap_err()
                .contains("inconsistent")
        );

        let mut duplicate_extern_name = document.clone();
        duplicate_extern_name.api.extern_functions = vec![
            ApiExternFunction {
                name: "load".into(),
                kind: ApiExternKind::Future,
                rust_path: "crate::backend::load".into(),
                params: Vec::new(),
                progress: None,
                output: "str".into(),
                error: Some("str".into()),
            },
            ApiExternFunction {
                name: "load".into(),
                kind: ApiExternKind::Task,
                rust_path: "crate::backend::load_task".into(),
                params: Vec::new(),
                progress: None,
                output: "str".into(),
                error: None,
            },
        ];
        duplicate_extern_name.fingerprint = duplicate_extern_name.expected_fingerprint().unwrap();
        fs::write(&path, serde_json::to_vec(&duplicate_extern_name).unwrap()).unwrap();
        assert!(
            read_fingerprint(&path)
                .unwrap_err()
                .contains("api.extern_functions must be strictly sorted and unique")
        );

        fs::write(&path, b"{ definitely not json").unwrap();
        assert!(read_fingerprint(&path).unwrap_err().contains("malformed"));
    }

    #[cfg(unix)]
    #[test]
    fn package_identity_uses_the_symlink_location_not_its_external_target() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let package = temp.path().join("package");
        let external = temp.path().join("external");
        fs::create_dir_all(package.join("src")).unwrap();
        fs::create_dir_all(&external).unwrap();
        fs::write(
            package.join("Cargo.toml"),
            "[package]\nname = \"linked-api\"\nversion = \"1.2.3\"\n",
        )
        .unwrap();
        let target = external.join("api.ice");
        fs::write(&target, "component External()\n  space\n").unwrap();
        let source = package.join("src/api.ice");
        symlink(&target, &source).unwrap();

        let identity = super::package_for_source(&source).unwrap();
        assert_eq!(identity.name, "linked-api");
        assert_eq!(identity.version, "1.2.3");
    }
}
