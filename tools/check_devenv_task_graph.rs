#![doc = "Typed validator for the resolved Devenv acceptance task graph."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

#[path = "digest_binding.rs"]
mod content_digest;
#[path = "execution_evidence.rs"]
mod execution_evidence;

use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const POLICY_PATH: &str = "nix/devenv/workflow-policy.toml";
const VALID_TASKS_PATH: &str = "fixtures/devenv/v1/valid-tasks.json";
const TASK_CASES_PATH: &str = "fixtures/devenv/v1/task-cases.tsv";
const CONFIG_CASES_PATH: &str = "fixtures/devenv/v1/config-cases.tsv";
const RUNTIME_CASES_PATH: &str = "fixtures/devenv/v1/runtime-package-cases.tsv";
const EXECUTION_EVIDENCE_PATH: &str = "assurance/v1/devenv-fixture-executions.tsv";
const EXECUTION_TOOL_PATHS: [&str; 3] = [
    "tools/check_devenv_task_graph.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
];
const EXECUTION_CONFIGURATION_PATHS: [&str; 2] = [POLICY_PATH, VALID_TASKS_PATH];
const POLICY_SCHEMA: &str = "buoy-devenv-workflow-policy/v1";
const TASK_CASE_SCHEMA: &str = "buoy-devenv-task-case/v1";
const CONFIG_CASE_SCHEMA: &str = "buoy-devenv-config-case/v1";
const RUNTIME_CASE_SCHEMA: &str = "buoy-devenv-runtime-case/v1";

