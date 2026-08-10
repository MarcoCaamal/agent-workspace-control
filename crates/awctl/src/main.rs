//! `awctl` — AWC command-line boundary: parsing, deterministic JSON/human
//! rendering, and exit codes (0 success, 1 operational, 2 usage, 3 not-found).

use std::path::{Path, PathBuf};

use awc_core::{
    application,
    domain::{AddProject, Artifact, ArtifactStatus, CheckResult, CommandResult, Project},
    error::AwcError,
};
use clap::{Parser, Subcommand};
use serde::Serialize;

/// Control AWC workspaces (synchronous; no runtime).
#[derive(Parser)]
struct Cli {
    /// Emit one newline-terminated JSON document on stdout.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a workspace in the current directory.
    Init,
    Status,
    Doctor {
        /// Only the quick check set is supported; the flag is required.
        #[arg(long, required = true)]
        quick: bool,
    },
    /// Manage projects.
    Project {
        #[command(subcommand)]
        action: ProjectCommand,
    },
    /// Manage governed artifacts.
    Artifact {
        #[command(subcommand)]
        action: ArtifactCommand,
    },
}

#[derive(Subcommand)]
enum ArtifactCommand {
    /// Create a new empty governed artifact under artifacts/.
    Create {
        /// Project ID or unique prefix.
        #[arg(long)]
        project: String,
        /// Artifact type (e.g. doc, report).
        #[arg(long)]
        r#type: String,
        /// Artifact title.
        #[arg(long)]
        title: String,
    },
    /// Show one artifact by ID or unique prefix.
    Show { id: String },
    /// List artifacts with optional filters.
    List {
        /// Filter by project ID or unique prefix.
        #[arg(long)]
        project: Option<String>,
        /// Filter by artifact type.
        #[arg(long)]
        r#type: Option<String>,
        /// Filter by status (active, archived, trashed).
        #[arg(long)]
        status: Option<String>,
    },
    /// Archive an artifact (status-only).
    Archive { id: String },
    /// Move an artifact into governed trash.
    Trash { id: String },
    /// Restore a trashed artifact to its original path.
    Restore { id: String },
    /// Relink an artifact to a new artifacts/ path.
    Relink {
        id: String,
        /// New relative path under artifacts/.
        #[arg(long)]
        path: String,
    },
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// Add a project; the slug derives from the name unless --slug is given.
    Add {
        /// Project name.
        #[arg(long)]
        name: String,
        /// Explicit canonical slug, validated by the slug rules.
        #[arg(long)]
        slug: Option<String>,
        /// External root path — stored as metadata only, never written.
        #[arg(long)]
        root_path: Option<PathBuf>,
    },
    /// List all projects in slug order.
    List,
    /// Show one project by ID or unique prefix.
    Show {
        /// Project ID or unique prefix.
        id: String,
    },
}

