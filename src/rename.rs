//! Renames media files on disk without ever overwriting or losing one.

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

/// Highest ` (n)` suffix tried before a taken name is reported as an error.
const MAX_COLLISION_SUFFIX: u32 = 9999;

#[derive(Debug, Clone)]
pub enum RenameOutcome {
    Renamed,
    DryRun,
    Unchanged,
    Skipped(String),
    Failed(String),
}

/// What [`apply`] did with one file.
#[derive(Debug, Clone)]
pub struct RenameReport {
    /// Where the file went, or would go in a dry run.
    pub target: PathBuf,
    /// The proposed name was taken, so the target got a ` (n)` suffix.
    pub collision_adjusted: bool,
    pub outcome: RenameOutcome,
}

/// Renames `original` to `new_name` in the same folder.
///
/// `used_targets` holds the targets already claimed by earlier files of the
/// batch, case-folded, so two files never get the same name, even on a
/// case-insensitive file system. A dry run resolves the target and touches
/// nothing.
pub fn apply(
    original: &Path,
    new_name: &str,
    used_targets: &mut HashSet<PathBuf>,
    dry_run: bool,
) -> RenameReport {
    let target = build_target_path(original, new_name);
    let (target, collision_adjusted) = match resolve_collision(original, &target, used_targets) {
        Ok(resolved) => resolved,
        Err(err) => {
            return RenameReport {
                target,
                collision_adjusted: false,
                outcome: RenameOutcome::Failed(err.to_string()),
            };
        }
    };
    used_targets.insert(fold_case(&target));

    let outcome = if target == original {
        RenameOutcome::Unchanged
    } else if dry_run {
        RenameOutcome::DryRun
    } else {
        match rename_file(original, &target) {
            Ok(()) => RenameOutcome::Renamed,
            Err(err) => RenameOutcome::Failed(err.to_string()),
        }
    };
    RenameReport {
        target,
        collision_adjusted,
        outcome,
    }
}

/// `new_name` next to `original`, keeping the original extension unless
/// `new_name` already ends with it.
pub fn build_target_path(original: &Path, new_name: &str) -> PathBuf {
    let parent = original.parent().unwrap_or_else(|| Path::new(""));
    let extension = original.extension().and_then(|ext| ext.to_str());
    let file_name = match extension {
        Some(ext) => {
            let ext_suffix = format!(".{ext}");
            if new_name
                .to_lowercase()
                .ends_with(&ext_suffix.to_lowercase())
            {
                new_name.to_string()
            } else {
                format!("{new_name}{ext_suffix}")
            }
        }
        None => new_name.to_string(),
    };
    parent.join(file_name)
}

/// The first free name among `target`, `target (1)`, `target (2)` and so on,
/// and whether a suffix was needed. A name is taken when another file has it
/// on disk or an earlier file of the batch claimed it in any letter case;
/// `original` itself does not count, so an already named file stays put.
pub fn resolve_collision(
    original: &Path,
    target: &Path,
    used_targets: &HashSet<PathBuf>,
) -> io::Result<(PathBuf, bool)> {
    resolve_collision_within(original, target, used_targets, MAX_COLLISION_SUFFIX)
}

fn resolve_collision_within(
    original: &Path,
    target: &Path,
    used_targets: &HashSet<PathBuf>,
    limit: u32,
) -> io::Result<(PathBuf, bool)> {
    let taken = |candidate: &Path| -> io::Result<bool> {
        Ok(used_targets.contains(&fold_case(candidate))
            || (occupied(candidate)? && !is_original(original, candidate)))
    };
    if !taken(target)? {
        return Ok((target.to_path_buf(), false));
    }

    let parent = target.parent().unwrap_or_else(|| Path::new(""));
    let stem = target
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let extension = target.extension().and_then(|value| value.to_str());

    for suffix in 1..=limit {
        let file_name = match extension {
            Some(ext) => format!("{stem} ({suffix}).{ext}"),
            None => format!("{stem} ({suffix})"),
        };
        let candidate = parent.join(file_name);
        if !taken(&candidate)? {
            return Ok((candidate, true));
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "{} and its {limit} numbered variants are all taken",
            target.display()
        ),
    ))
}

/// Moves `original` to `target`, refusing to replace another file.
///
/// A case-only rename on a case-insensitive file system goes through a
/// temporary name. A move across devices copies, syncs, then removes the
/// original; every other rename error is returned as is.
pub fn rename_file(original: &Path, target: &Path) -> io::Result<()> {
    if original == target {
        return Ok(());
    }
    if occupied(target)? {
        if is_original(original, target) {
            return rename_through_temp(original, target);
        }
        return Err(already_exists(target));
    }
    // ponytail: another process can still create `target` between the check
    // above and the rename; renameat2(RENAME_NOREPLACE) closes that gap on Linux.
    match fs::rename(original, target) {
        Err(err) if is_cross_device_link(&err) => copy_then_remove(original, target),
        result => result,
    }
}

