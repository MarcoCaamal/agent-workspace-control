//! `awctl` — AWC command-line boundary: parsing, deterministic JSON/human
//! rendering, and exit codes (0 success, 1 operational, 2 usage, 3 not-found).

use std::path::{Path, PathBuf};

use awc_core::{
    application,
    domain::{AddProject, CheckResult, CommandResult, Project},
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
