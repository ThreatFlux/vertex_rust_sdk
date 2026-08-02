#!/usr/bin/env python3
"""Validate the repository's machine-checkable documentation contract."""

from __future__ import annotations

import argparse
import difflib
import os
import re
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import unquote, urlsplit


QUICKSTART_BEGIN = "<!-- BEGIN QUICKSTART -->"
QUICKSTART_END = "<!-- END QUICKSTART -->"
FEATURE_HEADING = re.compile(r"^## Cargo features\s*$", re.MULTILINE)
NEXT_H2 = re.compile(r"^## ", re.MULTILINE)
INLINE_LINK = re.compile(r"!?\[[^\]\n]*\]\(\s*(?P<target><[^>\n]+>|[^\s)]+)")
REFERENCE_LINK = re.compile(
    r"^\s{0,3}\[[^\]\n]+\]:\s*(?P<target><[^>\n]+>|\S+)", re.MULTILINE
)
VERSION = re.compile(r"(?<![\w.])(\d+\.\d+(?:\.\d+)?)(?![\w.])")


@dataclass(frozen=True)
class Manifest:
    rust_version: str
    features: frozenset[str]


@dataclass(frozen=True)
class Problem:
    path: Path
    line: int
    message: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="repository root",
    )
    parser.add_argument("--print-msrv", action="store_true")
    return parser.parse_args()


def read_manifest(path: Path) -> Manifest:
    with path.open("rb") as cargo_file:
        cargo = tomllib.load(cargo_file)

    package = cargo["package"]
    features = cargo.get("features", {})
    return Manifest(
        rust_version=str(package["rust-version"]),
        features=frozenset(features),
    )