#[derive(Clone, Debug, Deserialize)]
struct Task {
    name: String,
    command: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    after: Vec<String>,
    #[serde(default)]
    before: Vec<String>,
    status: Option<Value>,
    #[serde(default)]
    exec_if_modified: Vec<String>,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema: String,
    root_token: String,
    sink: String,
    lifecycle: String,
    post_phase: Vec<String>,
    manual: Vec<String>,
    baseline: Vec<String>,
    configuration: Configuration,
    phase: Vec<Phase>,
    script_owner: Vec<ScriptOwner>,
    runtime_package: Vec<RuntimePackage>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    require_version: bool,
    rust_module: bool,
    rust_toolchain_file: String,
    rust_channel: String,
    rust_profile: String,
    rust_components: Vec<String>,
    rust_targets: Vec<String>,
    module_files: Vec<String>,
    hooks: Vec<String>,
    common_packages: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Phase {
    name: String,
    children: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScriptOwner {
    module: String,
    scripts: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimePackage {
    script: String,
    packages: Vec<String>,
}

#[derive(Clone, Debug)]
struct Case {
    id: String,
    row_index: usize,
    source_row: String,
    polarity: String,
    mutation: String,
    expected: String,
}

#[derive(Clone, Debug)]
struct Graph {
    tasks: BTreeMap<String, Task>,
    prerequisites: BTreeMap<String, BTreeSet<String>>,
}

fn read(path: &Path, code: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{code}:{error}"))
}

fn parse_policy(text: &str) -> Result<Policy, String> {
    let policy =
        toml::from_str::<Policy>(text).map_err(|error| format!("DEVENV-POLICY-TOML:{error}"))?;
    if policy.schema != POLICY_SCHEMA {
        return Err(format!("DEVENV-POLICY-SCHEMA:{}", policy.schema));
    }
    Ok(policy)
}

fn parse_tasks(text: &str) -> Result<Vec<Task>, String> {
    serde_json::from_str(text).map_err(|error| format!("DEVENV-TASK-JSON:{error}"))
}

fn dependency_name(value: &str) -> &str {
    value.split_once('@').map_or(value, |(name, _)| name)
}

fn required_task_names(policy: &Policy) -> BTreeSet<&str> {
    policy
        .baseline
        .iter()
        .chain(
            policy
                .phase
                .iter()
                .flat_map(|phase| std::iter::once(&phase.name).chain(phase.children.iter())),
        )
        .chain(policy.post_phase.iter())
        .chain(policy.manual.iter())
        .chain([&policy.sink, &policy.lifecycle])
        .map(String::as_str)
        .collect()
}

fn build_graph(tasks: Vec<Task>, policy: &Policy) -> Result<Graph, String> {
    let mut by_name = BTreeMap::new();
    for task in tasks {
        let name = task.name.clone();
        if by_name.insert(name.clone(), task).is_some() {
            return Err(format!("DEVENV-TASK-DUPLICATE:{name}"));
        }
    }
    for required in required_task_names(policy) {
        if !by_name.contains_key(required) {
            if policy.phase.iter().any(|phase| phase.name == required) {
                return Err(format!("DEVENV-PHASE-MISSING:{required}"));
            }
            if policy.manual.iter().any(|task| task == required) {
                return Err(format!("DEVENV-MANUAL-TASK-MISSING:{required}"));
            }
            return Err(format!("DEVENV-TASK-MISSING:{required}"));
        }
    }

    let mut prerequisites = by_name
        .keys()
        .map(|name| (name.clone(), BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for task in by_name.values() {
        for dependency in &task.after {
            let dependency = dependency_name(dependency);
            if !by_name.contains_key(dependency) {
                return Err(format!(
                    "DEVENV-TASK-UNKNOWN-DEPENDENCY:{}:{dependency}",
                    task.name
                ));
            }
            prerequisites
                .get_mut(&task.name)
                .expect("task prerequisite entry exists")
                .insert(dependency.to_owned());
        }
        for dependent in &task.before {
            let dependent = dependency_name(dependent);
            if !by_name.contains_key(dependent) {
                return Err(format!(
                    "DEVENV-TASK-UNKNOWN-DEPENDENCY:{}:{dependent}",
                    task.name
                ));
            }
            prerequisites
                .get_mut(dependent)
                .expect("dependent prerequisite entry exists")
                .insert(task.name.clone());
        }
    }
    let mut dependents = by_name
        .keys()
        .map(|name| (name.clone(), BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for (dependent, dependencies) in &prerequisites {
        for dependency in dependencies {
            dependents
                .get_mut(dependency)
                .expect("dependency entry exists")
                .insert(dependent.clone());
        }
    }
    reject_cycles(&prerequisites, &dependents)?;
    Ok(Graph {
        tasks: by_name,
        prerequisites,
    })
}

fn reject_cycles(
    prerequisites: &BTreeMap<String, BTreeSet<String>>,
    dependents: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    let mut remaining = prerequisites
        .iter()
        .map(|(name, dependencies)| (name.clone(), dependencies.len()))
        .collect::<BTreeMap<_, _>>();
    let mut ready = remaining
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    let mut visited = 0;
    while let Some(name) = ready.pop_first() {
        visited += 1;
        for dependent in dependents
            .get(&name)
            .expect("cycle checker dependent entry exists")
        {
            let count = remaining
                .get_mut(dependent)
                .expect("cycle checker count exists");
            *count -= 1;
            if *count == 0 {
                ready.insert(dependent.clone());
            }
        }
    }
    if visited == remaining.len() {
        Ok(())
    } else {
        let first = remaining
            .iter()
            .find(|(_, count)| **count > 0)
            .map_or("unknown", |(name, _)| name.as_str());
        Err(format!("DEVENV-TASK-CYCLE:{first}"))
    }
}

fn direct_prerequisites<'a>(graph: &'a Graph, task: &str) -> &'a BTreeSet<String> {
    graph
        .prerequisites
        .get(task)
        .expect("validated task has prerequisite entry")
}

fn require_edge(graph: &Graph, dependent: &str, prerequisite: &str) -> Result<(), String> {
    if direct_prerequisites(graph, dependent).contains(prerequisite) {
        Ok(())
    } else {
        Err(format!("DEVENV-ORDER:{prerequisite}->{dependent}"))
    }
}

fn transitively_depends_on(graph: &Graph, task: &str, prerequisite: &str) -> bool {
    let mut pending = vec![task];
    let mut visited = BTreeSet::new();
    while let Some(current) = pending.pop() {
        for dependency in direct_prerequisites(graph, current) {
            if dependency == prerequisite {
                return true;
            }
            if visited.insert(dependency.as_str()) {
                pending.push(dependency);
            }
        }
    }
    false
}

fn validate_graph(tasks: Vec<Task>, policy: &Policy, root: &str) -> Result<usize, String> {
    let mut manual = BTreeSet::new();
    for task in &policy.manual {
        if !manual.insert(task) {
            return Err(format!("DEVENV-POLICY-MANUAL-DUPLICATE:{task}"));
        }
    }
    let graph = build_graph(tasks, policy)?;
    for task in &policy.manual {
        if transitively_depends_on(&graph, &policy.lifecycle, task) {
            return Err(format!("DEVENV-MANUAL-TASK-AUTOMATIC:{task}"));
        }
        if let Some(name) = task.strip_prefix("buoy:") {
            let alias = format!("spec-provenance:{name}");
            if graph.tasks.contains_key(&alias)
                && transitively_depends_on(&graph, &policy.lifecycle, &alias)
            {
                return Err(format!("DEVENV-MANUAL-TASK-AUTOMATIC:{alias}"));
            }
        }
    }
    let phases = policy
        .phase
        .iter()
        .map(|phase| phase.name.as_str())
        .collect::<BTreeSet<_>>();
    for phase in &policy.phase {
        let dependencies = direct_prerequisites(&graph, &phase.name);
        if let Some(nested) = dependencies
            .iter()
            .find(|name| phases.contains(name.as_str()))
        {
            return Err(format!("DEVENV-PHASE-NESTED:{}:{nested}", phase.name));
        }
    }

    let mut owners = BTreeMap::<&str, Vec<&str>>::new();
    for phase in &policy.phase {
        let dependencies = direct_prerequisites(&graph, &phase.name);
        for child in &phase.children {
            if dependencies.contains(child) {
                owners.entry(child).or_default().push(phase.name.as_str());
            }
        }
    }
    for child in policy.phase.iter().flat_map(|phase| &phase.children) {
        let child_owners = owners.get(child.as_str()).cloned().unwrap_or_default();
        if child_owners.len() != 1 {
            return Err(format!(
                "DEVENV-CHILD-OWNER:{child}:count={}",
                child_owners.len()
            ));
        }
    }
    for (left_index, left) in policy.phase.iter().enumerate() {
        for right in policy.phase.iter().skip(left_index + 1) {
            for child in &left.children {
                if direct_prerequisites(&graph, &right.name).contains(child) {
                    return Err(format!(
                        "DEVENV-CHILD-OWNER:{child}:owners={},{}",
                        left.name, right.name
                    ));
                }
            }
        }
    }

    for pair in policy.baseline.windows(2) {
        require_edge(&graph, &pair[1], &pair[0])?;
    }
    let mut prior = policy
        .baseline
        .last()
        .ok_or_else(|| "DEVENV-POLICY-BASELINE-EMPTY".to_owned())?;
    for phase in &policy.phase {
        let first = phase
            .children
            .first()
            .ok_or_else(|| format!("DEVENV-POLICY-PHASE-EMPTY:{}", phase.name))?;
        require_edge(&graph, first, prior)?;
        for pair in phase.children.windows(2) {
            require_edge(&graph, &pair[1], &pair[0])?;
        }
        let expected = phase.children.iter().cloned().collect::<BTreeSet<_>>();
        if direct_prerequisites(&graph, &phase.name) != &expected {
            return Err(format!("DEVENV-PHASE-DEPENDENCIES:{}", phase.name));
        }
        prior = &phase.name;
    }
    for task in &policy.post_phase {
        require_edge(&graph, task, prior)?;
        prior = task;
    }
    let sink_dependencies = direct_prerequisites(&graph, &policy.sink);
    for phase in &policy.phase {
        for child in &phase.children {
            if sink_dependencies.contains(child) {
                return Err(format!("DEVENV-SINK-DIRECT-CHILD:{child}"));
            }
        }
    }
    if sink_dependencies != &BTreeSet::from([prior.clone()]) {
        return Err(format!("DEVENV-SINK-DEPENDENCIES:{}", policy.sink));
    }
    if !direct_prerequisites(&graph, &policy.lifecycle).contains(&policy.sink) {
        return Err(format!("DEVENV-LIFECYCLE:{}", policy.lifecycle));
    }

    let acceptance = required_task_names(policy);
    for name in &acceptance {
        let task = graph.tasks.get(*name).expect("required task exists");
        if task.kind != "oneshot" {
            return Err(format!("DEVENV-TASK-TYPE:{name}:{}", task.kind));
        }
        if *name != policy.lifecycle && task.command.is_none() {
            return Err(format!("DEVENV-TASK-COMMAND:{name}"));
        }
        if task.status.is_some() || !task.exec_if_modified.is_empty() {
            return Err(format!("DEVENV-TASK-CACHE:{name}"));
        }
    }
    for task in graph
        .tasks
        .values()
        .filter(|task| task.name.starts_with("buoy:") || task.name.starts_with("spec-provenance:"))
    {
        if task.cwd.as_deref() != Some(root) {
            return Err(format!("DEVENV-TASK-ROOT:{}", task.name));
        }
    }
    Ok(acceptance.len())
}

fn validate_configuration(policy: &Policy, root: Option<&Path>) -> Result<(), String> {
    let configuration = &policy.configuration;
    if !configuration.require_version {
        return Err("DEVENV-CONFIG-VERSION".to_owned());
    }
    let expected_components = [
        "clippy",
        "llvm-tools-preview",
        "miri",
        "rust-analyzer",
        "rust-src",
        "rustc-dev",
        "rustfmt",
    ];
    if !configuration.rust_module
        || configuration.rust_toolchain_file != "rust-toolchain.toml"
        || configuration.rust_channel != "nightly-2026-04-16"
        || configuration.rust_profile != "minimal"
        || configuration
            .rust_components
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            != expected_components
        || configuration.rust_targets != ["thumbv7em-none-eabi"]
    {
        return Err("DEVENV-CONFIG-RUST".to_owned());
    }
    let required_modules = [
        "nix/devenv/environment.nix",
        "nix/devenv/hooks.nix",
        "nix/devenv/tasks.nix",
        "nix/devenv/scripts/core.nix",
        "nix/devenv/scripts/assurance.nix",
        "nix/devenv/scripts/adversarial.nix",
        "nix/devenv/scripts/acceptance.nix",
    ];
    if required_modules.iter().any(|required| {
        !configuration
            .module_files
            .iter()
            .any(|module| module == required)
    }) {
        return Err("DEVENV-CONFIG-MODULE".to_owned());
    }
    let expected_hooks = BTreeSet::from(["buoy-nixfmt", "buoy-openspec", "buoy-rustfmt"]);
    let actual_hooks = configuration
        .hooks
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if actual_hooks != expected_hooks {
        let hook = actual_hooks
            .symmetric_difference(&expected_hooks)
            .next()
            .copied()
            .unwrap_or("unknown");
        return Err(format!("DEVENV-CONFIG-HOOK:{hook}"));
    }
    let mut scripts = BTreeSet::new();
    for owner in &policy.script_owner {
        if owner.module.trim().is_empty() || owner.scripts.is_empty() {
            return Err("DEVENV-CONFIG-SCRIPT-OWNER".to_owned());
        }
        for script in &owner.scripts {
            if !scripts.insert(script) {
                return Err(format!("DEVENV-CONFIG-SCRIPT-OWNER:{script}"));
            }
        }
    }
    validate_runtime_packages(policy)?;
    if let Some(root) = root {
        validate_live_configuration(policy, root)?;
    }
    Ok(())
}

fn runtime_map(policy: &Policy) -> BTreeMap<&str, BTreeSet<&str>> {
    policy
        .runtime_package
        .iter()
        .map(|runtime| {
            (
                runtime.script.as_str(),
                runtime.packages.iter().map(String::as_str).collect(),
            )
        })
        .collect()
}

fn validate_runtime_packages(policy: &Policy) -> Result<(), String> {
    let runtime = runtime_map(policy);
    let required = [
        ("buoy-fcis", "dylint"),
        ("buoy-formal-spikes", "aeneas"),
        ("buoy-formal-spikes", "lean-aeneas"),
        ("buoy-formal-spikes", "verus"),
        ("buoy-formal-spikes", "z3-standalone"),
        ("buoy-core-proofs", "verus"),
        ("buoy-coverage", "cargo-llvm-cov"),
        ("buoy-coverage", "dylint"),
        ("buoy-fuzz-replay", "cargo-fuzz"),
        ("buoy-fuzz-campaign", "cargo-fuzz"),
        ("buoy-mutation-campaign", "cargo-mutants"),
        ("buoy-deny", "cargo-deny"),
        ("buoy-offline-acceptance", "clean-cargo-runner"),
        ("buoy-closure-size", "closure-report-runner"),
    ];
    for (script, package) in required {
        if !runtime
            .get(script)
            .is_some_and(|packages| packages.contains(package))
        {
            return Err(format!("DEVENV-RUNTIME-MISSING:{script}:{package}"));
        }
    }
    let specialized = policy
        .runtime_package
        .iter()
        .flat_map(|runtime| runtime.packages.iter())
        .filter(|package| !package.ends_with("-runner"))
        .collect::<BTreeSet<_>>();
    if let Some(package) =
        configuration_intersection(&policy.configuration.common_packages, &specialized)
    {
        return Err(format!("DEVENV-RUNTIME-COMMON:{package}"));
    }
    let allowed = [
        "aeneas",
        "cargo-deny",
        "cargo-fuzz",
        "cargo-llvm-cov",
        "cargo-mutants",
        "clean-cargo-runner",
        "closure-report-runner",
        "dylint",
        "lean-aeneas",
        "verus",
        "z3-standalone",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    for package in policy
        .runtime_package
        .iter()
        .flat_map(|runtime| runtime.packages.iter())
    {
        if !allowed.contains(package.as_str()) {
            return Err(format!("DEVENV-RUNTIME-AMBIENT:{package}"));
        }
    }
    let versions = runtime
        .get("buoy-versions")
        .ok_or_else(|| "DEVENV-RUNTIME-VERSION:buoy-versions".to_owned())?;
    for package in specialized {
        if !versions.contains(package.as_str()) {
            return Err(format!("DEVENV-RUNTIME-VERSION:{package}"));
        }
    }
    Ok(())
}

fn configuration_intersection<'a>(
    common: &'a [String],
    specialized: &BTreeSet<&String>,
) -> Option<&'a str> {
    common
        .iter()
        .find(|package| specialized.contains(package))
        .map(String::as_str)
}

fn validate_live_configuration(policy: &Policy, root: &Path) -> Result<(), String> {
    let yaml = read(&root.join("devenv.yaml"), "DEVENV-CONFIG-READ")?;
    if !yaml
        .lines()
        .any(|line| line.trim() == "require_version: true")
    {
        return Err("DEVENV-CONFIG-VERSION".to_owned());
    }
    let root_nix = read(&root.join("devenv.nix"), "DEVENV-CONFIG-READ")?;
    if root_nix.contains("fromRustupToolchainFile")
        || !root_nix.contains("config.languages.rust.toolchainPackage")
    {
        return Err("DEVENV-CONFIG-RUST".to_owned());
    }
    let toolchain_text = read(
        &root.join(&policy.configuration.rust_toolchain_file),
        "DEVENV-CONFIG-READ",
    )?;
    let toolchain = toml::from_str::<toml::Value>(&toolchain_text)
        .map_err(|error| format!("DEVENV-CONFIG-RUST:{error}"))?;
    let table = toolchain
        .get("toolchain")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "DEVENV-CONFIG-RUST:toolchain".to_owned())?;
    let string = |name: &str| {
        table
            .get(name)
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("DEVENV-CONFIG-RUST:{name}"))
    };
    let strings = |name: &str| {
        table
            .get(name)
            .and_then(toml::Value::as_array)
            .ok_or_else(|| format!("DEVENV-CONFIG-RUST:{name}"))?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("DEVENV-CONFIG-RUST:{name}"))
            })
            .collect::<Result<Vec<_>, _>>()
    };
    if string("channel")? != policy.configuration.rust_channel
        || string("profile")? != policy.configuration.rust_profile
        || strings("components")? != policy.configuration.rust_components
        || strings("targets")? != policy.configuration.rust_targets
    {
        return Err("DEVENV-CONFIG-RUST:toolchain-drift".to_owned());
    }
    for module in &policy.configuration.module_files {
        if !root.join(module).is_file() || !root_nix.contains(&format!("./{module}")) {
            return Err(format!("DEVENV-CONFIG-MODULE:{module}"));
        }
    }
    let hooks = read(&root.join("nix/devenv/hooks.nix"), "DEVENV-CONFIG-READ")?;
    let hook_contracts = [
        (
            // The rustfmt hook wraps the format command so it runs the pinned
            // toolchain in or out of the devenv shell. Assert the two properties
            // that matter rather than one brittle entry spelling: the check uses
            // the pinned toolchain, and it runs the exact format command.
            "buoy-rustfmt",
            [
                "config.languages.rust.toolchainPackage",
                "cargo fmt --all -- --check",
            ],
        ),
        (
            "buoy-nixfmt",
            [
                "entry = \"${pkgs.nixfmt}/bin/nixfmt --check\";",
                "package = pkgs.nixfmt;",
            ],
        ),
        (
            "buoy-openspec",
            [
                "entry = \"${pkgs.openspec}/bin/openspec validate --all --strict --no-interactive\";",
                "package = pkgs.openspec;",
            ],
        ),
    ];
    for hook in &policy.configuration.hooks {
        let (_, contract) = hook_contracts
            .iter()
            .find(|(name, _)| name == hook)
            .ok_or_else(|| format!("DEVENV-CONFIG-HOOK:{hook}"))?;
        if !hooks.contains(&format!("{hook} = {{"))
            || contract.iter().any(|line| !hooks.contains(line))
        {
            return Err(format!("DEVENV-CONFIG-HOOK:{hook}"));
        }
    }
    for forbidden in [
        "buoy-adversarial",
        "buoy-coverage",
        "buoy-formal-spikes",
        "buoy-fuzz",
        "buoy-gate",
        "buoy-mutation",
        "devenv test",
    ] {
        if hooks.contains(forbidden) {
            return Err(format!("DEVENV-CONFIG-HOOK-HEAVY:{forbidden}"));
        }
    }
    let environment = read(
        &root.join("nix/devenv/environment.nix"),
        "DEVENV-CONFIG-READ",
    )?;
    let packages = source_block(&environment, "  packages = [", "\n  ];")
        .ok_or_else(|| "DEVENV-CONFIG-COMMON-PACKAGES".to_owned())?;
    for package in &policy.configuration.common_packages {
        if !packages.contains(&format!("pkgs.{package}")) {
            return Err(format!("DEVENV-CONFIG-COMMON-PACKAGE:{package}"));
        }
    }
    for package in policy
        .runtime_package
        .iter()
        .flat_map(|runtime| runtime.packages.iter())
    {
        if packages.contains(runtime_package_expression(package)?) {
            return Err(format!("DEVENV-RUNTIME-COMMON:{package}"));
        }
    }
    let mut module_sources = BTreeMap::new();
    for owner in &policy.script_owner {
        let path = root.join(format!("nix/devenv/scripts/{}.nix", owner.module));
        let text = read(&path, "DEVENV-CONFIG-READ")?;
        for script in &owner.scripts {
            if !text.contains(&format!("scripts.{script} = {{")) {
                return Err(format!("DEVENV-CONFIG-SCRIPT-OWNER:{script}"));
            }
        }
        module_sources.insert(owner.module.as_str(), text);
    }
    for runtime in &policy.runtime_package {
        let module = policy
            .script_owner
            .iter()
            .find(|owner| owner.scripts.contains(&runtime.script))
            .ok_or_else(|| format!("DEVENV-CONFIG-SCRIPT-OWNER:{}", runtime.script))?;
        let text = module_sources
            .get(module.module.as_str())
            .expect("script owner module source exists");
        let marker = format!("scripts.{} = {{", runtime.script);
        let block = source_block(text, &marker, "\n  };")
            .expect("validated script body exists in cached module source");
        if !block.contains("packages = [") {
            return Err(format!("DEVENV-RUNTIME-MISSING:{}", runtime.script));
        }
        for package in &runtime.packages {
            let expression = runtime_package_expression(package)
                .expect("runtime packages passed closed-vocabulary validation");
            if !block.contains(expression) {
                return Err(format!(
                    "DEVENV-RUNTIME-MISSING:{}:{package}",
                    runtime.script
                ));
            }
        }
    }
    Ok(())
}

