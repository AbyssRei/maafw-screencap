// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Preserve the native compiler's view of the ordinary, unchanged F class.
//! JSON AST is compiler-version-specific observation, not a semantic census.
use crate::{Result, records};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Stdio},
};

const SOURCE: &str = "source/MaaMacOSControlUnit/Screencap/ScreenCaptureKitScreencap.mm";
const HEADER: &str = "source/MaaMacOSControlUnit/Screencap/ScreenCaptureKitScreencap.h";
const SOURCE_HASH: &str = "fd0d5e4a0a960abf25b7908fe9a779a0c8a319291b81cd90b7f91f23fe4e9643";
const HEADER_HASH: &str = "a0333c763c586ecaeabfeb3df7ff90b030290607905a04c22a9c8ca28e0574fb";

#[derive(Deserialize)]
struct Entry {
    directory: String,
    file: String,
    command: Option<String>,
    arguments: Option<Vec<String>>,
    output: Option<String>,
}

// CMake's POSIX command spelling only. No shell executes this input; refusing
// expansions/operators avoids silently changing the recorded command's meaning.
fn words(command: &str) -> Result<Vec<String>> {
    if command.contains(['\0', '\r', '\n']) {
        return Err("multiline/NUL compiler command is unsupported".into());
    }
    let mut result = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut quote = None;
    let mut chars = command.chars();
    while let Some(ch) = chars.next() {
        match (quote, ch) {
            (Some('\''), '\'') | (Some('"'), '"') => quote = None,
            (Some('\''), _) => word.push(ch),
            (_, '\\') => {
                let next = chars.next().ok_or("trailing compiler-command escape")?;
                if quote == Some('"') && !matches!(next, '\\' | '"' | '$' | '`') {
                    word.push('\\');
                }
                word.push(next);
                started = true;
            }
            (_, '$' | '`')
            | (
                None,
                ';' | '|' | '&' | '<' | '>' | '(' | ')' | '*' | '?' | '[' | '~' | '#' | '{' | '}',
            ) => {
                return Err("compiler command needs unsupported shell evaluation".into());
            }
            (None, '\'' | '"') => {
                quote = Some(ch);
                started = true;
            }
            (None, ch) if ch.is_whitespace() => {
                if started {
                    result.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            _ => {
                word.push(ch);
                started = true;
            }
        }
    }
    if quote.is_some() {
        return Err("unterminated compiler-command quote".into());
    }
    if started {
        result.push(word);
    }
    if result.is_empty() {
        return Err("empty compiler command".into());
    }
    Ok(result)
}

fn arguments(entry: &Entry) -> Result<Vec<String>> {
    match (&entry.arguments, &entry.command) {
        (Some(args), None) if !args.is_empty() => Ok(args.clone()),
        (None, Some(command)) => words(command),
        _ => Err("expected exactly one compile command representation".into()),
    }
}

fn ast_arguments(args: &[String], directory: &Path, source: &Path) -> Result<Vec<String>> {
    let mut output = Vec::new();
    let mut compile_count = 0;
    let mut source_count = 0;
    let mut object = None;
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-c" => compile_count += 1,
            "-o" => {
                if object
                    .replace(iter.next().ok_or("missing object output")?)
                    .is_some()
                {
                    return Err("multiple object outputs".into());
                }
            }
            _ => {
                if arg.starts_with('@')
                    || arg.starts_with("-o")
                    || arg.starts_with("-M")
                    || arg.starts_with("-save-temps")
                    || matches!(
                        arg.as_str(),
                        "-Xclang" | "-fsyntax-only" | "-S" | "-E" | "--"
                    )
                {
                    return Err(format!("unsupported compile/side-output argument {arg:?}").into());
                }
                if !arg.starts_with('-')
                    && directory.join(arg).canonicalize().ok().as_deref() == Some(source)
                {
                    source_count += 1;
                }
                output.push(arg.clone());
            }
        }
    }
    let object = object.ok_or("ordinary compile command lacks -o")?;
    if !object
        .replace('\\', "/")
        .contains("CMakeFiles/mfs_macos_source_context.dir/")
        || compile_count != 1
        || source_count != 1
    {
        return Err("expected one ordinary original-class source/object compile".into());
    }
    output.extend(
        [
            "-fsyntax-only",
            "-Xclang",
            "-ast-dump=json",
            "-Xclang",
            "-ast-dump-filter=MfsOriginalMac",
        ]
        .map(String::from),
    );
    Ok(output)
}