def line_number(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def add_problem(
    problems: list[Problem], path: Path, text: str, offset: int, message: str
) -> None:
    problems.append(Problem(path, line_number(text, offset), message))


def check_msrv(
    path: Path, readme: str, manifest: Manifest, problems: list[Problem]
) -> None:
    matching_lines = [
        line
        for line in readme.splitlines()
        if "MSRV" in line or "minimum supported Rust version" in line
    ]
    versions = {version for line in matching_lines for version in VERSION.findall(line)}
    if versions != {manifest.rust_version}:
        problems.append(
            Problem(
                path,
                1,
                "README MSRV declarations must exactly match Cargo.toml "
                f"({manifest.rust_version}); found {sorted(versions)}",
            )
        )


def feature_names(section: str) -> list[str]:
    names: list[str] = []
    for line in section.splitlines():
        if not line.lstrip().startswith("|"):
            continue
        first_cell = line.strip().strip("|").split("|", maxsplit=1)[0]
        match = re.search(r"`([A-Za-z0-9_-]+)`", first_cell)
        if match:
            names.append(match.group(1))
    return names


def check_features(
    path: Path, readme: str, manifest: Manifest, problems: list[Problem]
) -> None:
    heading = FEATURE_HEADING.search(readme)
    if heading is None:
        problems.append(Problem(path, 1, "README must contain '## Cargo features'"))
        return

    next_heading = NEXT_H2.search(readme, heading.end())
    end = next_heading.start() if next_heading else len(readme)
    documented = feature_names(readme[heading.end() : end])
    documented_set = set(documented)
    if documented_set != manifest.features:
        missing = sorted(manifest.features - documented_set)
        unknown = sorted(documented_set - manifest.features)
        add_problem(
            problems,
            path,
            readme,
            heading.start(),
            f"Cargo feature table mismatch; missing={missing}, unknown={unknown}",
        )
    if len(documented) != len(documented_set):
        add_problem(
            problems, path, readme, heading.start(), "duplicate Cargo feature entry"
        )


def check_installation(path: Path, readme: str, problems: list[Problem]) -> None:
    required = (
        "cargo add threatflux-vertex-rust-sdk --no-default-features",
        "cargo add tokio --features macros,rt-multi-thread",
    )
    for command in required:
        if command not in readme:
            problems.append(Problem(path, 1, f"README must contain: {command}"))

    hard_coded_dependency = re.search(
        r'threatflux-vertex-rust-sdk\s*=\s*\{[^\n]*version\s*=', readme
    )
    if hard_coded_dependency is not None:
        add_problem(
            problems,
            path,
            readme,
            hard_coded_dependency.start(),
            "README install guidance must not hard-code the crate release",
        )


def extract_quickstart(path: Path, readme: str, problems: list[Problem]) -> str | None:
    if readme.count(QUICKSTART_BEGIN) != 1 or readme.count(QUICKSTART_END) != 1:
        problems.append(
            Problem(path, 1, "README must contain one quickstart marker pair")
        )
        return None

    begin = readme.index(QUICKSTART_BEGIN) + len(QUICKSTART_BEGIN)
    end = readme.index(QUICKSTART_END)
    region = readme[begin:end].strip("\n")
    match = re.fullmatch(r"```rust\n(?P<code>.*)\n```", region, re.DOTALL)
    if match is None:
        add_problem(
            problems, path, readme, begin, "quickstart markers must wrap one Rust block"
        )
        return None
    return match.group("code")


def check_quickstart(
    readme_path: Path,
    readme: str,
    example_path: Path,
    problems: list[Problem],
) -> None:
    actual = extract_quickstart(readme_path, readme, problems)
    if actual is None:
        return
    expected = example_path.read_text(encoding="utf-8").rstrip("\n")
    if actual == expected:
        return

    diff = "\n".join(
        difflib.unified_diff(
            expected.splitlines(),
            actual.splitlines(),
            fromfile="examples/quickstart.rs",
            tofile="README quickstart",
            lineterm="",
        )
    )
    problems.append(Problem(readme_path, 1, f"quickstart is out of sync\n{diff}"))


def markdown_files(root: Path) -> list[Path]:
    return sorted(
        path
        for path in root.rglob("*.md")
        if ".git" not in path.parts and "target" not in path.parts
    )


def link_targets(markdown: str) -> list[tuple[int, str]]:
    matches = list(INLINE_LINK.finditer(markdown))
    matches.extend(REFERENCE_LINK.finditer(markdown))
    matches.sort(key=lambda match: match.start())
    return [
        (
            line_number(markdown, match.start("target")),
            match.group("target").strip("<>"),
        )
        for match in matches
    ]


def check_local_links(root: Path, problems: list[Problem]) -> None:
    for path in markdown_files(root):
        markdown = path.read_text(encoding="utf-8")
        for line, target in link_targets(markdown):
            parsed = urlsplit(target)
            if (
                parsed.scheme
                or parsed.netloc
                or not parsed.path
                or parsed.path.startswith("/")
            ):
                continue
            resolved = (path.parent / unquote(parsed.path)).resolve()
            if not resolved.is_relative_to(root):
                problems.append(
                    Problem(path, line, f"local link escapes repository: {target}")
                )
                continue
            if not resolved.exists():
                problems.append(Problem(path, line, f"broken local link: {target}"))


def check_readme_contract(path: Path, readme: str, problems: list[Problem]) -> None:
    required = (
        "not an official",
        "docs/api-coverage.md",
        "docs/configuration.md",
    )
    for phrase in required:
        if phrase not in readme:
            problems.append(Problem(path, 1, f"README must contain: {phrase}"))

    banned = (
        "GcpAuthProvider",
        'VertexClient::new("',
        "gcloud auth application-default login",
    )
    for phrase in banned:
        offset = readme.find(phrase)
        if offset >= 0:
            add_problem(
                problems, path, readme, offset, f"obsolete README text: {phrase}"
            )


def report(root: Path, problems: list[Problem]) -> int:
    if not problems:
        print("Documentation contract passed.")
        return 0

    for problem in problems:
        relative = problem.path.relative_to(root).as_posix()
        if os.environ.get("GITHUB_ACTIONS") == "true":
            message = problem.message.replace("%", "%25").replace("\n", "%0A")
            print(f"::error file={relative},line={problem.line}::{message}")
        print(f"{relative}:{problem.line}: error: {problem.message}")
    print(f"Documentation contract failed with {len(problems)} error(s).")
    return 1


def main() -> int:
    args = parse_args()
    root = args.root.resolve()
    manifest = read_manifest(root / "Cargo.toml")
    if args.print_msrv:
        print(manifest.rust_version)
        return 0

    readme_path = root / "README.md"
    readme = readme_path.read_text(encoding="utf-8")
    problems: list[Problem] = []
    check_msrv(readme_path, readme, manifest, problems)
    check_features(readme_path, readme, manifest, problems)
    check_installation(readme_path, readme, problems)
    check_quickstart(readme_path, readme, root / "examples/quickstart.rs", problems)
    check_readme_contract(readme_path, readme, problems)
    check_local_links(root, problems)
    return report(root, problems)


if __name__ == "__main__":
    sys.exit(main())