fn source_block<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let block = &text[text.find(start)?..];
    Some(&block[..block.find(end).unwrap_or(block.len())])
}

fn runtime_package_expression(package: &str) -> Result<&'static str, String> {
    match package {
        "aeneas" => Ok("formalTools.aeneas"),
        "cargo-deny" => Ok("pkgs.cargo-deny"),
        "cargo-fuzz" => Ok("pkgs.cargo-fuzz"),
        "cargo-llvm-cov" => Ok("pkgs.cargo-llvm-cov"),
        "cargo-mutants" => Ok("pkgs.cargo-mutants"),
        "clean-cargo-runner" => Ok("supplyChain.cleanCargoRunner"),
        "closure-report-runner" => Ok("supplyChain.closureReportRunner"),
        "dylint" => Ok("dylintTools"),
        "lean-aeneas" => Ok("formalTools.leanAeneas"),
        "verus" => Ok("formalTools.verus"),
        "z3-standalone" => Ok("formalTools.z3"),
        other => Err(format!("DEVENV-RUNTIME-AMBIENT:{other}")),
    }
}

fn parse_cases(text: &str, schema: &str) -> Result<Vec<Case>, String> {
    let mut lines = text.lines();
    if lines.next() != Some("schema\tcase_id\tpolarity\tmutation\texpected") {
        return Err("DEVENV-FIXTURE-HEADER".to_owned());
    }
    lines
        .enumerate()
        .filter(|(_, line)| !line.is_empty())
        .map(|(index, line)| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 5 || fields[0] != schema || fields[1].is_empty() {
                return Err(format!("DEVENV-FIXTURE-SHAPE:{}", index + 2));
            }
            Ok(Case {
                id: fields[1].to_owned(),
                row_index: index,
                source_row: line.to_owned(),
                polarity: fields[2].to_owned(),
                mutation: fields[3].to_owned(),
                expected: fields[4].to_owned(),
            })
        })
        .collect()
}

