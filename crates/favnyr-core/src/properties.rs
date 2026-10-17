//! Facts about a single filesystem item, for the properties panel.
//!
//! Reading is deliberately separated from presentation: this module answers
//! "what is true of this path" and never formats, translates or decides what
//! to show. Everything it returns is optional where the platform or the
//! filesystem may not provide it, so a missing birth time or an unreadable
//! owner degrades one line instead of failing the whole panel.

use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What the path itself is, before any symlink is followed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    File,
    Directory,
    /// A symbolic link, whatever it points at. `link_target` carries the
    /// destination and `target_kind` what that destination turned out to be.
    Symlink,
    /// A socket, fifo, device node: nothing the panel can describe further.
    Other,
}

/// Permission of one class of users, as the panel offers it.
///
/// The three states cover every combination the two bits can express except
/// write-without-read, which is legal and occasionally deliberate. That case
/// is reported as `Custom` rather than rounded to a neighbour, so the panel
/// can show it without silently rewriting it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Neither read nor write.
    Forbidden,
    /// Read only.
    ReadOnly,
    /// Read and write.
    ReadWrite,
    /// Write without read.
    Custom,
}

impl Access {
    fn from_bits(read: bool, write: bool) -> Self {
        match (read, write) {
            (false, false) => Self::Forbidden,
            (true, false) => Self::ReadOnly,
            (true, true) => Self::ReadWrite,
            (false, true) => Self::Custom,
        }
    }

    /// The (read, write) pair to write back, or `None` for `Custom`, which
    /// exists to be displayed and never to be applied.
    ///
    /// Only the Unix writer consults it; elsewhere permissions are read and
    /// shown, never set.
    #[cfg(unix)]
    fn to_bits(self) -> Option<(bool, bool)> {
        match self {
            Self::Forbidden => Some((false, false)),
            Self::ReadOnly => Some((true, false)),
            Self::ReadWrite => Some((true, true)),
            Self::Custom => None,
        }
    }
}

/// Which set of permission bits an operation addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessClass {
    Owner,
    Group,
    Others,
}

impl AccessClass {
    /// Bit offset of this class inside the low nine permission bits.
    fn shift(self) -> u32 {
        match self {
            Self::Owner => 6,
            Self::Group => 3,
            Self::Others => 0,
        }
    }
}

/// Unix ownership and permissions. Always `None` on platforms that have no
/// such notion, so the structure keeps one shape across targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnixFacts {
    /// The permission bits only, including setuid, setgid and sticky.
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

impl UnixFacts {
    /// Permission of one class, derived from the bits.
    pub fn access(&self, class: AccessClass) -> Access {
        let shift = class.shift();
        Access::from_bits(
            self.mode & (0o4 << shift) != 0,
            self.mode & (0o2 << shift) != 0,
        )
    }

    /// Whether the owner may execute the item. Directories use the same bit
    /// for traversal, which is why the panel labels it per kind.
    pub fn owner_executable(&self) -> bool {
        self.mode & 0o100 != 0
    }

    /// Login name of the owner, read on demand.
    ///
    /// Deliberately a method and not a field: answering it opens the system
    /// account database, so nothing is read for a panel that never shows a
    /// name. `None` means no entry matched, and the caller shows the number.
    #[cfg(unix)]
    pub fn owner_name(&self) -> Option<String> {
        lookup_name("/etc/passwd", self.uid)
    }

    /// Group name, read on demand, under the same rule as `owner_name`.
    #[cfg(unix)]
    pub fn group_name(&self) -> Option<String> {
        lookup_name("/etc/group", self.gid)
    }
}

/// Everything the panel reads about one item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemProperties {
    /// File name alone. Empty only for a filesystem root.
    pub name: String,
    /// Directory holding the item, which is what the panel shows as location.
    pub parent: PathBuf,
    pub kind: ItemKind,
    /// Destination of a symlink, unresolved, exactly as recorded on disk.
    pub link_target: Option<PathBuf>,
    /// What the destination of a symlink turned out to be, `None` when it
    /// cannot be reached: a broken link is a fact worth showing.
    pub target_kind: Option<ItemKind>,
    /// Apparent size in bytes. For a directory this is the size of the
    /// directory entry itself, never the total of its contents: computing that
    /// is a walk, which belongs to the caller and its existing machinery.
    pub size: u64,
    /// Bytes actually occupied, from the allocated block count. Absent on
    /// platforms that do not report blocks, and differs from `size` for sparse
    /// or compressed files as well as for the usual block rounding.
    pub size_on_disk: Option<u64>,
    /// Absent where the filesystem keeps no birth time.
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    /// Unix ownership and permissions, `None` elsewhere.
    pub unix: Option<UnixFacts>,
}

