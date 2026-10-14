"""Run each trusted native-language authoring module and compile portable profiles.

Normal cargo/WASM builds use the checked-in artifacts and need no other runtimes.
--check executes all nine modules and rejects stale generated artifacts.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
MODULES = {
    "c": ("c", "languages/c/profile.c"),
    "rust": ("rust", "languages/rust/profile.rs"),
    "python": ("python", "languages/python/profile.py"),
    "javascript": ("node", "languages/javascript/profile.js"),
    "typescript": ("node", "languages/typescript/profile.ts"),
    "go": ("go", "languages/go/profile.go"),
    "cpp": ("cpp", "languages/cpp/profile.cpp"),
    "java": ("java", "languages/java/Profile.java"),
    "csharp": ("csharp", "languages/csharp/Profile.cs"),
}
STRINGS = {"id", "name", "call_group", "line_comment", "type_keyword", "type_default_kind", "receiver_keyword", "required_function_keyword", "arrow_token", "sigil", "color"}
BOOLS = {"block_comments", "raw_strings", "lifetimes", "triple_quotes"}
LISTS = {"extensions", "object_keywords", "controls", "reject_function_prefixes", "member_operators", "self_qualifiers", "merge_containers"}
MAPS = {"indented_declarations", "braced_declarations", "type_kinds"}
KEYS = STRINGS | BOOLS | LISTS | MAPS | {"repeats", "collar"}

def tool(name):
    path = shutil.which(name)
    if not path:
        raise ValueError(f"Profile authoring requires {name}; ordinary cargo and website builds do not. See languages/README.md.")
    return path

def run(command):
    return subprocess.check_output(command, cwd=ROOT, text=True, encoding="utf-8", env={**os.environ, "GO111MODULE": "off"}, timeout=180)

def emit(kind, source, output):
    executable = output / ("profile.exe" if os.name == "nt" else "profile")
    if kind in {"c", "cpp", "rust"}:
        compiler = tool({"c": "gcc", "cpp": "g++", "rust": "rustc"}[kind])
        flags = ["--edition=2021"] if kind == "rust" else ["-Wall", "-Wextra", "-Werror", "-std=c11" if kind == "c" else "-std=c++17"]
        run([compiler, *flags, str(source), "-o", str(executable)])
        return run([str(executable)])
    if kind == "python":
        return run([sys.executable, str(source)])
    if kind == "node":
        return run([tool("node"), str(source)])
    if kind == "go":
        return run([tool("go"), "run", str(source)])
    if kind == "java":
        compiler = [tool("javac")] if shutil.which("javac") else [tool("java"), "-m", "jdk.compiler/com.sun.tools.javac.Main"]
        run([*compiler, "-encoding", "UTF-8", "-d", str(output), str(source)])
        return run([tool("java"), "-cp", str(output), "Profile"])
    if kind == "csharp":
        assembly = output / "Profile.exe"
        run([tool("mcs"), "-warnaserror", "-out:" + str(assembly), str(source)])
        return run([tool("mono"), str(assembly)])
    raise ValueError(f"Unknown authoring toolchain: {kind}")

def validate(profile):
    if set(profile) != KEYS:
        raise ValueError(f"Profile {profile.get('id')} has missing/unknown keys: {set(profile) ^ KEYS}")
    for key in STRINGS:
        if not isinstance(profile[key], str):
            raise ValueError(f"{key} must be a string")
    for key in BOOLS:
        if type(profile[key]) is not bool:
            raise ValueError(f"{key} must be boolean")
    for key in LISTS:
        if not isinstance(profile[key], list) or any(not isinstance(v, str) for v in profile[key]):
            raise ValueError(f"{key} must be a list of strings")
    for key in MAPS:
        if not isinstance(profile[key], dict) or any(not isinstance(k, str) or not isinstance(v, str) for k, v in profile[key].items()):
            raise ValueError(f"{key} must be a string map")
    if not profile["line_comment"] or not profile["sigil"] or not profile["extensions"]:
        raise ValueError("Comment marker, sigil and extensions must be nonempty")
    if not re.fullmatch(r"#[0-9a-fA-F]{6}", profile["color"]):
        raise ValueError("Language color must be a six-digit RGB hex color")
    if any(not re.fullmatch(r"[a-z]+", ext) for ext in profile["extensions"]):
        raise ValueError("Invalid extension")
    if type(profile["repeats"]) is not int or not 1 <= profile["repeats"] <= 64:
        raise ValueError("Invalid collar repeat count")
    if not isinstance(profile["collar"], list) or not profile["collar"]:
        raise ValueError("Missing collar geometry")
    for operation in profile["collar"]:
        if operation.get("op") == "path":
            if set(operation) != {"op", "data"} or not re.fullmatch(r"[MmLlHhVvCcSsQqTtAaZz0-9eE+.,\s-]+", operation["data"]):
                raise ValueError("Invalid portable path geometry")
        elif operation.get("op") == "polygon":
            if set(operation) != {"op", "cx", "cy", "r", "n", "step", "rotation_degrees"}:
                raise ValueError("Invalid polygon keys")
            if any(type(operation[k]) not in {int, float} or not math.isfinite(operation[k]) for k in {"cx", "cy", "r", "rotation_degrees"}):
                raise ValueError("Non-finite polygon geometry")
            if type(operation["n"]) is not int or type(operation["step"]) is not int or not 3 <= operation["n"] <= 64 or not 0 < operation["step"] < operation["n"] or operation["r"] <= 0:
                raise ValueError("Invalid polygon geometry")
        else:
            raise ValueError("Unsupported geometry operation")

def compile_profiles(raw):
    compiled = {}
    visiting = set()
    def resolve(key):
        if key in compiled:
            return compiled[key]
        if key in visiting or key not in raw:
            raise ValueError(f"Unknown/cyclic profile inheritance: {key}")
        visiting.add(key)
        source = dict(raw[key])
        base = source.pop("base", None)
        if set(source) - KEYS:
            raise ValueError(f"Unknown profile fields in {key}: {set(source) - KEYS}")
        profile = {**(resolve(base) if base else {}), **source}
        if profile["id"] != key:
            raise ValueError("Profile module ID does not match its registry entry")
        validate(profile)
        visiting.remove(key)
        compiled[key] = profile
        return profile
    profiles = [resolve(key) for key in sorted(raw)]
    suffixes = [suffix for p in profiles for suffix in p["extensions"]]
    if len(set(suffixes)) != len(suffixes) or len({p["name"] for p in profiles}) != len(profiles):
        raise ValueError("Duplicate language name or source extension")
    return profiles

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Fail if native generators and committed portable artifacts differ")
    args = parser.parse_args()
    raw = {}
    with tempfile.TemporaryDirectory(prefix="magic-profiles-") as temp:
        for key, (kind, path) in MODULES.items():
            output = Path(temp) / key
            output.mkdir()
            raw[key] = json.loads(emit(kind, ROOT / path, output))
    profiles = compile_profiles(raw)
    registry = {"version": 1, "profiles": profiles}
    portable = json.dumps(registry, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    suffixes = sorted(ext for profile in profiles for ext in profile["extensions"])
    browser = "// Generated by scripts/build-profiles.py; edit native authoring modules.\nexport const sourceExtensions = " + json.dumps(suffixes) + ";\n"
    browser += "export const languageColors = " + json.dumps({profile["name"]: profile["color"] for profile in profiles}, sort_keys=True) + ";\n"
    manifest = json.dumps({"version": 1, "sources": {path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest() for _, path in MODULES.values()}}, sort_keys=True, indent=2) + "\n"
    outputs = {"languages/profiles.json": portable, "languages/sources.json": manifest, "web/languages.js": browser}
    for path, content in outputs.items():
        destination = ROOT / path
        if args.check:
            if not destination.is_file() or destination.read_text(encoding="utf-8") != content:
                raise ValueError(f"Stale profile artifact: {path}. Run python3 scripts/build-profiles.py.")
        else:
            destination.write_text(content, encoding="utf-8")
    print(f"Nine native-language modules {'verified' if args.check else 'compiled'}; {len(suffixes)} extensions, deterministic portable profiles.")

if __name__ == "__main__":
    try:
        main()
    except (ValueError, subprocess.SubprocessError) as error:
        print(f"Profile build failed: {error}", file=sys.stderr)
        sys.exit(1)