fn task_case_result(case: &Case, valid_text: &str, policy: &Policy) -> Result<usize, String> {
    if case.mutation == "malformed-json" {
        return Err(parse_tasks("{").expect_err("malformed task fixture"));
    }
    if case.mutation == "duplicate-manual-task" {
        let mut changed = policy.clone();
        changed.manual.push(changed.manual[0].clone());
        let tasks = parse_tasks(valid_text).expect("valid task fixture must parse");
        return validate_graph(tasks, &changed, "/repo");
    }
    if case.mutation == "manual-non-buoy-prefix" {
        let mut changed = policy.clone();
        changed.manual.push(String::from("custom:manual"));
        let mut tasks = parse_tasks(valid_text).expect("valid task fixture must parse");
        let mut manual = tasks
            .iter()
            .find(|task| task.name == "buoy:coverage")
            .expect("manual task fixture exists")
            .clone();
        String::from("custom:manual").clone_into(&mut manual.name);
        tasks.push(manual);
        return validate_graph(tasks, &changed, "/repo");
    }
    let mut tasks = parse_tasks(valid_text)?;
    match case.mutation.as_str() {
        "none" => {}
        "duplicate-task" => tasks.push(tasks[0].clone()),
        "unknown-dependency" => tasks[0].after.push("missing:task".to_owned()),
        "cycle" => tasks[0].after.push("buoy:gate".to_owned()),
        "missing-lifecycle" => task_mut(&mut tasks, "devenv:enterTest").after.clear(),
        "missing-phase" => tasks.retain(|task| task.name != "buoy:p3"),
        "nested-phase" => task_mut(&mut tasks, "buoy:p1")
            .after
            .push("buoy:p0".to_owned()),
        "duplicate-owner" => task_mut(&mut tasks, "buoy:p1")
            .after
            .push("buoy:schema".to_owned()),
        "direct-final-child" => task_mut(&mut tasks, "buoy:gate")
            .after
            .push("buoy:properties".to_owned()),
        "missing-manual-task" => {
            tasks.retain(|task| task.name != "buoy:mutation-campaign");
        }
        "manual-direct" => task_mut(&mut tasks, "buoy:gate")
            .after
            .push("buoy:coverage".to_owned()),
        "manual-transitive" => task_mut(&mut tasks, "buoy:fixtures")
            .after
            .push("buoy:mutation-campaign".to_owned()),
        "manual-alias-transitive" => {
            let mut alias = tasks
                .iter()
                .find(|task| task.name == "buoy:coverage")
                .expect("manual task fixture exists")
                .clone();
            "spec-provenance:coverage".clone_into(&mut alias.name);
            tasks.push(alias);
            task_mut(&mut tasks, "buoy:fixtures")
                .after
                .push("spec-provenance:coverage".to_owned());
        }
        "status" => task_mut(&mut tasks, "buoy:coverage").status = Some(Value::Bool(true)),
        "exec-if-modified" => task_mut(&mut tasks, "buoy:coverage")
            .exec_if_modified
            .push("src".to_owned()),
        "missing-root" => task_mut(&mut tasks, "buoy:coverage").cwd = None,
        "wrong-root" => task_mut(&mut tasks, "buoy:coverage").cwd = Some("/other".to_owned()),
        "missing-order-edge" => task_mut(&mut tasks, "buoy:reference").after.clear(),
        "missing-task" => tasks.retain(|task| task.name != "buoy:versions"),
        "unknown-before" => task_mut(&mut tasks, "buoy:gate")
            .before
            .push("missing:task".to_owned()),
        "missing-phase-child" => {
            task_mut(&mut tasks, "buoy:p0")
                .after
                .retain(|name| name != "buoy:properties");
        }
        "extra-phase-dependency" => task_mut(&mut tasks, "buoy:p0")
            .after
            .push("buoy:docs".to_owned()),
        "extra-sink-dependency" => task_mut(&mut tasks, "buoy:gate")
            .after
            .push("buoy:p3".to_owned()),
        "wrong-task-type" => {
            "process".clone_into(&mut task_mut(&mut tasks, "buoy:coverage").kind);
        }
        "missing-command" => task_mut(&mut tasks, "buoy:coverage").command = None,
        "missing-baseline-edge" => task_mut(&mut tasks, "buoy:devenv").after.clear(),
        "missing-phase-start-edge" => task_mut(&mut tasks, "buoy:schema").after.clear(),
        "missing-post-edge" => task_mut(&mut tasks, "buoy:specs").after.clear(),
        other => return Err(format!("DEVENV-FIXTURE-MUTATION:{other}")),
    }
    validate_graph(tasks, policy, "/repo")
}

fn task_mut<'a>(tasks: &'a mut [Task], name: &str) -> &'a mut Task {
    tasks
        .iter_mut()
        .find(|task| task.name == name)
        .expect("self-test task exists")
}

fn configuration_case_result(case: &Case, policy: &Policy) -> Result<(), String> {
    let mut changed = policy.clone();
    match case.mutation.as_str() {
        "none" => {}
        "missing-require-version" => changed.configuration.require_version = false,
        "missing-rust-module" => changed.configuration.rust_module = false,
        "wrong-toolchain-file" => {
            "other.toml".clone_into(&mut changed.configuration.rust_toolchain_file);
        }
        "wrong-rust-channel" => {
            "nightly".clone_into(&mut changed.configuration.rust_channel);
        }
        "wrong-rust-profile" => {
            "default".clone_into(&mut changed.configuration.rust_profile);
        }
        "missing-rust-analyzer" => changed
            .configuration
            .rust_components
            .retain(|component| component != "rust-analyzer"),
        "wrong-rust-target" => {
            changed.configuration.rust_targets.clear();
        }
        "missing-module" => {
            changed.configuration.module_files.pop();
        }
        "missing-hook" => {
            changed.configuration.hooks.pop();
        }
        "heavy-hook" => {
            changed.configuration.hooks.push("buoy-coverage".to_owned());
        }
        "duplicate-script-owner" => {
            let duplicate = changed.script_owner[0].scripts[0].clone();
            changed.script_owner[1].scripts.push(duplicate);
        }
        "empty-script-module" => changed.script_owner[0].module.clear(),
        "empty-script-list" => changed.script_owner[0].scripts.clear(),
        other => return Err(format!("DEVENV-FIXTURE-MUTATION:{other}")),
    }
    validate_configuration(&changed, None)
}