/// EXDEV on Unix, ERROR_NOT_SAME_DEVICE on Windows: the only rename error a
/// copy can work around.
pub fn is_cross_device_link(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::CrossesDevices
}

/// `fs::rename` replaces an existing target on Unix, and some file systems
/// ignore a rename that only changes letter case, so hop through a free name.
fn rename_through_temp(original: &Path, target: &Path) -> io::Result<()> {
    let temp = free_temp_sibling(original)?;
    fs::rename(original, &temp)?;
    // The target name is free now, unless it belonged to another file.
    let moved = match occupied(target) {
        Ok(false) => fs::rename(&temp, target),
        Ok(true) => Err(already_exists(target)),
        Err(err) => Err(err),
    };
    moved.map_err(|err| match fs::rename(&temp, original) {
        Ok(()) => err,
        Err(restore) => io::Error::new(
            err.kind(),
            format!("{err}; the file was left at {} ({restore})", temp.display()),
        ),
    })
}

fn free_temp_sibling(path: &Path) -> io::Result<PathBuf> {
    let name = path.file_name().unwrap_or_default();
    for n in 1..=100 {
        let mut temp = OsString::from(".");
        temp.push(name);
        temp.push(format!(".kayabot-{n}.tmp"));
        let temp = path.with_file_name(temp);
        if !occupied(&temp)? {
            return Ok(temp);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("no free temporary name next to {}", path.display()),
    ))
}

fn copy_then_remove(original: &Path, target: &Path) -> io::Result<()> {
    let mut source = File::open(original)?;
    let mut copy = File::create_new(target)?;
    let copied = io::copy(&mut source, &mut copy).and_then(|_| {
        // fs::copy kept the permissions; keep doing so.
        if let Ok(metadata) = source.metadata() {
            let _ = copy.set_permissions(metadata.permissions());
        }
        copy.sync_all()
    });
    drop(copy);
    drop(source);
    if let Err(err) = copied {
        let _ = fs::remove_file(target);
        return Err(err);
    }
    fs::remove_file(original).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!(
                "copied to {} but could not remove the original: {err}",
                target.display()
            ),
        )
    })
}

fn already_exists(target: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("{} already exists", target.display()),
    )
}

/// Whether something, a dangling symlink included, has this name.
fn occupied(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

/// The batch key of a target: names that differ only in letter case are one
/// name on a case-insensitive file system.
fn fold_case(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().to_lowercase())
}

/// Whether `candidate` is `original` itself, maybe spelled in another letter
/// case. A hard link under another name is a different file.
fn is_original(original: &Path, candidate: &Path) -> bool {
    fold_case(original) == fold_case(candidate) && is_same_file(original, candidate)
}

/// Whether both paths name the same file, as two spellings that differ only
/// in case do on a case-insensitive file system.
#[cfg(unix)]
fn is_same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (fs::symlink_metadata(a), fs::symlink_metadata(b)) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

