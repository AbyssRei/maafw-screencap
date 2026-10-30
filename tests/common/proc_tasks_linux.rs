// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::{
    Absence, Fault, Stage, Task, accepted_retired_read, exact_retired_read, known_live_source,
    parse, read_stream, same_generation_retirement, valid_id,
};
use std::{
    ffi::{OsStr, OsString},
    fs::{self, File},
    os::fd::IntoRawFd,
    path::Path,
};

pub(crate) fn task_ids() -> Vec<OsString> {
    let mut ids = fs::read_dir("/proc/self/task")
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<Vec<_>>();
    ids.sort();
    ids
}
pub(super) fn read_opened(id: &OsStr, file: &mut File) -> Result<Task, Fault> {
    read_stream(id, file)
}
pub(super) fn read_task(id: &OsStr) -> Result<Option<Task>, Fault> {
    if !valid_id(id) {
        return parse(id, "").map(Some);
    }
    let mut file = match File::open(Path::new("/proc/self/task").join(id).join("stat")) {
        Ok(file) => file,
        Err(error) if error.raw_os_error() == Some(libc::ENOENT) => return Ok(None),
        Err(error) => {
            return Err(Fault {
                id: id.to_owned(),
                stage: Stage::Open,
                native_error: error.raw_os_error(),
                bytes: 0,
            });
        }
    };
    read_opened(id, &mut file).map(Some)
}
pub(crate) fn live_tasks() -> Vec<Task> {
    task_ids()
        .iter()
        .filter_map(|id| {
            read_task(id).unwrap_or_else(|e| panic!("cannot inspect live task {id:?}: {e:?}"))
        })
        .collect()
}
pub(super) fn close_file(file: File) -> Option<i32> {
    let fd = file.into_raw_fd();
    // SAFETY: This exact diagnostic File owner is detached above and closed
    // once. No other owner or numeric reuse intervenes; never retry close.
    if unsafe { libc::close(fd) } == 0 {
        None
    } else {
        Some(std::io::Error::last_os_error().raw_os_error().unwrap_or(-1))
    }
}
pub(super) fn probe_absence(id: &OsStr) -> Absence {
    if !valid_id(id) {
        return Absence {
            stat_error: Some(libc::EINVAL),
            ..Absence::default()
        };
    }
    let directory = Path::new("/proc/self/task").join(id);
    match File::open(directory.join("stat")) {
        Ok(file) => Absence {
            stat_opened: true,
            close_error: close_file(file),
            ..Absence::default()
        },
        Err(error) => {
            let mut result = Absence {
                stat_error: error.raw_os_error(),
                ..Absence::default()
            };
            if result.stat_error == Some(libc::ENOENT) {
                match fs::metadata(directory) {
                    Ok(_) => result.directory_exists = true,
                    Err(e) => result.directory_error = e.raw_os_error(),
                }
            }
            result
        }
    }
}
pub(crate) fn only_retiring_tasks(
    actual: &[OsString],
    baseline: &[OsString],
    live: &[Task],
) -> bool {
    // PRECONDITION: Native stop/join/destruction already completed. This never
    // waits for callbacks, active workers, asynchronous cleanup or descriptors.
    baseline.iter().all(|id| actual.contains(id))
        && actual.iter().filter(|id| !baseline.contains(id)).all(|id| {
            let Some(before) = live.iter().find(|t| t.id == *id) else {
                return false;
            };
            if !known_live_source(before) {
                return false;
            }
            match read_task(id) {
                Ok(None) => true,
                Ok(Some(after)) => same_generation_retirement(before, &after),
                Err(error) => {
                    if !exact_retired_read(before, &error) {
                        eprintln!("post_join_task_read rejected={error:?}");
                        return false;
                    }
                    // Two fresh observations classify the failed read; neither is
                    // a retry of capture, native cleanup or the failed stream read.
                    let probe = probe_absence(id);
                    let accepted = accepted_retired_read(Some(before), &error, &probe);
                    eprintln!("post_join_task_read id={id:?} started={} failure={error:?} fresh={probe:?} proved_absent={accepted}", before.started);
                    accepted
                }
            }
        })
}