fn runtime_case_result(case: &Case, policy: &Policy) -> Result<(), String> {
    let mut changed = policy.clone();
    match case.mutation.as_str() {
        "none" => {}
        "missing-runtime-package" => runtime_mut(&mut changed, "buoy-fcis").packages.clear(),
        "specialized-in-common" => changed
            .configuration
            .common_packages
            .push("cargo-fuzz".to_owned()),
        "ambient-runtime-package" => runtime_mut(&mut changed, "buoy-fcis")
            .packages
            .push("ambient-host-tool".to_owned()),
        "missing-version-package" => runtime_mut(&mut changed, "buoy-versions")
            .packages
            .retain(|package| package != "cargo-fuzz"),
        "missing-version-runtime" => changed
            .runtime_package
            .retain(|runtime| runtime.script != "buoy-versions"),
        other => return Err(format!("DEVENV-FIXTURE-MUTATION:{other}")),
    }
    validate_runtime_packages(&changed)
}

fn runtime_mut<'a>(policy: &'a mut Policy, script: &str) -> &'a mut RuntimePackage {
    policy
        .runtime_package
        .iter_mut()
        .find(|runtime| runtime.script == script)
        .expect("self-test runtime package exists")
}

fn require_case_result<T>(case: &Case, result: Result<T, String>) -> Result<(), String> {
    match (case.polarity.as_str(), result) {
        ("positive", Ok(_)) if case.expected == "pass" => Ok(()),
        ("negative", Err(error)) if error.starts_with(&case.expected) => Ok(()),
        ("positive", Err(error)) => Err(format!("DEVENV-SELF-TEST-POSITIVE:{error}")),
        ("negative", Ok(_)) => Err(format!("DEVENV-SELF-TEST-NEGATIVE:{}", case.mutation)),
        (_, Err(error)) => Err(format!(
            "DEVENV-SELF-TEST-DIAGNOSTIC:{}:{error}",
            case.expected
        )),
        _ => Err("DEVENV-FIXTURE-POLARITY".to_owned()),
    }
}

fn execution_records<T, F>(
    path: &str,
    cases: &[Case],
    evaluate: F,
) -> Result<Vec<execution_evidence::Record>, String>
where
    F: Fn(&Case) -> Result<T, String>,
{
    let mut records = Vec::new();
    for case in cases {
        let result = evaluate(case);
        let observed = result
            .as_ref()
            .map_or_else(Clone::clone, |_| String::from("pass"));
        require_case_result(case, result)?;
        if case.mutation == "none" {
            continue;
        }
        records.push(
            execution_evidence::passing(
                "fixture-mutation",
                format!("{path}#{}#{}:{}", case.id, case.row_index, case.mutation),
                path,
                case.source_row.as_bytes(),
                &EXECUTION_TOOL_PATHS,
                &EXECUTION_CONFIGURATION_PATHS,
                format!(
                    "mutation={};observed={observed};expected={}",
                    case.mutation, case.expected
                ),
            )
            .expect(
                "fixture ids, repository paths, and single-line results are validated \
                 non-empty real inputs before this call",
            ),
        );
    }
    Ok(records)
}

fn assert_live_error(policy: &Policy, root: &Path, expected: &str) {
    let error = validate_live_configuration(policy, root).expect_err("negative live fixture");
    assert!(
        error.starts_with(expected),
        "expected {expected}, received {error}"
    );
}

fn write_fixture(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture path parent")).expect("create fixture path");
    fs::write(path, text).expect("write live configuration fixture");
}

