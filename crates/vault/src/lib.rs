use std::path::{Component, Path};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile { ReadOnly, Dev, Ci }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision { Allow, Deny }
pub fn authorize(profile: Profile, command: &str, workspace: &Path, cwd: &Path) -> Decision {
    if !cwd.starts_with(workspace) { return Decision::Deny; }
    let executable = command.split_whitespace().next().unwrap_or("");
    if executable.is_empty() || command.contains("..") || command.contains("$(") || command.contains('`') { return Decision::Deny; }
    match profile { Profile::ReadOnly => if ["cat", "ls", "pwd", "find", "rg", "git"].contains(&executable) { Decision::Allow } else { Decision::Deny }, Profile::Dev => if ["cargo", "git", "python", "python3", "npm", "node", "rustc", "ls", "cat", "rg"].contains(&executable) { Decision::Allow } else { Decision::Deny }, Profile::Ci => if ["cargo", "npm", "python", "python3", "git"].contains(&executable) { Decision::Allow } else { Decision::Deny } }
}
pub fn safe_relative(path: &Path) -> bool { !path.is_absolute() && !path.components().any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))) }
#[cfg(test)]
mod tests { use super::*; #[test] fn denies_escape_and_shell_evaluation() { let root = Path::new("/workspace"); assert_eq!(authorize(Profile::Dev, "cat ../secret", root, root), Decision::Deny); assert_eq!(authorize(Profile::Dev, "cat $(id)", root, root), Decision::Deny); } #[test] fn read_only_is_narrow() { let root = Path::new("/workspace"); assert_eq!(authorize(Profile::ReadOnly, "cat file", root, root), Decision::Allow); assert_eq!(authorize(Profile::ReadOnly, "rm file", root, root), Decision::Deny); } }
