//! Deterministic, metadata-only classifier for adopt scan (design: Adopt
//! classification). Signals are location, filename, extension, and size —
//! never file contents.

use std::path::Path;

use crate::domain::ScanCategory;

/// Suggested action for a classified candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestedAction {
    /// Register as an artifact with the given type.
    Register,
    /// Move to `inbox/` (conservative sink for unknowns).
    MoveToInbox,
    /// Do nothing.
    Skip,
}

/// Classifies a workspace-relative path using deterministic signals only.
///
/// Order matters: sensitive and runtime classes win over generic patterns;
/// ignored paths are excluded by the caller before classification.
pub fn classify(rel: &Path) -> (ScanCategory, SuggestedAction) {
    let name = rel
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let ext = rel
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // Sensitive candidates: never registered, moved, or read.
    if is_sensitive(rel, &name, &ext) {
        return (ScanCategory::SensitiveCandidate, SuggestedAction::Skip);
    }
    // Known runtime files: recognized, never touched.
    if is_known_runtime(rel, &name) {
        return (ScanCategory::KnownRuntime, SuggestedAction::Skip);
    }
    // Temporary candidates: reported without proposed registration.
    if is_temporary(&name, &ext) {
        return (ScanCategory::TemporaryCandidate, SuggestedAction::Skip);
    }
    // Managed candidates by filename/extension pattern.
    if name.contains("plan") && matches!(ext.as_str(), "md" | "txt") {
        return (ScanCategory::ManagedCandidate, SuggestedAction::Register);
    }
    if (name.contains("review") || name.starts_with("pr-")) && matches!(ext.as_str(), "md" | "txt")
    {
        return (ScanCategory::ManagedCandidate, SuggestedAction::Register);
    }
    if name.contains("report") && matches!(ext.as_str(), "md" | "txt") {
        return (ScanCategory::ManagedCandidate, SuggestedAction::Register);
    }
    // Unknown: conservative sink.
    (ScanCategory::Unknown, SuggestedAction::MoveToInbox)
}

fn is_sensitive(rel: &Path, name: &str, ext: &str) -> bool {
    if name.starts_with(".env") {
        return true;
    }
    if matches!(ext, "pem" | "key") {
        return true;
    }
    if rel.starts_with(".ssh") {
        return true;
    }
    if name.contains("secret") || name.contains("credential") || name.contains("password") {
        return true;
    }
    name.contains("private_key") || name.contains("id_rsa") || name.contains("id_ed25519")
}

fn is_known_runtime(rel: &Path, name: &str) -> bool {
    matches!(name, "agents.md" | "soul.md" | "memory.md")
        || rel.starts_with("memory")
        || rel.starts_with("skills")
}

fn is_temporary(name: &str, ext: &str) -> bool {
    matches!(ext, "tmp" | "bak") || name.starts_with('~') || name.ends_with('~')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ScanCategory::*;

    fn c(path: &str) -> (ScanCategory, SuggestedAction) {
        classify(Path::new(path))
    }

    #[test]
    fn classifies_managed_patterns() {
        assert_eq!(
            c("adopt-plan.md"),
            (ManagedCandidate, SuggestedAction::Register)
        );
        assert_eq!(
            c("review-pr-13.md"),
            (ManagedCandidate, SuggestedAction::Register)
        );
        assert_eq!(
            c("pr-review.txt"),
            (ManagedCandidate, SuggestedAction::Register)
        );
        assert_eq!(
            c("q3-report.md"),
            (ManagedCandidate, SuggestedAction::Register)
        );
    }

    #[test]
    fn classifies_temporary_candidates() {
        assert_eq!(c("backup.tmp"), (TemporaryCandidate, SuggestedAction::Skip));
        assert_eq!(c("notes.bak"), (TemporaryCandidate, SuggestedAction::Skip));
        assert_eq!(c("~draft.md"), (TemporaryCandidate, SuggestedAction::Skip));
        assert_eq!(c("draft.md~"), (TemporaryCandidate, SuggestedAction::Skip));
    }

    #[test]
    fn classifies_sensitive_candidates_as_skip() {
        for path in [
            ".env",
            ".env.production",
            "cert.pem",
            "deploy.key",
            "api-secret.txt",
            "credentials.json",
            ".ssh/config",
        ] {
            assert_eq!(
                c(path),
                (SensitiveCandidate, SuggestedAction::Skip),
                "{path}"
            );
        }
    }

    #[test]
    fn recognizes_runtime_files_without_mutation() {
        for path in [
            "AGENTS.md",
            "SOUL.md",
            "MEMORY.md",
            "memory/notes.md",
            "skills/x.md",
        ] {
            assert_eq!(c(path), (KnownRuntime, SuggestedAction::Skip), "{path}");
        }
    }

    #[test]
    fn unknown_goes_to_inbox() {
        assert_eq!(c("notes-old.md"), (Unknown, SuggestedAction::MoveToInbox));
        assert_eq!(c("random.txt"), (Unknown, SuggestedAction::MoveToInbox));
    }

    #[test]
    fn non_markdown_managed_names_are_unknown() {
        assert_eq!(c("plan.pdf"), (Unknown, SuggestedAction::MoveToInbox));
        assert_eq!(c("report.xlsx"), (Unknown, SuggestedAction::MoveToInbox));
    }
}