fn live_configuration_self_test(root: &Path, policy: &Policy) {
    let scratch = env::temp_dir().join(format!("buoy-devenv-self-test-{}", std::process::id()));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("remove stale live configuration fixture root");
    }
    fs::create_dir_all(&scratch).expect("create live configuration fixture root");
    let fixture_root = scratch.as_path();
    let paths = [
        "devenv.yaml",
        "devenv.nix",
        "rust-toolchain.toml",
        "nix/devenv/environment.nix",
        "nix/devenv/hooks.nix",
        "nix/devenv/tasks.nix",
        "nix/devenv/scripts/core.nix",
        "nix/devenv/scripts/assurance.nix",
        "nix/devenv/scripts/adversarial.nix",
        "nix/devenv/scripts/acceptance.nix",
    ];
    let originals = paths
        .iter()
        .map(|path| {
            let text = fs::read_to_string(root.join(path)).expect("read live configuration source");
            write_fixture(fixture_root, path, &text);
            (*path, text)
        })
        .collect::<BTreeMap<_, _>>();
    assert!(validate_live_configuration(policy, fixture_root).is_ok());

    let check_text = |relative: &str, changed: String, expected: &str| {
        write_fixture(fixture_root, relative, &changed);
        assert_live_error(policy, fixture_root, expected);
        write_fixture(
            fixture_root,
            relative,
            originals.get(relative).expect("fixture source exists"),
        );
    };
    check_text(
        "devenv.yaml",
        originals["devenv.yaml"].replace("require_version: true", "require_version: false"),
        "DEVENV-CONFIG-VERSION",
    );
    check_text(
        "devenv.nix",
        format!("fromRustupToolchainFile\n{}", originals["devenv.nix"]),
        "DEVENV-CONFIG-RUST",
    );
    check_text(
        "devenv.nix",
        originals["devenv.nix"].replace(
            "config.languages.rust.toolchainPackage",
            "rustPkgs.rust-bin.stable.latest.default",
        ),
        "DEVENV-CONFIG-RUST",
    );
    fs::remove_file(fixture_root.join("devenv.nix")).expect("remove root Nix fixture");
    assert_live_error(policy, fixture_root, "DEVENV-CONFIG-READ");
    write_fixture(fixture_root, "devenv.nix", &originals["devenv.nix"]);
    check_text("rust-toolchain.toml", "{".to_owned(), "DEVENV-CONFIG-RUST");
    check_text(
        "rust-toolchain.toml",
        "[other]\nvalue = true\n".to_owned(),
        "DEVENV-CONFIG-RUST:toolchain",
    );
    check_text(
        "rust-toolchain.toml",
        originals["rust-toolchain.toml"].replace("channel = \"nightly-2026-04-16\"", "channel = 1"),
        "DEVENV-CONFIG-RUST:channel",
    );
    check_text(
        "rust-toolchain.toml",
        originals["rust-toolchain.toml"]
            .replace("channel = \"nightly-2026-04-16\"", "channel = \"nightly\""),
        "DEVENV-CONFIG-RUST:toolchain-drift",
    );
    check_text(
        "rust-toolchain.toml",
        originals["rust-toolchain.toml"].replace("profile = \"minimal\"", "profile = 1"),
        "DEVENV-CONFIG-RUST:profile",
    );
    check_text(
        "rust-toolchain.toml",
        originals["rust-toolchain.toml"].replace("profile = \"minimal\"", "profile = \"default\""),
        "DEVENV-CONFIG-RUST:toolchain-drift",
    );
    check_text(
        "rust-toolchain.toml",
        originals["rust-toolchain.toml"].replace(
            "components = [\"clippy\", \"llvm-tools-preview\", \"miri\", \"rust-analyzer\", \"rust-src\", \"rustc-dev\", \"rustfmt\"]",
            "components = \"clippy\"",
        ),
        "DEVENV-CONFIG-RUST:components",
    );
    check_text(
        "rust-toolchain.toml",
        originals["rust-toolchain.toml"].replace(
            "components = [\"clippy\", \"llvm-tools-preview\", \"miri\", \"rust-analyzer\", \"rust-src\", \"rustc-dev\", \"rustfmt\"]",
            "components = [1]",
        ),
        "DEVENV-CONFIG-RUST:components",
    );
    check_text(
        "rust-toolchain.toml",
        originals["rust-toolchain.toml"].replace("\"rust-analyzer\", ", ""),
        "DEVENV-CONFIG-RUST:toolchain-drift",
    );
    check_text(
        "rust-toolchain.toml",
        originals["rust-toolchain.toml"].replace(
            "targets = [\"thumbv7em-none-eabi\"]",
            "targets = \"thumbv7em-none-eabi\"",
        ),
        "DEVENV-CONFIG-RUST:targets",
    );
    check_text(
        "rust-toolchain.toml",
        originals["rust-toolchain.toml"]
            .replace("targets = [\"thumbv7em-none-eabi\"]", "targets = []"),
        "DEVENV-CONFIG-RUST:toolchain-drift",
    );
    fs::remove_file(fixture_root.join("rust-toolchain.toml"))
        .expect("remove Rust toolchain fixture");
    assert_live_error(policy, fixture_root, "DEVENV-CONFIG-READ");
    write_fixture(
        fixture_root,
        "rust-toolchain.toml",
        &originals["rust-toolchain.toml"],
    );

    let environment_path = fixture_root.join("nix/devenv/environment.nix");
    fs::remove_file(&environment_path).expect("remove module fixture");
    assert_live_error(policy, fixture_root, "DEVENV-CONFIG-MODULE");
    write_fixture(
        fixture_root,
        "nix/devenv/environment.nix",
        &originals["nix/devenv/environment.nix"],
    );
    check_text(
        "devenv.nix",
        originals["devenv.nix"].replace("    ./nix/devenv/environment.nix\n", ""),
        "DEVENV-CONFIG-MODULE",
    );
    let mut missing_environment = policy.clone();
    missing_environment
        .configuration
        .module_files
        .retain(|path| path != "nix/devenv/environment.nix");
    fs::remove_file(&environment_path).expect("remove environment fixture");
    assert_live_error(&missing_environment, fixture_root, "DEVENV-CONFIG-READ");
    write_fixture(
        fixture_root,
        "nix/devenv/environment.nix",
        &originals["nix/devenv/environment.nix"],
    );

    let mut missing_hooks = policy.clone();
    missing_hooks
        .configuration
        .module_files
        .retain(|path| path != "nix/devenv/hooks.nix");
    fs::remove_file(fixture_root.join("nix/devenv/hooks.nix")).expect("remove hook fixture");
    assert_live_error(&missing_hooks, fixture_root, "DEVENV-CONFIG-READ");
    write_fixture(
        fixture_root,
        "nix/devenv/hooks.nix",
        &originals["nix/devenv/hooks.nix"],
    );

    let mut unknown_hook = policy.clone();
    unknown_hook.configuration.hooks = vec!["unknown-hook".to_owned()];
    assert_live_error(
        &unknown_hook,
        fixture_root,
        "DEVENV-CONFIG-HOOK:unknown-hook",
    );
    check_text(
        "nix/devenv/hooks.nix",
        originals["nix/devenv/hooks.nix"].replace("buoy-rustfmt = {", "other-rustfmt = {"),
        "DEVENV-CONFIG-HOOK:buoy-rustfmt",
    );
    check_text(
        "nix/devenv/hooks.nix",
        originals["nix/devenv/hooks.nix"].replace(
            "${config.languages.rust.toolchainPackage}/bin",
            "/run/current-system/sw/bin",
        ),
        "DEVENV-CONFIG-HOOK:buoy-rustfmt",
    );
    check_text(
        "nix/devenv/hooks.nix",
        originals["nix/devenv/hooks.nix"]
            .replace("exec cargo fmt --all -- --check", "exec cargo fmt"),
        "DEVENV-CONFIG-HOOK:buoy-rustfmt",
    );
    check_text(
        "nix/devenv/hooks.nix",
        originals["nix/devenv/hooks.nix"].replace(
            "entry = \"${pkgs.nixfmt}/bin/nixfmt --check\";",
            "entry = \"nixfmt --check\";",
        ),
        "DEVENV-CONFIG-HOOK:buoy-nixfmt",
    );
    check_text(
        "nix/devenv/hooks.nix",
        originals["nix/devenv/hooks.nix"].replace(
            "entry = \"${pkgs.openspec}/bin/openspec validate --all --strict --no-interactive\";",
            "entry = \"openspec validate --all --strict --no-interactive\";",
        ),
        "DEVENV-CONFIG-HOOK:buoy-openspec",
    );
    check_text(
        "nix/devenv/hooks.nix",
        format!("# buoy-coverage\n{}", originals["nix/devenv/hooks.nix"]),
        "DEVENV-CONFIG-HOOK-HEAVY:buoy-coverage",
    );
    check_text(
        "nix/devenv/environment.nix",
        originals["nix/devenv/environment.nix"]
            .replace("  packages = [", "  environmentPackages = ["),
        "DEVENV-CONFIG-COMMON-PACKAGES",
    );
    check_text(
        "nix/devenv/environment.nix",
        originals["nix/devenv/environment.nix"].replace("    pkgs.rustup\n", ""),
        "DEVENV-CONFIG-COMMON-PACKAGE:rustup",
    );
    check_text(
        "nix/devenv/environment.nix",
        originals["nix/devenv/environment.nix"]
            .replace("  packages = [", "  packages = [\n    pkgs.cargo-fuzz"),
        "DEVENV-RUNTIME-COMMON:cargo-fuzz",
    );
    check_text(
        "nix/devenv/scripts/acceptance.nix",
        originals["nix/devenv/scripts/acceptance.nix"]
            .replace("scripts.buoy-versions = {", "scripts.other-versions = {"),
        "DEVENV-CONFIG-SCRIPT-OWNER:buoy-versions",
    );
    check_text(
        "nix/devenv/scripts/acceptance.nix",
        originals["nix/devenv/scripts/acceptance.nix"].replacen(
            "    packages = [",
            "    inputs = [",
            1,
        ),
        "DEVENV-RUNTIME-MISSING:buoy-versions",
    );
    check_text(
        "nix/devenv/scripts/acceptance.nix",
        originals["nix/devenv/scripts/acceptance.nix"].replace("      pkgs.cargo-fuzz\n", ""),
        "DEVENV-RUNTIME-MISSING:buoy-versions:cargo-fuzz",
    );

    let mut ambient_runtime = policy.clone();
    runtime_mut(&mut ambient_runtime, "buoy-versions")
        .packages
        .push("ambient-host-tool".to_owned());
    assert_live_error(
        &ambient_runtime,
        fixture_root,
        "DEVENV-RUNTIME-AMBIENT:ambient-host-tool",
    );

    let mut missing_owner = policy.clone();
    for owner in &mut missing_owner.script_owner {
        owner.scripts.retain(|script| script != "buoy-fcis");
    }
    assert_live_error(
        &missing_owner,
        fixture_root,
        "DEVENV-CONFIG-SCRIPT-OWNER:buoy-fcis",
    );

    let mut missing_script_module = policy.clone();
    missing_script_module
        .configuration
        .module_files
        .retain(|path| path != "nix/devenv/scripts/acceptance.nix");
    fs::remove_file(fixture_root.join("nix/devenv/scripts/acceptance.nix"))
        .expect("remove script module fixture");
    assert_live_error(&missing_script_module, fixture_root, "DEVENV-CONFIG-READ");
    fs::remove_dir_all(&scratch).expect("remove live configuration fixture root");
}

