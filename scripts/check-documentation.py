# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
"""Repository Markdown checks; no capture build or native acceptance is implied."""
from pathlib import Path
import os
import re
import subprocess
import sys
import tempfile
import unicodedata
import unittest
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parent.parent


def git(*args):
    return subprocess.check_output(["git", "-C", str(ROOT), *args])


def anchors(path):
    result, counts = set(), {}
    fence = None
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        marker = re.match(r"^\s*(`{3,}|~{3,})", line)
        if marker:
            token = marker[1]
            if fence is None:
                fence = token
            elif token[0] == fence[0] and len(token) >= len(fence):
                fence = None
            continue
        if fence:
            continue
        heading = re.match(r"^#{1,6}\s+(.+?)\s*#*$", line)
        if not heading:
            continue
        title = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", heading[1]).lower()
        slug = "".join(
            char for char in title
            if char in "-_ " or not unicodedata.category(char).startswith(("P", "S"))
        ).replace(" ", "-")
        count = counts.get(slug, 0)
        counts[slug] = count + 1
        result.add(slug + (f"-{count}" if count else ""))
    return result


def validate_links(files, root):
    links = fragments = 0
    for path in files:
        for match in re.finditer(r"(?<!!)\[[^\]]+\]\(([^)]+)\)", path.read_text(encoding="utf-8-sig")):
            value = match[1].strip().strip("<>")
            parsed = urlsplit(value)
            if parsed.scheme or parsed.netloc:
                continue
            target = (path.parent / unquote(parsed.path)).resolve() if parsed.path else path
            if not target.is_relative_to(root) or not target.is_file():
                raise ValueError(f"{path}: missing or external repository target {value}")
            links += 1
            if parsed.fragment:
                if unquote(parsed.fragment) not in anchors(target):
                    raise ValueError(f"{path}: missing anchor {value}")
                fragments += 1
    return links, fragments


def commit_subject(message):
    # Keep the existing xtask quality::commit_message subject grammar: lowercase
    # custom types, optional nonempty scope, optional trailing breaking marker.
    subject = message.splitlines()[0] if message.splitlines() else ""
    prefix, separator, description = subject.partition(": ")
    if not separator or not description.strip():
        raise ValueError("Commit subject must use <type>[scope][!]: <description>")
    prefix = prefix.removesuffix("!")
    if "(" in prefix:
        kind, scope = prefix.split("(", 1)
        if not scope.endswith(")"):
            raise ValueError("Unclosed commit scope")
        scope = scope[:-1]
        if not scope.strip() or "(" in scope or ")" in scope:
            raise ValueError("Invalid commit scope")
    else:
        kind = prefix
    if not re.fullmatch("[a-z]+", kind):
        raise ValueError("Commit type must contain lowercase ASCII letters")


class Checks(unittest.TestCase):
    def test_preserves_duplicate_unicode_anchors_and_excludes_nested_fences(self):
        with temporary() as directory:
            path = Path(directory) / "page.md"
            path.write_text("# 原生 `Frame`\n# 原生 `Frame`\n````text\n```\n# hidden\n````\n~~~\n# hidden too\n~~~\n", encoding="utf-8")
            self.assertEqual(anchors(path), {"原生-frame", "原生-frame-1"})

    def test_accepts_encoded_paths_and_rejects_missing_targets_or_anchors(self):
        with temporary() as directory:
            root = Path(directory).resolve()
            target = root / "named file.md"
            target.write_text("# One\n", encoding="utf-8")
            page = root / "page.md"
            page.write_text("[valid](named%20file.md#one)\n[web](https://example.invalid/)\n", encoding="utf-8")
            self.assertEqual(validate_links([page], root), (1, 1))
            for value in ["missing.md", "named%20file.md#absent", "../outside.md"]:
                page.write_text(f"[invalid]({value})\n", encoding="utf-8")
                with self.assertRaises(ValueError):
                    validate_links([page], root)

    def test_accepts_scopes_breaking_markers_and_custom_types(self):
        for message in ["feat: capture", "fix(gdi)!: reject invalid buffers\n\nBREAKING CHANGE: old buffers", "ci: validate", "custom: purpose"]:
            commit_subject(message)

    def test_rejects_malformed_or_empty_commit_subjects(self):
        for message in ["", "update code", "Feat: wrong case", "feat(): empty scope", "fix(gdi: missing close", "fix!: ", "fix: ", "fix(gdi))!: malformed"]:
            with self.subTest(message=message), self.assertRaises(ValueError):
                commit_subject(message)


def temporary():
    deps = ROOT.parent / "maafw-screencap-deps"
    deps.mkdir(exist_ok=True)
    return tempfile.TemporaryDirectory(prefix="documentation-check-", dir=deps)


def main():
    if sys.argv[1:] == ["--self-test"]:
        unittest.main(argv=[sys.argv[0]])
        return
    if sys.argv[1:]:
        raise ValueError("Expected no arguments or --self-test")
    ref = os.environ.get("MFS_COMMIT_MESSAGE_REF", "HEAD")
    if ref != "HEAD" and not re.fullmatch(r"[0-9a-f]{40}", ref):
        raise ValueError("Documentation commit must be an exact Git SHA or HEAD")
    commit_subject(git("log", "-1", "--format=%B", ref).decode("utf-8"))
    files = [ROOT / os.fsdecode(name) for name in git("ls-files", "-z", "*.md").split(b"\0") if name]
    links, fragments = validate_links(files, ROOT)
    subprocess.run(["git", "-C", str(ROOT), "show", "--format=", "--check", ref], check=True)
    subprocess.run(["git", "-C", str(ROOT), "diff", "--check"], check=True)
    print(f"Documentation checks passed: files={len(files)} relative_links={links} anchors={fragments}; Conventional Commit subject passed. Native capture was not executed.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"Documentation check failed: {error}", file=sys.stderr)
        sys.exit(1)
