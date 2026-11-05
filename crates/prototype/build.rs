// ADR: The build stamps the short git commit into `PROTOTYPE_COMMIT` ("no commit" without git) and reruns when HEAD or the branch it names moves, so the corner and --version name the build.
use std::path::PathBuf;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn watch(path: PathBuf) {
    // A missing file would make cargo rerun the script on every build.
    if path.exists() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let commit =
        git(&["rev-parse", "--short=7", "HEAD"]).unwrap_or_else(|| "no commit".to_string());
    println!("cargo:rustc-env=PROTOTYPE_COMMIT={commit}");
    // A worktree keeps HEAD in its own git dir and branches in the common dir.
    if let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        watch(PathBuf::from(git_dir).join("HEAD"));
    }
    if let Some(common) = git(&["rev-parse", "--path-format=absolute", "--git-common-dir"]) {
        let common = PathBuf::from(common);
        watch(common.join("packed-refs"));
        if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"]) {
            watch(common.join(branch));
        }
    }
}