impl ItemProperties {
    /// Reads everything available for `path`.
    ///
    /// The item is examined without following a final symlink, so a link
    /// reports its own dates and permissions rather than its destination's,
    /// which is what a properties panel is expected to show.
    pub fn read(path: &Path) -> std::io::Result<Self> {
        let meta = std::fs::symlink_metadata(path)?;
        let kind = kind_of(&meta);

        let (link_target, target_kind) = if kind == ItemKind::Symlink {
            let target = std::fs::read_link(path).ok();
            let resolved = std::fs::metadata(path).ok().map(|m| kind_of(&m));
            (target, resolved)
        } else {
            (None, None)
        };

        Ok(Self {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            parent: path.parent().unwrap_or(path).to_path_buf(),
            kind,
            link_target,
            target_kind,
            size: meta.len(),
            size_on_disk: size_on_disk(&meta),
            // `created` fails rather than returning `None` on filesystems with
            // no birth time, so the error is flattened away here.
            created: meta.created().ok(),
            modified: meta.modified().ok(),
            accessed: meta.accessed().ok(),
            unix: unix_facts(&meta),
        })
    }
}

fn kind_of(meta: &Metadata) -> ItemKind {
    let t = meta.file_type();
    if t.is_symlink() {
        ItemKind::Symlink
    } else if t.is_dir() {
        ItemKind::Directory
    } else if t.is_file() {
        ItemKind::File
    } else {
        ItemKind::Other
    }
}

#[cfg(unix)]
fn size_on_disk(meta: &Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    // `blocks` counts 512-byte units by definition, whatever the filesystem
    // block size, which is what `du` reports.
    Some(meta.blocks().saturating_mul(512))
}

#[cfg(not(unix))]
fn size_on_disk(_meta: &Metadata) -> Option<u64> {
    None
}

#[cfg(unix)]
fn unix_facts(meta: &Metadata) -> Option<UnixFacts> {
    use std::os::unix::fs::MetadataExt;
    let uid = meta.uid();
    let gid = meta.gid();
    Some(UnixFacts {
        // The file type is also encoded in `mode`; only the permission bits
        // concern the panel, setuid, setgid and sticky included.
        mode: meta.mode() & 0o7777,
        uid,
        gid,
    })
}

#[cfg(not(unix))]
fn unix_facts(_meta: &Metadata) -> Option<UnixFacts> {
    None
}

/// Name matching a numeric id in an account database.
///
/// `/etc/passwd` and `/etc/group` share the same shape: colon-separated
/// fields, the login name first and the numeric id third. Despite its name,
/// `/etc/passwd` has held no password since the shadow suite moved them to
/// `/etc/shadow`; it is the world-readable account directory every tool
/// consults to print a name instead of a number, and it is read here for the
/// same reason. Only the first field is taken. The fifth, which may carry a
/// real name or a telephone number, is never looked at, nothing is retained
/// and nothing is logged.
///
/// Reading it directly avoids a dependency for two labels. A system whose
/// accounts live in a directory service yields `None`, and the caller then
/// shows the number.
#[cfg(unix)]
fn lookup_name(database: &str, id: u32) -> Option<String> {
    let content = std::fs::read_to_string(database).ok()?;
    find_name(&content, id)
}

/// Split out from the reader so the parsing can be tested without touching
/// the machine's real account databases.
#[cfg(unix)]
fn find_name(content: &str, id: u32) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split(':');
        let name = fields.next()?;
        let _password = fields.next();
        let Some(found) = fields.next().and_then(|v| v.parse::<u32>().ok()) else {
            continue;
        };
        if found == id && !name.is_empty() {
            return Some(name.to_owned());
        }
    }
    None
}