fn malformed_self_test_controls(root: &Path) {
    let scratch = env::temp_dir().join(format!(
        "buoy-devenv-malformed-self-test-{}",
        std::process::id()
    ));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("remove stale malformed self-test root");
    }
    let paths = [
        POLICY_PATH,
        VALID_TASKS_PATH,
        TASK_CASES_PATH,
        CONFIG_CASES_PATH,
        RUNTIME_CASES_PATH,
    ];
    let originals = paths
        .iter()
        .map(|path| {
            let text = fs::read_to_string(root.join(path)).expect("read self-test fixture");
            write_fixture(&scratch, path, &text);
            (*path, text)
        })
        .collect::<BTreeMap<_, _>>();
    for path in paths {
        fs::remove_file(scratch.join(path)).expect("remove self-test fixture");
        assert!(self_test(&scratch, false).is_err());
        write_fixture(
            &scratch,
            path,
            originals.get(path).expect("self-test fixture exists"),
        );
    }
    write_fixture(&scratch, POLICY_PATH, "not toml");
    assert!(self_test(&scratch, false).is_err());
    write_fixture(&scratch, POLICY_PATH, &originals[POLICY_PATH]);
    write_fixture(&scratch, VALID_TASKS_PATH, "{");
    assert!(self_test(&scratch, false).is_err());
    write_fixture(&scratch, VALID_TASKS_PATH, &originals[VALID_TASKS_PATH]);
    for path in [TASK_CASES_PATH, CONFIG_CASES_PATH, RUNTIME_CASES_PATH] {
        let changed =
            originals[path].replacen("\tpositive\tnone\tpass", "\tpositive\tnone\tnot-pass", 1);
        write_fixture(&scratch, path, &changed);
        assert!(self_test(&scratch, false).is_err());
        write_fixture(&scratch, path, &originals[path]);
    }
    for path in [TASK_CASES_PATH, CONFIG_CASES_PATH, RUNTIME_CASES_PATH] {
        write_fixture(&scratch, path, "wrong header\n");
        assert!(self_test(&scratch, false).is_err());
        write_fixture(
            &scratch,
            path,
            originals.get(path).expect("self-test fixture exists"),
        );
    }
    let reduced = originals[TASK_CASES_PATH]
        .lines()
        .filter(|line| !line.contains("\tnegative\t"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    write_fixture(&scratch, TASK_CASES_PATH, &reduced);
    let stale = self_test(&scratch, false).expect_err("reduced fixture cases must be stale");
    assert!(
        stale.starts_with("EXECUTION-EVIDENCE-STALE"),
        "expected stale evidence, received {stale}"
    );
    write_fixture(&scratch, TASK_CASES_PATH, &originals[TASK_CASES_PATH]);
    fs::remove_dir_all(&scratch).expect("remove malformed self-test root");
}

fn current_root() -> Result<PathBuf, String> {
    env::current_dir().map_err(|error| format!("DEVENV-ROOT:{error}"))
}

fn coverage_self_test(root: &Path, policy: &Policy, policy_text: &str, valid_tasks: &str) {
    assert!(read(&root.join("missing-devenv-fixture"), "DEVENV-COVERAGE").is_err());
    assert!(parse_policy("not toml").is_err());
    let wrong_schema_text = policy_text.replacen(POLICY_SCHEMA, "wrong", 1);
    assert_eq!(
        parse_policy(&wrong_schema_text).expect_err("wrong policy schema"),
        "DEVENV-POLICY-SCHEMA:wrong"
    );
    assert!(source_block("text", "missing", "end").is_none());
    assert_eq!(
        source_block("start body", "start", "missing"),
        Some("start body")
    );
    assert_eq!(
        runtime_package_expression("ambient").expect_err("ambient runtime package"),
        "DEVENV-RUNTIME-AMBIENT:ambient"
    );
    assert!(parse_cases("wrong", TASK_CASE_SCHEMA).is_err());
    assert!(
        parse_cases(
            "schema\tcase_id\tpolarity\tmutation\texpected\nshort",
            TASK_CASE_SCHEMA
        )
        .is_err()
    );
    assert!(
        parse_cases(
            "schema\tcase_id\tpolarity\tmutation\texpected\nwrong\tid\tpositive\tnone\tpass",
            TASK_CASE_SCHEMA
        )
        .is_err()
    );
    assert!(parse_cases(
        &format!(
            "schema\tcase_id\tpolarity\tmutation\texpected\n{TASK_CASE_SCHEMA}\t\tpositive\tnone\tpass"
        ),
        TASK_CASE_SCHEMA
    )
    .is_err());
    assert_eq!(
        parse_cases(
            &format!(
                "schema\tcase_id\tpolarity\tmutation\texpected\n\n{TASK_CASE_SCHEMA}\tid\tpositive\tnone\tpass"
            ),
            TASK_CASE_SCHEMA
        )
        .expect("blank fixture rows are ignored")
        .len(),
        1
    );

    let unknown_case = Case {
        id: "unknown".to_owned(),
        row_index: 0,
        source_row: "unknown".to_owned(),
        polarity: "negative".to_owned(),
        mutation: "unknown".to_owned(),
        expected: "DEVENV-FIXTURE-MUTATION".to_owned(),
    };
    assert!(task_case_result(&unknown_case, valid_tasks, policy).is_err());
    assert!(configuration_case_result(&unknown_case, policy).is_err());
    assert!(runtime_case_result(&unknown_case, policy).is_err());
    let positive = Case {
        id: "positive".to_owned(),
        row_index: 0,
        source_row: "positive".to_owned(),
        polarity: "positive".to_owned(),
        mutation: "control".to_owned(),
        expected: "pass".to_owned(),
    };
    assert!(require_case_result::<()>(&positive, Err("error".to_owned())).is_err());
    assert!(require_case_result::<usize>(&positive, Err("error".to_owned())).is_err());
    let negative = Case {
        id: "negative".to_owned(),
        row_index: 0,
        source_row: "negative".to_owned(),
        polarity: "negative".to_owned(),
        mutation: "control".to_owned(),
        expected: "expected".to_owned(),
    };
    assert!(require_case_result(&negative, Ok(())).is_err());
    assert!(require_case_result::<usize>(&negative, Ok(0)).is_err());
    assert!(require_case_result::<()>(&negative, Err("other".to_owned())).is_err());
    assert!(require_case_result::<usize>(&negative, Err("other".to_owned())).is_err());
    let invalid = Case {
        id: "invalid".to_owned(),
        row_index: 0,
        source_row: "invalid".to_owned(),
        polarity: "invalid".to_owned(),
        mutation: "control".to_owned(),
        expected: "pass".to_owned(),
    };
    assert!(require_case_result(&invalid, Ok(())).is_err());
    assert!(require_case_result::<usize>(&invalid, Ok(0)).is_err());
    let unexpected_positive = Case {
        expected: "not-pass".to_owned(),
        ..positive
    };
    assert!(require_case_result(&unexpected_positive, Ok(())).is_err());
    assert!(require_case_result::<usize>(&unexpected_positive, Ok(0)).is_err());

    let mut baseline_empty = policy.clone();
    baseline_empty.baseline.clear();
    assert!(
        validate_graph(
            parse_tasks(valid_tasks).expect("valid task fixture"),
            &baseline_empty,
            "/repo"
        )
        .is_err()
    );
    let mut phase_empty = policy.clone();
    phase_empty.phase[0].children.clear();
    assert!(
        validate_graph(
            parse_tasks(valid_tasks).expect("valid task fixture"),
            &phase_empty,
            "/repo"
        )
        .is_err()
    );
    let mut missing_runtime = policy.clone();
    runtime_mut(&mut missing_runtime, "buoy-fcis")
        .packages
        .clear();
    assert!(validate_configuration(&missing_runtime, None).is_err());

    let missing = root.join("missing-devenv-check-fixture");
    assert!(
        check(&CheckArguments {
            tasks: root.join(VALID_TASKS_PATH),
            policy: missing.clone(),
            root: root.to_path_buf(),
        })
        .is_err()
    );
    assert!(
        check(&CheckArguments {
            tasks: root.join(VALID_TASKS_PATH),
            policy: root.join(POLICY_PATH),
            root: missing.clone(),
        })
        .is_err()
    );
    assert!(
        check(&CheckArguments {
            tasks: missing.clone(),
            policy: root.join(POLICY_PATH),
            root: root.to_path_buf(),
        })
        .is_err()
    );
    let empty_root = env::temp_dir().join(format!("buoy-devenv-empty-root-{}", std::process::id()));
    fs::create_dir_all(&empty_root).expect("create empty check root");
    assert!(
        check(&CheckArguments {
            tasks: root.join(VALID_TASKS_PATH),
            policy: root.join(POLICY_PATH),
            root: empty_root.clone(),
        })
        .is_err()
    );
    fs::remove_dir_all(empty_root).expect("remove empty check root");
    let bad_policy = env::temp_dir().join(format!("buoy-devenv-bad-policy-{}", std::process::id()));
    fs::write(&bad_policy, "not toml").expect("write malformed policy data");
    assert!(
        check(&CheckArguments {
            tasks: root.join(VALID_TASKS_PATH),
            policy: bad_policy.clone(),
            root: root.to_path_buf(),
        })
        .is_err()
    );
    fs::remove_file(bad_policy).expect("remove malformed policy data");
    let bad_tasks = env::temp_dir().join(format!("buoy-devenv-bad-tasks-{}", std::process::id()));
    fs::write(&bad_tasks, "{").expect("write malformed task data");
    assert!(
        check(&CheckArguments {
            tasks: bad_tasks.clone(),
            policy: root.join(POLICY_PATH),
            root: root.to_path_buf(),
        })
        .is_err()
    );
    fs::write(&bad_tasks, "[]").expect("write incomplete task data");
    assert!(
        check(&CheckArguments {
            tasks: bad_tasks.clone(),
            policy: root.join(POLICY_PATH),
            root: root.to_path_buf(),
        })
        .is_err()
    );
    fs::remove_file(bad_tasks).expect("remove bad task data");

    assert!(normalized_root(&root.join("missing-root")).is_err());
    #[cfg(unix)]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let invalid = PathBuf::from(OsString::from_vec(vec![0xff, b'd', b'e', b'v']));
        assert_eq!(
            utf8_root(&invalid).expect_err("reject non-UTF-8 root"),
            "DEVENV-ROOT-UTF8"
        );
    }
    assert!(parse_check_arguments(&["--invalid".to_owned()]).is_err());
    assert!(parse_check_arguments(&["--tasks".to_owned()]).is_err());
    assert!(parse_check_arguments(&["--policy".to_owned()]).is_err());
    assert!(parse_check_arguments(&["--root".to_owned()]).is_err());

    let original_cwd = current_root().expect("read current root");
    let vanished = env::temp_dir().join(format!("buoy-devenv-vanished-{}", std::process::id()));
    fs::create_dir_all(&vanished).expect("create vanished root");
    env::set_current_dir(&vanished).expect("enter vanished root");
    fs::remove_dir_all(&vanished).expect("remove current root");
    assert!(current_root().is_err());
    assert!(parse_check_arguments(&[]).is_err());
    assert!(run().is_err());
    env::set_current_dir(original_cwd).expect("restore current root");

    let live_scratch =
        env::temp_dir().join(format!("buoy-devenv-self-test-{}", std::process::id()));
    fs::create_dir_all(&live_scratch).expect("create stale live fixture root");
    live_configuration_self_test(root, policy);
    live_configuration_self_test(root, policy);
    let malformed_scratch = env::temp_dir().join(format!(
        "buoy-devenv-malformed-self-test-{}",
        std::process::id()
    ));
    fs::create_dir_all(&malformed_scratch).expect("create stale malformed self-test root");
    malformed_self_test_controls(root);
    malformed_self_test_controls(root);
}