fn main() {
    let cli = Cli::parse(); // usage errors print to stderr and exit 2
    let cwd = std::env::current_dir().expect("current directory");
    let result = match cli.command {
        Command::Init => application::init(&cwd),
        Command::Status => application::status(&cwd),
        Command::Doctor { .. } => application::doctor_quick(&cwd),
        Command::Project { action } => match action {
            ProjectCommand::Add {
                name,
                slug,
                root_path,
            } => application::add_project(
                &cwd,
                AddProject {
                    name,
                    slug,
                    root_path,
                },
            ),
            ProjectCommand::List => application::list_projects(&cwd),
            ProjectCommand::Show { id } => application::show_project(&cwd, &id),
        },
        Command::Artifact { action } => match action {
            ArtifactCommand::Create {
                project,
                r#type,
                title,
            } => application::create_artifact(&cwd, &project, &r#type, &title),
            ArtifactCommand::Show { id } => application::show_artifact(&cwd, &id),
            ArtifactCommand::List {
                project,
                r#type,
                status,
            } => {
                let status = parse_status_filter(status.as_deref());
                status.and_then(|status| {
                    application::list_artifacts(&cwd, project.as_deref(), r#type.as_deref(), status)
                })
            }
            ArtifactCommand::Archive { id } => application::archive_artifact(&cwd, &id),
            ArtifactCommand::Trash { id } => application::trash_artifact(&cwd, &id),
            ArtifactCommand::Restore { id } => application::restore_artifact(&cwd, &id),
            ArtifactCommand::Relink { id, path } => application::relink_artifact(&cwd, &id, &path),
        },
    };
    match result {
        Ok(result) if cli.json => render_json(&result),
        Ok(result) => render_human(&result),
        Err(err) => {
            render_error(cli.json, &err);
            std::process::exit(err.exit_code());
        }
    }
}

/// Parses an optional status filter; an invalid value is a usage error.
fn parse_status_filter(raw: Option<&str>) -> Result<Option<ArtifactStatus>, AwcError> {
    match raw {
        None => Ok(None),
        Some(value) => ArtifactStatus::parse(value).map(Some).ok_or_else(|| {
            AwcError::Usage("invalid status; expected active, archived, or trashed".into())
        }),
    }
}

// --- JSON views: compact, typed, declaration order; paths only where the
// contract requires them (the reported root). --------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceView {
    root: String,
    schema_version: u32,
    database_ok: bool,
    schema_ok: bool,
}

#[derive(Serialize)]
struct CheckView {
    name: &'static str,
    ok: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectView {
    id: String,
    slug: String,
    name: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    root_path: Option<String>,
}

fn project_view(p: &Project) -> ProjectView {
    ProjectView {
        id: p.id.0.to_string(),
        slug: p.slug.clone(),
        name: p.name.clone(),
        status: p.status.clone(),
        root_path: p.root_path.as_ref().map(|path| path.display().to_string()),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactView {
    id: String,
    project_id: String,
    artifact_type: String,
    title: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
    last_seen_at: String,
    created_at: String,
    updated_at: String,
}

fn artifact_view(a: &Artifact) -> ArtifactView {
    ArtifactView {
        id: a.id.0.to_string(),
        project_id: a.project_id.0.to_string(),
        artifact_type: a.artifact_type.clone(),
        title: a.title.clone(),
        status: a.status.as_str().to_string(),
        path: a.path.as_ref().map(|p| p.display().to_string()),
        original_path: a.original_path.as_ref().map(|p| p.display().to_string()),
        sha256: a.sha256.clone(),
        size: a.size,
        last_seen_at: a.last_seen_at.clone(),
        created_at: a.created_at.clone(),
        updated_at: a.updated_at.clone(),
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum DataView {
    Workspace(WorkspaceView),
    Doctor {
        root: String,
        checks: Vec<CheckView>,
    },
    Project {
        project: ProjectView,
    },
    ProjectList {
        projects: Vec<ProjectView>,
    },
    Artifact {
        artifact: ArtifactView,
    },
    ArtifactList {
        artifacts: Vec<ArtifactView>,
    },
}

#[derive(Serialize)]
struct ErrorView {
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonDoc {
    schema_version: u32,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<DataView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorView>,
}

/// Workspace summary for Init/Status, or None for Doctor and project results.
fn parts(result: &CommandResult) -> Option<(&Path, u32, bool, bool)> {
    match result {
        CommandResult::Init(s) => Some((&s.root, s.schema_version, s.database_ok, s.schema_ok)),
        CommandResult::Status(s) => Some((&s.root, s.schema_version, s.database_ok, s.schema_ok)),
        CommandResult::Doctor(_)
        | CommandResult::ProjectAdded(_)
        | CommandResult::ProjectList(_)
        | CommandResult::ProjectShown(_)
        | CommandResult::ArtifactCreated(_)
        | CommandResult::ArtifactList(_)
        | CommandResult::ArtifactShown(_) => None,
        CommandResult::AdoptScan(_) => None,
        CommandResult::AdoptPlanCreated { .. } => None,
    }
}

fn ok_doc(data: DataView) -> JsonDoc {
    JsonDoc {
        schema_version: 1,
        ok: true,
        data: Some(data),
        error: None,
    }
}

fn write_json(doc: &JsonDoc) {
    println!("{}", serde_json::to_string(doc).expect("serialize JSON"));
}

fn render_json(result: &CommandResult) {
    let doc = match result {
        CommandResult::Doctor(d) => ok_doc(DataView::Doctor {
            root: d.root.display().to_string(),
            checks: d.checks.iter().map(check_view).collect(),
        }),
        CommandResult::ProjectAdded(p) | CommandResult::ProjectShown(p) => {
            ok_doc(DataView::Project {
                project: project_view(p),
            })
        }
        CommandResult::ProjectList(projects) => ok_doc(DataView::ProjectList {
            projects: projects.iter().map(project_view).collect(),
        }),
        CommandResult::ArtifactCreated(a) | CommandResult::ArtifactShown(a) => {
            ok_doc(DataView::Artifact {
                artifact: artifact_view(a),
            })
        }
        CommandResult::ArtifactList(artifacts) => ok_doc(DataView::ArtifactList {
            artifacts: artifacts.iter().map(artifact_view).collect(),
        }),
        r => ok_doc(ws(parts(r).expect("doctor handled"))),
    };
    write_json(&doc);
}

fn ws((root, v, db, schema): (&Path, u32, bool, bool)) -> DataView {
    DataView::Workspace(WorkspaceView {
        root: root.display().to_string(),
        schema_version: v,
        database_ok: db,
        schema_ok: schema,
    })
}

fn check_view(c: &CheckResult) -> CheckView {
    CheckView {
        name: c.name,
        ok: c.ok,
        message: c.message.clone(),
    }
}

fn render_human(result: &CommandResult) {
    match result {
        CommandResult::Doctor(d) => {
            println!("AWC workspace at {}", d.root.display());
            for c in &d.checks {
                let status = if c.ok { "ok" } else { "failed" };
                println!("  {}: {status}", c.name);
                if !c.ok && !c.message.is_empty() {
                    println!("    {}", c.message);
                }
            }
        }
        CommandResult::ProjectAdded(p) => {
            println!("project added: {} ({})", p.slug, p.name);
            print_project(p);
        }
        CommandResult::ProjectShown(p) => {
            println!("project: {} ({})", p.slug, p.name);
            print_project(p);
        }
        CommandResult::ProjectList(projects) => {
            println!("projects ({}):", projects.len());
            for p in projects {
                println!("  - {} ({})", p.slug, p.name);
            }
        }
        CommandResult::ArtifactCreated(a) => {
            println!("artifact created: {} ({})", a.id.0, a.title);
            print_artifact(a);
        }
        CommandResult::ArtifactShown(a) => {
            println!("artifact: {} ({})", a.id.0, a.title);
            print_artifact(a);
        }
        CommandResult::ArtifactList(artifacts) => {
            println!("artifacts ({}):", artifacts.len());
            for a in artifacts {
                println!("  - {} [{}] {}", a.id.0, a.status.as_str(), a.title);
            }
        }
        r => {
            let (root, v, db, schema) = parts(r).expect("doctor handled");
            println!("AWC workspace at {}", root.display());
            println!("  schema version: {v}");
            println!("  database: {}", if db { "ok" } else { "unhealthy" });
            println!("  schema: {}", if schema { "ok" } else { "unhealthy" });
        }
    }
}

fn print_project(p: &Project) {
    println!("  id: {}", p.id.0);
    println!("  status: {}", p.status);
    if let Some(root) = &p.root_path {
        println!("  root: {}", root.display());
    }
}

fn print_artifact(a: &Artifact) {
    println!("  id: {}", a.id.0);
    println!("  project: {}", a.project_id.0);
    println!("  type: {}", a.artifact_type);
    println!("  status: {}", a.status.as_str());
    if let Some(path) = &a.path {
        println!("  path: {}", path.display());
    }
    if let Some(sha) = &a.sha256 {
        println!("  sha256: {sha}");
    }
    if let Some(size) = a.size {
        println!("  size: {size}");
    }
    println!("  created: {}", a.created_at);
}

fn render_error(json: bool, err: &AwcError) {
    if json {
        let (code, message) = match err {
            AwcError::Usage(msg) => ("usage", msg.clone()),
            AwcError::WorkspaceNotFound => ("workspace_not_found", err.to_string()),
            AwcError::UnsafeStatePath => ("unsafe_state_path", err.to_string()),
            AwcError::InvalidConfig(_) => ("invalid_config", err.to_string()),
            AwcError::UnsupportedConfigVersion(_) => {
                ("unsupported_config_version", err.to_string())
            }
            AwcError::Io(_) => ("io", err.to_string()),
            AwcError::Database(_) => ("database", err.to_string()),
            AwcError::ProjectNotFound => ("project_not_found", err.to_string()),
            AwcError::AmbiguousProjectId => ("ambiguous_project_id", err.to_string()),
            AwcError::SlugConflict(_) => ("slug_conflict", err.to_string()),
            AwcError::LegacySchemaData => ("legacy_schema_data", err.to_string()),
            AwcError::InvalidSlug(_) => ("invalid_slug", err.to_string()),
            AwcError::ArtifactNotFound => ("artifact_not_found", err.to_string()),
            AwcError::AmbiguousArtifactId => ("ambiguous_artifact_id", err.to_string()),
            AwcError::ArtifactStatusConflict(..) => ("artifact_status_conflict", err.to_string()),
            AwcError::PathOwned(_) => ("path_owned", err.to_string()),
            AwcError::ProtectedPath(_) => ("protected_path", err.to_string()),
            AwcError::PathEscape(_) => ("path_escape", err.to_string()),
            AwcError::MigrationConflict(_) => ("migration_conflict", err.to_string()),
            AwcError::AdoptPlanNotFound(_) => ("adopt_plan_not_found", err.to_string()),
            AwcError::StaleAdoptPlan(_) => ("stale_adopt_plan", err.to_string()),
            AwcError::RestoreConflict(_) => ("restore_conflict", err.to_string()),
            AwcError::DuplicateFingerprint(_) => ("duplicate_fingerprint", err.to_string()),
            AwcError::CompensationFailed(_) => ("compensation_failed", err.to_string()),
        };
        write_json(&JsonDoc {
            schema_version: 1,
            ok: false,
            data: None,
            error: Some(ErrorView { code, message }),
        });
    } else {
        eprintln!("awctl: {err}");
    }
}
