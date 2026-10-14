"""Profile compiler contract checks independent of native toolchain availability."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("profile_build", ROOT / "scripts/build-profiles.py")
build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)

class ProfileCompilerTests(unittest.TestCase):
    def setUp(self):
        self.raw = {p["id"]: p for p in json.loads((ROOT / "languages/profiles.json").read_text())["profiles"]}

    def test_inherited_rules_can_be_overridden_without_mutating_the_base(self):
        raw = {"c": copy.deepcopy(self.raw["c"]), "rust": {"base": "c", "id": "rust", "name": "Rust", "extensions": ["rs"], "required_function_keyword": "fn"}}
        compiled = {p["id"]: p for p in build.compile_profiles(raw)}
        self.assertEqual(compiled["rust"]["required_function_keyword"], "fn")
        self.assertEqual(compiled["c"]["required_function_keyword"], "")
        self.assertEqual(compiled["rust"]["controls"], compiled["c"]["controls"])

    def test_cycles_and_unknown_bases_fail(self):
        for raw in ({"x": {"base": "missing"}}, {"x": {"base": "y"}, "y": {"base": "x"}}):
            with self.assertRaises(ValueError):
                build.compile_profiles(raw)

    def test_duplicate_extensions_fail(self):
        self.raw["rust"]["extensions"] = ["c"]
        with self.assertRaises(ValueError):
            build.compile_profiles(self.raw)

    def test_executable_or_external_geometry_fails(self):
        for operation in ({"op": "script", "data": "alert(1)"}, {"op": "path", "data": "M 0 0' onload='alert(1)"}):
            raw = copy.deepcopy(self.raw)
            raw["c"]["collar"] = [operation]
            with self.assertRaises(ValueError):
                build.compile_profiles(raw)

if __name__ == "__main__":
    unittest.main()
