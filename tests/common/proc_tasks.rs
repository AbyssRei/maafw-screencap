// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Test-only procfs evidence. Default/live reads are strict. Only callers AFTER
//! native stop/join/destruction may classify a known source task's retirement.
use std::ffi::{OsStr, OsString};
use std::io::Read;

#[cfg(target_os = "linux")]
const ESRCH: i32 = libc::ESRCH;
#[cfg(target_os = "linux")]
const ENOENT: i32 = libc::ENOENT;
// Portable controls describe Linux records; these are never Windows IO codes.
#[cfg(not(target_os = "linux"))]
const ESRCH: i32 = 3;
#[cfg(not(target_os = "linux"))]
const ENOENT: i32 = 2;

#[derive(Clone, Debug)]
pub(crate) struct Task {
    pub id: OsString,
    pub name: String,
    pub started: u64,
    pub flags: u64,
    pub address_space: u64,
    pub resident_pages: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    Open,
    Read,
    Parse,
}
#[derive(Clone, Debug)]
pub(crate) struct Fault {
    pub id: OsString,
    pub stage: Stage,
    pub native_error: Option<i32>,
    pub bytes: usize,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Absence {
    pub stat_opened: bool,
    pub stat_error: Option<i32>,
    pub close_error: Option<i32>,
    pub directory_exists: bool,
    pub directory_error: Option<i32>,
}
fn valid_id(id: &OsStr) -> bool {
    id.to_str().is_some_and(|s| {
        !s.is_empty()
            && s.bytes().all(|b| b.is_ascii_digit())
            && s.parse::<u32>().is_ok_and(|n| n != 0)
    })
}
fn known_live_source(task: &Task) -> bool {
    valid_id(&task.id)
        && task.name == "MaaScreencap"
        && task.started != 0
        && task.flags & 0x4 == 0
        && task.address_space != 0
}
fn exact_retired_read(before: &Task, failure: &Fault) -> bool {
    known_live_source(before)
        && failure.id == before.id
        && failure.stage == Stage::Read
        && failure.native_error == Some(ESRCH)
        && failure.bytes == 0
}
fn proved_absent(probe: &Absence) -> bool {
    !probe.stat_opened
        && probe.stat_error == Some(ENOENT)
        && probe.close_error.is_none()
        && !probe.directory_exists
        && probe.directory_error == Some(ENOENT)
}
fn accepted_retired_read(before: Option<&Task>, failure: &Fault, probe: &Absence) -> bool {
    before.is_some_and(|before| exact_retired_read(before, failure) && proved_absent(probe))
}
fn same_generation_retirement(before: &Task, after: &Task) -> bool {
    known_live_source(before)
        && after.id == before.id
        && after.name == before.name
        && after.started == before.started
        && after.flags & 0x4 != 0
        && after.address_space == 0
        && after.resident_pages == 0
}
fn parse(id: &OsStr, text: &str) -> Result<Task, Fault> {
    let invalid = || Fault {
        id: id.to_owned(),
        stage: Stage::Parse,
        native_error: None,
        bytes: text.len(),
    };
    if !valid_id(id) {
        return Err(invalid());
    }
    let begin = text.find('(').ok_or_else(invalid)?;
    let end = text.rfind(')').ok_or_else(invalid)?;
    if begin >= end || text[..begin].trim() != id.to_str().unwrap() {
        return Err(invalid());
    }
    let fields = text[end + 1..].split_whitespace().collect::<Vec<_>>();
    if fields.len() < 22 {
        return Err(invalid());
    }
    Ok(Task {
        id: id.to_owned(),
        name: text[begin + 1..end].to_owned(),
        started: fields[19].parse().map_err(|_| invalid())?,
        flags: fields[6].parse().map_err(|_| invalid())?,
        address_space: fields[20].parse().map_err(|_| invalid())?,
        resident_pages: fields[21].parse().map_err(|_| invalid())?,
    })
}

fn read_stream(id: &OsStr, input: &mut impl Read) -> Result<Task, Fault> {
    // Keep every byte delivered before an IO error, even if it is not UTF-8.
    // A zero-length String after decoding failure is not zero-byte evidence.
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes).map_err(|error| Fault {
        id: id.to_owned(),
        stage: Stage::Read,
        native_error: error.raw_os_error(),
        bytes: bytes.len(),
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|_| Fault {
        id: id.to_owned(),
        stage: Stage::Parse,
        native_error: None,
        bytes: bytes.len(),
    })?;
    parse(id, text)
}

#[cfg(target_os = "linux")]
#[path = "proc_tasks_linux.rs"]
mod native;
#[cfg(all(target_os = "linux", feature = "linux-pipewire"))]
pub(crate) use native::live_tasks;
#[cfg(target_os = "linux")]
pub(crate) use native::{only_retiring_tasks, task_ids};

#[cfg(test)]
mod tests {
    use super::*;
    fn before() -> Task {
        Task {
            id: "1234".into(),
            name: "MaaScreencap".into(),
            started: 9,
            flags: 0,
            address_space: 4096,
            resident_pages: 2,
        }
    }
    fn fault() -> Fault {
        Fault {
            id: "1234".into(),
            stage: Stage::Read,
            native_error: Some(ESRCH),
            bytes: 0,
        }
    }
    fn absent() -> Absence {
        Absence {
            stat_error: Some(ENOENT),
            directory_error: Some(ENOENT),
            ..Absence::default()
        }
    }
    #[test]
    fn refuses_unknown_owners_open_errors_partial_reads_and_unproved_absence() {
        let original = before();
        let failure = fault();
        let proof = absent();
        assert!(accepted_retired_read(Some(&original), &failure, &proof));
        assert!(!accepted_retired_read(None, &failure, &proof));
        for altered in [
            Task {
                name: "other".into(),
                ..before()
            },
            Task {
                started: 0,
                ..before()
            },
            Task {
                flags: 4,
                ..before()
            },
            Task {
                address_space: 0,
                ..before()
            },
            Task {
                id: "../1234".into(),
                ..before()
            },
            Task {
                id: "0".into(),
                ..before()
            },
        ] {
            assert!(!accepted_retired_read(Some(&altered), &failure, &proof));
        }
        for altered in [
            Fault {
                stage: Stage::Open,
                ..fault()
            },
            Fault {
                stage: Stage::Parse,
                ..fault()
            },
            Fault {
                native_error: Some(ENOENT),
                ..fault()
            },
            Fault {
                native_error: Some(5),
                ..fault()
            },
            Fault {
                native_error: None,
                ..fault()
            },
            Fault {
                bytes: 1,
                ..fault()
            },
            Fault {
                id: "1235".into(),
                ..fault()
            },
        ] {
            assert!(!accepted_retired_read(Some(&original), &altered, &proof));
        }
        for altered in [
            Absence {
                stat_opened: true,
                ..absent()
            },
            Absence {
                stat_error: Some(13),
                ..absent()
            },
            Absence {
                stat_error: Some(24),
                ..absent()
            },
            Absence {
                close_error: Some(5),
                ..absent()
            },
            Absence {
                directory_exists: true,
                ..absent()
            },
            Absence {
                directory_error: Some(13),
                ..absent()
            },
            Absence {
                directory_error: None,
                ..absent()
            },
        ] {
            assert!(!accepted_retired_read(Some(&original), &failure, &altered));
        }
    }
    #[test]
    fn refuses_live_or_substituted_task_generations() {
        let original = before();
        let exiting = Task {
            flags: 4,
            address_space: 0,
            resident_pages: 0,
            ..before()
        };
        assert!(same_generation_retirement(&original, &exiting));
        for altered in [
            before(),
            Task {
                started: 10,
                ..exiting.clone()
            },
            Task {
                id: "1235".into(),
                ..exiting.clone()
            },
            Task {
                name: "other".into(),
                ..exiting.clone()
            },
            Task {
                flags: 0,
                ..exiting.clone()
            },
            Task {
                address_space: 1,
                ..exiting.clone()
            },
            Task {
                resident_pages: 1,
                ..exiting.clone()
            },
        ] {
            assert!(!same_generation_retirement(&original, &altered));
        }
    }
    #[test]
    fn parses_parentheses_in_comm_but_rejects_missing_or_replaced_identity() {
        let mut fields = vec!["0"; 22];
        fields[0] = "S";
        fields[6] = "4";
        fields[19] = "9";
        let line = format!("1234 (name (with) parentheses) {}", fields.join(" "));
        let task = parse(OsStr::new("1234"), &line).unwrap();
        assert_eq!(
            (
                task.name.as_str(),
                task.started,
                task.flags,
                task.address_space,
                task.resident_pages
            ),
            ("name (with) parentheses", 9, 4, 0, 0)
        );
        for bad in [
            "",
            "1234 () S",
            &line.replace("1234 (", "1235 ("),
            &line.replace(" 9 ", " invalid "),
        ] {
            assert!(parse(OsStr::new("1234"), bad).is_err());
        }
    }
    #[test]
    fn preserves_partial_and_invalid_bytes_in_strict_read_failures() {
        struct Broken {
            delivered: bool,
            bytes: &'static [u8],
        }
        impl Read for Broken {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                if self.delivered {
                    return Err(std::io::Error::from_raw_os_error(ESRCH));
                }
                self.delivered = true;
                output[..self.bytes.len()].copy_from_slice(self.bytes);
                Ok(self.bytes.len())
            }
        }
        for bytes in [&b"1234"[..], &b"\xff"[..]] {
            let failure = read_stream(
                OsStr::new("1234"),
                &mut Broken {
                    delivered: false,
                    bytes,
                },
            )
            .unwrap_err();
            assert_eq!(failure.stage, Stage::Read);
            assert_eq!(failure.native_error, Some(ESRCH));
            assert_eq!(failure.bytes, bytes.len());
            assert!(!accepted_retired_read(Some(&before()), &failure, &absent()));
        }
        let failure = read_stream(OsStr::new("1234"), &mut &b"\xff"[..]).unwrap_err();
        assert_eq!(failure.stage, Stage::Parse);
        assert_eq!(failure.bytes, 1);
    }
}
#[cfg(all(test, target_os = "linux"))]
#[path = "proc_tasks_native.rs"]
mod native_fixture;