/// Applies one class's read and write bits, leaving everything else alone.
///
/// Reading the current mode first is what preserves the execute bits and the
/// setuid, setgid and sticky flags: writing a whole mode computed from three
/// dropdowns would silently drop them.
#[cfg(unix)]
pub fn set_access(path: &Path, class: AccessClass, access: Access) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let Some((read, write)) = access.to_bits() else {
        // `Custom` describes a mode the panel read; it is never a request.
        return Ok(());
    };
    let shift = class.shift();
    let mut mode = std::fs::symlink_metadata(path)?.permissions().mode() & 0o7777;
    mode &= !((0o4 | 0o2) << shift);
    if read {
        mode |= 0o4 << shift;
    }
    if write {
        mode |= 0o2 << shift;
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

/// Turns the execute bit on or off for all three classes at once.
///
/// One checkbox drives the three bits because that is how the item behaves:
/// an executable readable by everyone but runnable by its owner alone is a
/// distinction the advanced view exists for, not this one.
#[cfg(unix)]
pub fn set_executable(path: &Path, executable: bool) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut mode = std::fs::symlink_metadata(path)?.permissions().mode() & 0o7777;
    if executable {
        mode |= 0o111;
    } else {
        mode &= !0o111;
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A scratch directory of our own, removed on drop, so the tests never
    /// depend on the machine they run on.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("favnyr-properties-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("scratch directory");
            Self(dir)
        }

        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            let mut f = std::fs::File::create(&path).expect("create");
            f.write_all(bytes).expect("write");
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_file_reports_its_name_location_and_size() {
        let scratch = Scratch::new("basics");
        let path = scratch.file("my_file.txt", b"0123456789");

        let p = ItemProperties::read(&path).expect("read");
        assert_eq!(p.name, "my_file.txt");
        assert_eq!(p.parent, scratch.0);
        assert_eq!(p.kind, ItemKind::File);
        assert_eq!(p.size, 10);
        assert!(p.modified.is_some());
        assert!(p.link_target.is_none());
    }

    #[test]
    fn a_directory_is_reported_as_one() {
        let scratch = Scratch::new("dir");
        let path = scratch.0.join("folder01");
        std::fs::create_dir(&path).expect("create dir");

        let p = ItemProperties::read(&path).expect("read");
        assert_eq!(p.kind, ItemKind::Directory);
        assert_eq!(p.name, "folder01");
    }

    #[test]
    fn reading_a_missing_path_is_an_error_not_a_panic() {
        let scratch = Scratch::new("missing");
        assert!(ItemProperties::read(&scratch.0.join("absent")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_describes_itself_and_its_destination() {
        let scratch = Scratch::new("link");
        let target = scratch.file("my_file.txt", b"abc");
        let link = scratch.0.join("my_link");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        let p = ItemProperties::read(&link).expect("read");
        // The link itself, not what it points at: a panel that followed it
        // would show the destination's dates under the link's name.
        assert_eq!(p.kind, ItemKind::Symlink);
        assert_eq!(p.link_target.as_deref(), Some(target.as_path()));
        assert_eq!(p.target_kind, Some(ItemKind::File));
    }

    #[cfg(unix)]
    #[test]
    fn a_broken_symlink_still_reads() {
        let scratch = Scratch::new("broken");
        let link = scratch.0.join("my_link");
        std::os::unix::fs::symlink(scratch.0.join("absent"), &link).expect("symlink");

        let p = ItemProperties::read(&link).expect("read");
        assert_eq!(p.kind, ItemKind::Symlink);
        assert!(p.link_target.is_some());
        // Unreachable destination, which is exactly what the panel must show.
        assert_eq!(p.target_kind, None);
    }

    #[cfg(unix)]
    #[test]
    fn size_on_disk_is_reported_and_rounds_up_to_blocks() {
        let scratch = Scratch::new("blocks");
        let path = scratch.file("my_file.txt", b"x");

        let p = ItemProperties::read(&path).expect("read");
        let on_disk = p.size_on_disk.expect("blocks reported on unix");
        assert_eq!(p.size, 1);
        // One byte never occupies one byte; the figure is a multiple of 512.
        assert!(on_disk >= 512, "one byte occupied {on_disk}");
        assert_eq!(on_disk % 512, 0);
    }

    #[cfg(unix)]
    #[test]
    fn the_three_classes_are_derived_from_the_mode() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = Scratch::new("modes");
        let path = scratch.file("my_file.txt", b"");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).expect("set mode");

        let unix = ItemProperties::read(&path)
            .expect("read")
            .unix
            .expect("unix facts");
        assert_eq!(unix.access(AccessClass::Owner), Access::ReadWrite);
        assert_eq!(unix.access(AccessClass::Group), Access::ReadOnly);
        assert_eq!(unix.access(AccessClass::Others), Access::Forbidden);
        assert!(!unix.owner_executable());
    }

    #[cfg(unix)]
    #[test]
    fn write_without_read_is_reported_rather_than_rounded() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = Scratch::new("custom");
        let path = scratch.file("my_file.txt", b"");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600 | 0o020))
            .expect("set mode");

        let unix = ItemProperties::read(&path)
            .expect("read")
            .unix
            .expect("unix facts");
        assert_eq!(unix.access(AccessClass::Group), Access::Custom);
        // And it is never applied back, so the odd mode survives the panel.
        assert!(set_access(&path, AccessClass::Group, Access::Custom).is_ok());
        let after = ItemProperties::read(&path)
            .expect("read")
            .unix
            .expect("unix facts");
        assert_eq!(after.mode & 0o070, 0o020);
    }

    #[cfg(unix)]
    #[test]
    fn setting_one_class_leaves_the_others_and_the_special_bits_alone() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = Scratch::new("preserve");
        let path = scratch.file("my_file.txt", b"");
        // Execute for everyone, plus setuid: the bits a naive rewrite loses.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o4755)).expect("set mode");

        set_access(&path, AccessClass::Others, Access::Forbidden).expect("set access");

        let unix = ItemProperties::read(&path)
            .expect("read")
            .unix
            .expect("unix facts");
        assert_eq!(unix.access(AccessClass::Others), Access::Forbidden);
        assert_eq!(unix.access(AccessClass::Owner), Access::ReadWrite);
        assert_eq!(unix.access(AccessClass::Group), Access::ReadOnly);
        assert!(unix.owner_executable(), "execute bits must survive");
        assert_eq!(unix.mode & 0o4000, 0o4000, "setuid must survive");
    }

    #[cfg(unix)]
    #[test]
    fn the_execute_flag_moves_all_three_bits_and_keeps_the_rest() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = Scratch::new("exec");
        let path = scratch.file("my_file.txt", b"");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o2640)).expect("set mode");

        set_executable(&path, true).expect("set executable");
        let unix = ItemProperties::read(&path)
            .expect("read")
            .unix
            .expect("unix facts");
        assert_eq!(unix.mode & 0o111, 0o111);
        assert_eq!(unix.mode & 0o666, 0o640, "read and write must survive");
        assert_eq!(unix.mode & 0o2000, 0o2000, "setgid must survive");

        set_executable(&path, false).expect("clear executable");
        let unix = ItemProperties::read(&path)
            .expect("read")
            .unix
            .expect("unix facts");
        assert_eq!(unix.mode & 0o111, 0);
        assert_eq!(unix.mode & 0o666, 0o640);
    }

    #[cfg(unix)]
    #[test]
    fn an_account_database_is_parsed_by_numeric_id() {
        // Shaped like the real files, with a comment and a blank line, and
        // deliberately generic names.
        let passwd = "\
# an account database
root:x:0:0:root:/root:/bin/sh

daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin
account01:x:1000:1000::/home/account01:/bin/sh
";
        assert_eq!(find_name(passwd, 0).as_deref(), Some("root"));
        assert_eq!(find_name(passwd, 1000).as_deref(), Some("account01"));
        // No entry means the panel shows the number instead.
        assert_eq!(find_name(passwd, 4242), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_malformed_account_line_is_skipped_not_fatal() {
        let passwd = "broken-line-without-fields\nname01:x:7:7::/home/name01:/bin/sh\n";
        assert_eq!(find_name(passwd, 7).as_deref(), Some("name01"));
    }
}