fn self_test(root: &Path, write_execution_evidence: bool) -> Result<String, String> {
    let policy_text = read(&root.join(POLICY_PATH), "DEVENV-SELF-TEST-READ")?;
    let policy = parse_policy(&policy_text)?;
    let valid_tasks = read(&root.join(VALID_TASKS_PATH), "DEVENV-SELF-TEST-READ")?;
    let task_cases = parse_cases(
        &read(&root.join(TASK_CASES_PATH), "DEVENV-SELF-TEST-READ")?,
        TASK_CASE_SCHEMA,
    )?;
    let config_cases = parse_cases(
        &read(&root.join(CONFIG_CASES_PATH), "DEVENV-SELF-TEST-READ")?,
        CONFIG_CASE_SCHEMA,
    )?;
    let runtime_cases = parse_cases(
        &read(&root.join(RUNTIME_CASES_PATH), "DEVENV-SELF-TEST-READ")?,
        RUNTIME_CASE_SCHEMA,
    )?;

    let mut records = execution_records(TASK_CASES_PATH, &task_cases, |case| {
        task_case_result(case, &valid_tasks, &policy)
    })?;
    records.extend(execution_records(
        CONFIG_CASES_PATH,
        &config_cases,
        |case| configuration_case_result(case, &policy),
    )?);
    records.extend(execution_records(
        RUNTIME_CASES_PATH,
        &runtime_cases,
        |case| runtime_case_result(case, &policy),
    )?);
    let execution_count = records.len();
    // Record ids embed the fixture path and row index, so two records can
    // never collide and rendering cannot fail.
    let executions = execution_evidence::render(&records)
        .expect("every record id embeds a distinct fixture path and row index");
    execution_evidence::validate_materialized(
        EXECUTION_EVIDENCE_PATH,
        &executions,
        write_execution_evidence,
    )?;

    coverage_self_test(root, &policy, &policy_text, &valid_tasks);
    Ok(format!(
        "devenv-workflow-self-test: task-cases={} config-cases={} runtime-cases={} executions={execution_count}",
        task_cases.len(),
        config_cases.len(),
        runtime_cases.len()
    ))
}

#[derive(Debug)]
struct CheckArguments {
    tasks: PathBuf,
    policy: PathBuf,
    root: PathBuf,
}

fn check(arguments: &CheckArguments) -> Result<String, String> {
    let policy = parse_policy(&read(&arguments.policy, "DEVENV-POLICY-READ")?)?;
    let root = normalized_root(&arguments.root)?;
    validate_configuration(&policy, Some(Path::new(&root)))?;
    let tasks = parse_tasks(&read(&arguments.tasks, "DEVENV-TASK-READ")?)?;
    let count = validate_graph(tasks, &policy, &root)?;
    let children = policy
        .phase
        .iter()
        .map(|phase| phase.children.len())
        .sum::<usize>();
    Ok(format!(
        "devenv-workflow: tasks={count} phases={} children={children} root={}",
        policy.phase.len(),
        policy.root_token
    ))
}

fn normalized_root(root: &Path) -> Result<String, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("DEVENV-ROOT:{error}"))?;
    utf8_root(&root)
}

fn utf8_root(root: &Path) -> Result<String, String> {
    match root.to_str() {
        Some(value) => Ok(value.to_owned()),
        None => Err("DEVENV-ROOT-UTF8".to_owned()),
    }
}

fn usage() -> String {
    "DEVENV-USAGE: check-devenv-task-graph (--self-test | --write-execution-evidence | --check [--tasks PATH] [--policy PATH] [--root PATH])".to_owned()
}

fn parse_check_arguments(arguments: &[String]) -> Result<CheckArguments, String> {
    let mut tasks = env::var_os("DEVENV_TASK_FILE").map(PathBuf::from);
    let mut policy = PathBuf::from(POLICY_PATH);
    let mut root = current_root()?;
    let mut index = 0;
    while index < arguments.len() {
        let target = match arguments[index].as_str() {
            "--tasks" => &mut tasks,
            "--policy" => {
                let value = arguments.get(index + 1).ok_or_else(usage)?;
                policy = PathBuf::from(value);
                index += 2;
                continue;
            }
            "--root" => {
                let value = arguments.get(index + 1).ok_or_else(usage)?;
                root = PathBuf::from(value);
                index += 2;
                continue;
            }
            _ => return Err(usage()),
        };
        let value = arguments.get(index + 1).ok_or_else(usage)?;
        *target = Some(PathBuf::from(value));
        index += 2;
    }
    let Some(tasks) = tasks else {
        return Err("DEVENV-TASK-FILE-MISSING".to_owned());
    };
    Ok(CheckArguments {
        tasks,
        policy,
        root,
    })
}

fn run() -> Result<String, String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [flag] if flag == "--self-test" => self_test(&current_root()?, false),
        [flag] if flag == "--write-execution-evidence" => self_test(&current_root()?, true),
        [flag, rest @ ..] if flag == "--check" => check(&parse_check_arguments(rest)?),
        _ => Err(usage()),
    }
}

fn main() {
    match run() {
        Ok(message) => println!("{message}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