/// Std has no stable file id on Windows; the canonical path carries the
/// on-disk case, so two spellings of one file canonicalize alike.
#[cfg(windows)]
fn is_same_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[cfg(not(any(unix, windows)))]
fn is_same_file(_: &Path, _: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kayabot-rename-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .expect("read dir")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).expect("read file")
    }

    #[test]
    fn existing_target_stays_untouched() {
        let dir = temp_dir("no-clobber");
        let original = dir.join("a.mkv");
        let target = dir.join("b.mkv");
        fs::write(&original, "a").unwrap();
        fs::write(&target, "b").unwrap();

        let err = rename_file(&original, &target).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(read(&original), "a");
        assert_eq!(read(&target), "b");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn collision_gets_a_numbered_suffix() {
        let dir = temp_dir("collision");
        let original = dir.join("a.mkv");
        fs::write(&original, "a").unwrap();
        fs::write(dir.join("b.mkv"), "b").unwrap();

        let report = apply(&original, "b", &mut HashSet::new(), false);

        assert!(matches!(report.outcome, RenameOutcome::Renamed));
        assert!(report.collision_adjusted);
        assert_eq!(report.target, dir.join("b (1).mkv"));
        assert_eq!(names(&dir), ["b (1).mkv", "b.mkv"]);
        assert_eq!(read(&dir.join("b (1).mkv")), "a");
        assert_eq!(read(&dir.join("b.mkv")), "b");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_name_claimed_earlier_in_the_batch_is_taken() {
        let dir = temp_dir("batch");
        fs::write(dir.join("a.mkv"), "a").unwrap();
        fs::write(dir.join("c.mkv"), "c").unwrap();
        fs::write(dir.join("d.mkv"), "d").unwrap();
        let mut used = HashSet::new();

        let first = apply(&dir.join("a.mkv"), "Show", &mut used, true);
        let second = apply(&dir.join("c.mkv"), "Show", &mut used, true);
        // The same name in another case, as a case-insensitive file system sees it.
        let third = apply(&dir.join("d.mkv"), "show", &mut used, true);

        assert_eq!(first.target, dir.join("Show.mkv"));
        assert_eq!(second.target, dir.join("Show (1).mkv"));
        assert_eq!(third.target, dir.join("show (2).mkv"));
        assert!(third.collision_adjusted);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn exhausted_suffixes_are_an_error() {
        let dir = temp_dir("exhausted");
        let original = dir.join("a.mkv");
        for name in ["a.mkv", "b.mkv", "b (1).mkv", "b (2).mkv"] {
            fs::write(dir.join(name), name).unwrap();
        }

        let err = resolve_collision_within(&original, &dir.join("b.mkv"), &HashSet::new(), 2)
            .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn case_only_rename_changes_the_case() {
        let dir = temp_dir("case-only");
        let original = dir.join("arcane.mkv");
        fs::write(&original, "data").unwrap();

        let report = apply(&original, "Arcane", &mut HashSet::new(), false);

        assert!(
            matches!(report.outcome, RenameOutcome::Renamed),
            "{report:?}"
        );
        assert!(!report.collision_adjusted);
        assert_eq!(names(&dir), ["Arcane.mkv"]);
        assert_eq!(read(&dir.join("Arcane.mkv")), "data");

        // The hop itself, which only case-insensitive file systems reach above.
        rename_through_temp(&dir.join("Arcane.mkv"), &dir.join("arcane.mkv")).unwrap();
        assert_eq!(names(&dir), ["arcane.mkv"]);
        assert_eq!(read(&dir.join("arcane.mkv")), "data");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_hard_link_is_another_file() {
        let dir = temp_dir("hard-link");
        let original = dir.join("a.mkv");
        fs::write(&original, "data").unwrap();
        fs::hard_link(&original, dir.join("b.mkv")).unwrap();

        let report = apply(&original, "b", &mut HashSet::new(), false);

        assert!(
            matches!(report.outcome, RenameOutcome::Renamed),
            "{report:?}"
        );
        assert!(report.collision_adjusted);
        assert_eq!(names(&dir), ["b (1).mkv", "b.mkv"]);
        assert_eq!(read(&dir.join("b (1).mkv")), "data");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_hard_link_in_another_case_is_not_replaced() {
        let dir = temp_dir("hard-link-case");
        let original = dir.join("a.mkv");
        let link = dir.join("A.mkv");
        fs::write(&original, "data").unwrap();
        if fs::hard_link(&original, &link).is_err() {
            // A case-insensitive file system has no room for both names.
            fs::remove_dir_all(&dir).unwrap();
            return;
        }

        // Same file under the folded name, so this reaches the hop, which must
        // find the target still taken and put the file back.
        let err = rename_file(&original, &link).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(names(&dir), ["A.mkv", "a.mkv"]);
        assert_eq!(read(&original), "data");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn already_named_file_is_unchanged() {
        let dir = temp_dir("unchanged");
        let original = dir.join("Arcane.mkv");
        fs::write(&original, "data").unwrap();

        let report = apply(&original, "Arcane", &mut HashSet::new(), false);

        assert!(matches!(report.outcome, RenameOutcome::Unchanged));
        assert!(!report.collision_adjusted);
        assert_eq!(names(&dir), ["Arcane.mkv"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn dry_run_touches_nothing() {
        let dir = temp_dir("dry-run");
        let name = "The Office US - S02E03 - Office Olympics.mkv";
        let original = dir.join(name);
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/media_samples")
                .join(name),
            &original,
        )
        .expect("fixture copy");

        let report = apply(
            &original,
            "The Office (US) - 2x03 - Office Olympics",
            &mut HashSet::new(),
            true,
        );

        assert!(matches!(report.outcome, RenameOutcome::DryRun));
        assert_eq!(
            report.target,
            dir.join("The Office (US) - 2x03 - Office Olympics.mkv")
        );
        assert_eq!(names(&dir), [name]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn copy_fallback_moves_the_file_and_never_overwrites() {
        let dir = temp_dir("copy");
        let original = dir.join("a.mkv");
        let taken = dir.join("b.mkv");
        fs::write(&original, "a").unwrap();
        fs::write(&taken, "b").unwrap();

        let err = copy_then_remove(&original, &taken).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(read(&original), "a");
        assert_eq!(read(&taken), "b");

        copy_then_remove(&original, &dir.join("c.mkv")).unwrap();
        assert_eq!(names(&dir), ["b.mkv", "c.mkv"]);
        assert_eq!(read(&dir.join("c.mkv")), "a");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_cross_device_errors_take_the_copy_fallback() {
        #[cfg(unix)]
        assert!(is_cross_device_link(&io::Error::from_raw_os_error(18)));
        #[cfg(windows)]
        assert!(is_cross_device_link(&io::Error::from_raw_os_error(17)));
        assert!(!is_cross_device_link(&io::Error::from(
            io::ErrorKind::PermissionDenied
        )));
    }
}