fn counts(node: &Value, kinds: &mut BTreeMap<String, u64>) -> Result<()> {
    let kind = node
        .get("kind")
        .and_then(Value::as_str)
        .ok_or("AST node lacks kind")?;
    let count = kinds.entry(kind.to_owned()).or_default();
    *count = count.checked_add(1).ok_or("AST count overflow")?;
    if let Some(inner) = node.get("inner") {
        for child in inner.as_array().ok_or("AST inner is not an array")? {
            // Clang can use an empty object for an omitted child.
            if child.as_object().is_some_and(|value| value.is_empty()) {
                continue;
            }
            counts(child, kinds)?;
        }
    }
    Ok(())
}

pub(crate) fn run(args: &[String]) -> Result<()> {
    let [database, fixed_root, destination] = args else {
        return Err(
            "reference macos-ast <compile_commands.json> <fixed F root> <new external output>"
                .into(),
        );
    };
    if std::env::consts::OS != "macos" || std::env::consts::ARCH != "aarch64" {
        return Err("AST observation requires native macOS arm64 and the selected Xcode".into());
    }
    let fixed = Path::new(fixed_root).canonicalize()?;
    let source = fixed.join(SOURCE).canonicalize()?;
    let header = fixed.join(HEADER).canonicalize()?;
    if records::hash(&source)? != SOURCE_HASH || records::hash(&header)? != HEADER_HASH {
        return Err("AST input is not the exact pinned F source/header".into());
    }
    let entries: Vec<Entry> = records::read(Path::new(database))?;
    let mut selected = Vec::new();
    for entry in entries {
        let directory = Path::new(&entry.directory);
        if directory.join(&entry.file).canonicalize().ok().as_deref() != Some(source.as_path()) {
            continue;
        }
        let argv = arguments(&entry)?;
        // Other unchanged/observed/adapter class compilations stay separate.
        let ordinary = entry.output.as_ref().is_some_and(|output| {
            output
                .replace('\\', "/")
                .contains("CMakeFiles/mfs_macos_source_context.dir/")
        }) || argv.windows(2).any(|pair| {
            pair[0] == "-o"
                && pair[1]
                    .replace('\\', "/")
                    .contains("CMakeFiles/mfs_macos_source_context.dir/")
        });
        if ordinary {
            selected.push((entry, argv));
        }
    }
    if selected.len() != 1 {
        return Err("expected exactly one ordinary original source compile entry".into());
    }
    let (entry, argv) = selected.pop().ok_or("missing selected compiler entry")?;
    let directory = Path::new(&entry.directory);
    let compiler = directory.join(&argv[0]).canonicalize()?;
    let expected = Path::new(&std::env::var("CXX")?).canonicalize()?;
    if compiler != expected {
        return Err("compile entry differs from selected Xcode compiler".into());
    }
    let invocation = ast_arguments(&argv, directory, &source)?;
    let destination = Path::new(destination);
    if destination.exists() {
        return Err("AST output already exists; preserve the previous attempt".into());
    }
    let parent = destination.parent().ok_or("AST output lacks parent")?;
    records::external_root(parent)?;
    fs::create_dir(destination)?;
    let root = destination.canonicalize()?;
    fs::copy(database, root.join("compile_commands.json"))?;
    fs::copy(&source, root.join("ScreenCaptureKitScreencap.mm"))?;
    fs::copy(&header, root.join("ScreenCaptureKitScreencap.h"))?;
    records::create_json(
        &root.join("invocation.json"),
        &json!({
            "layout_version": 1, "fixed_commit": "4140ac9674a9fde174212c895c8c37ff3acf1ef8",
            "directory": entry.directory, "original_command": entry.command,
            "original_arguments": argv, "original_output": entry.output,
            "compiler": compiler, "compiler_sha256": records::hash(&compiler)?,
            "arguments": invocation, "source": source, "source_sha256": SOURCE_HASH,
            "header": header, "header_sha256": HEADER_HASH,
            "developer_dir": std::env::var("DEVELOPER_DIR").ok(), "sdkroot": std::env::var("SDKROOT").ok(),
            "transformation": "remove only -c and -o/object; append syntax-only JSON AST/filter flags; no shell",
            "native_capture_acceptance": false, "function_exhaustiveness_accepted": false, "full_P1_accepted": false
        }),
    )?;
    let version = Command::new(&compiler).arg("--version").output()?;
    fs::write(root.join("compiler-version.stdout"), &version.stdout)?;
    fs::write(root.join("compiler-version.stderr"), &version.stderr)?;
    if !version.status.success() {
        return Err("selected compiler version query failed".into());
    }
    let stdout = fs::File::create_new(root.join("ast.json-stream"))?;
    let stderr = fs::File::create_new(root.join("compiler.stderr"))?;
    let status = Command::new(&compiler)
        .args(&invocation)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .status()?;
    records::create_json(
        &root.join("exit.json"),
        &json!({"compiler_exit": status.code(), "success": status.success()}),
    )?;
    if !status.success() {
        return Err("native original-source AST compilation failed; raw outputs retained".into());
    }
    let bytes = fs::read(root.join("ast.json-stream"))?;
    let mut kinds = BTreeMap::new();
    let mut roots = 0_u64;
    for value in serde_json::Deserializer::from_slice(&bytes).into_iter::<Value>() {
        let value = value?;
        if value.get("name").and_then(Value::as_str) != Some("MfsOriginalMac")
            || value.get("kind").and_then(Value::as_str) != Some("NamespaceDecl")
        {
            return Err(
                "unexpected filtered native AST root; retain raw compiler schema for review".into(),
            );
        }
        counts(&value, &mut kinds)?;
        roots = roots.checked_add(1).ok_or("AST root count overflow")?;
    }
    if roots == 0 || kinds.get("BlockDecl").copied().unwrap_or(0) < 4 {
        return Err("native AST lacks the expected four nested screenshot blocks".into());
    }
    records::create_json(
        &root.join("observation.json"),
        &json!({
            "layout_version": 1, "roots": roots, "node_kind_occurrences": kinds,
            "scope": "Compiler-specific raw AST observation only; counts can include declarations and repeated references. Exact callable spans/ownership/semantics require separate source review.",
            "ast_sha256": records::hash(&root.join("ast.json-stream"))?,
            "native_capture_acceptance": false, "function_exhaustiveness_accepted": false, "full_P1_accepted": false
        }),
    )?;
    println!(
        "native macOS Clang AST archived roots={roots} raw_bytes={} capture_acceptance=0 census_acceptance=0",
        bytes.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_quoted_compile_arguments_without_shell_evaluation() {
        assert_eq!(
            words(r#"'/Xcode 27/clang++' -DNAME='a $literal' -I"/include dir" empty"" "" a\ b"#)
                .unwrap(),
            [
                "/Xcode 27/clang++",
                "-DNAME=a $literal",
                "-I/include dir",
                "empty",
                "",
                "a b"
            ]
        );
        for command in [
            "clang++ $(whoami)",
            "clang++ `id`",
            "clang++; id",
            "clang++ > output",
            "clang++ *.mm",
            "clang++ # comment",
            "clang++ {a,b}.mm",
            "clang++ '",
            "clang++ \\",
            "clang++\n-c a",
        ] {
            assert!(words(command).is_err(), "{command:?}");
        }
    }

    #[test]
    fn preserves_native_options_and_rejects_ambiguous_compiles() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let source = directory
            .join("tests/upstream-parity/macos-screencap/source-context.mm")
            .canonicalize()
            .unwrap();
        let ordinary = "CMakeFiles/mfs_macos_source_context.dir/original.mm.o";
        let mut argv: Vec<String> = [
            "/selected Xcode/clang++",
            "-x",
            "objective-c++",
            "-std=gnu++23",
            "-isysroot",
            "/selected SDK",
            "-arch",
            "arm64",
            "-mmacosx-version-min=14.0",
            "-I/OpenCV includes",
            "-DNAME=a $literal",
            "-fblocks",
            "-fno-objc-arc",
            "-o",
            ordinary,
            "-c",
        ]
        .map(String::from)
        .into();
        argv.push(source.to_str().unwrap().to_owned());
        let rewritten = ast_arguments(&argv, directory, &source).unwrap();
        let mut expected = argv[1..13].to_vec();
        expected.push(source.to_str().unwrap().to_owned());
        expected.extend(
            [
                "-fsyntax-only",
                "-Xclang",
                "-ast-dump=json",
                "-Xclang",
                "-ast-dump-filter=MfsOriginalMac",
            ]
            .map(String::from),
        );
        assert_eq!(rewritten, expected);
        for extra in [
            "-c",
            "@flags.rsp",
            "-oextra",
            "-MD",
            "-Xclang",
            source.to_str().unwrap(),
        ] {
            let mut invalid = argv.clone();
            invalid.push(extra.to_owned());
            assert!(
                ast_arguments(&invalid, directory, &source).is_err(),
                "{extra}"
            );
        }
        let mut observed = argv.clone();
        observed[14] = "CMakeFiles/mfs_macos_observed_context.dir/original.mm.o".into();
        assert!(ast_arguments(&observed, directory, &source).is_err());
        let mut missing_source = argv;
        missing_source.pop();
        assert!(ast_arguments(&missing_source, directory, &source).is_err());
    }
}
