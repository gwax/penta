from __future__ import annotations

import gzip
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "profile_attribution.py"


def fixture_documents(native_weight: int, hot_weight: int, end_time: int):
    strings = ["0x100", "0x101", "0x102", "0x200"]
    frame_table = {
        "address": [100, 101, 102, 200],
        "func": [0, 1, 2, 3],
        "length": 4,
        "line": [None, None, None, None],
    }
    func_table = {
        "fileName": [None, None, None, None],
        "length": 4,
        "lineNumber": [None, None, None, None],
        "name": [0, 1, 2, 3],
        "resource": [0, 0, 0, 1],
    }
    resource_table = {"length": 2, "lib": [0, 1], "name": [0, 3]}
    stack_table = {
        "frame": [0, 1, 3, 2],
        "length": 4,
        "prefix": [None, 0, 1, 1],
    }
    thread = {
        "name": "penta-match",
        "processName": "penta-match",
        "isMainThread": True,
        "stringArray": strings,
        "frameTable": frame_table,
        "funcTable": func_table,
        "resourceTable": resource_table,
        "stackTable": stack_table,
        "samples": {
            "length": 2,
            "stack": [2, 3],
            "time": [0, end_time],
            "weight": [native_weight, hot_weight],
            "weightType": "samples",
        },
    }
    profile = {
        "meta": {
            "preprocessedProfileVersion": 49,
            "product": "penta-match",
            "symbolicated": False,
            "version": 24,
        },
        "libs": [
            {"debugName": "penta-match", "codeId": "APP"},
            {"debugName": "libsystem_malloc.dylib", "codeId": "MALLOC"},
        ],
        "threads": [thread],
    }
    symbol_strings = [
        "penta_match::main",
        "<penta::game::Game>::legal_actions",
        "<penta::game::Game>::hot_path",
        "src/game/mod.rs",
        "malloc",
    ]
    symbols = {
        "string_table": symbol_strings,
        "data": [
            {
                "debug_name": "penta-match",
                "code_id": "APP",
                "known_addresses": [[100, 0], [101, 1], [102, 2]],
                "symbol_table": [
                    {
                        "frames": [{"function": 0, "file": 3, "line": 1}],
                        "symbol": 0,
                    },
                    {
                        "frames": [{"function": 1, "file": 3, "line": 10}],
                        "symbol": 1,
                    },
                    {
                        "frames": [{"function": 2, "file": 3, "line": 20}],
                        "symbol": 2,
                    },
                ],
            },
            {
                "debug_name": "libsystem_malloc.dylib",
                "code_id": "MALLOC",
                "known_addresses": [[200, 0]],
                "symbol_table": [{"symbol": 4}],
            },
        ],
    }
    return profile, symbols


class ProfileAttributionTest(unittest.TestCase):
    def write_documents(
        self,
        directory: Path,
        name: str,
        profile: dict,
        symbols: dict,
    ) -> Path:
        profile_path = directory / f"{name}.json.gz"
        with gzip.open(profile_path, "wt", encoding="utf-8") as destination:
            json.dump(profile, destination)
        symbols_path = directory / f"{name}.json.syms.json"
        symbols_path.write_text(json.dumps(symbols), encoding="utf-8")
        return profile_path

    def write_fixture(
        self, directory: Path, name: str, native_weight: int, hot_weight: int, end_time: int
    ) -> Path:
        profile, symbols = fixture_documents(native_weight, hot_weight, end_time)
        return self.write_documents(directory, name, profile, symbols)

    def run_json(self, *arguments: object) -> dict:
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), *map(str, arguments), "--json"],
            check=True,
            capture_output=True,
            text=True,
        )
        return json.loads(completed.stdout)

    def test_summary_discovers_symbols_and_attributes_native_leaf(self):
        with tempfile.TemporaryDirectory() as temporary:
            profile = self.write_fixture(Path(temporary), "sample", 2, 3, 4)
            result = self.run_json(
                "summary", profile, "--caller-of", "hot_path", "--top", 20
            )
            completed = subprocess.run(
                [sys.executable, str(SCRIPT), "summary", str(profile), "--top", "20"],
                check=True,
                capture_output=True,
                text=True,
            )

        self.assertEqual(result["sample_weight"], 5)
        self.assertEqual(result["profile_span_ms"], 4)
        self.assertTrue(result["symbols"].endswith("sample.json.syms.json"))
        self.assertEqual(
            result["system_attribution"]["allocator"][0],
            {
                "name": "<penta::game::Game>::legal_actions",
                "share_percent": 40.0,
                "weight": 2,
            },
        )
        self.assertEqual(
            result["callers"]["hot_path"][0]["name"],
            "<penta::game::Game>::legal_actions",
        )

        self.assertIn("ALLOCATOR CPU LEAVES ATTRIBUTED", completed.stdout)

    def test_compare_reports_absolute_and_profile_span_deltas(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            before = self.write_fixture(root, "before", 4, 6, 10)
            after = self.write_fixture(root, "after", 2, 3, 5)
            result = self.run_json("compare", before, after, "--top", 20)

        self.assertEqual(result["delta"]["profile_span_ms"], -5)
        self.assertEqual(result["delta"]["profile_span_percent"], -50)
        self.assertEqual(result["delta"]["sample_weight"], -5)
        self.assertEqual(result["delta"]["sample_weight_percent"], -50)

    def test_missing_symbol_sidecar_fails_with_recovery_guidance(self):
        with tempfile.TemporaryDirectory() as temporary:
            profile, _ = fixture_documents(1, 1, 1)
            profile_path = Path(temporary) / "missing.json.gz"
            with gzip.open(profile_path, "wt", encoding="utf-8") as destination:
                json.dump(profile, destination)
            completed = subprocess.run(
                [sys.executable, str(SCRIPT), "summary", str(profile_path)],
                check=False,
                capture_output=True,
                text=True,
            )

        self.assertEqual(completed.returncode, 2)
        self.assertIn("symbol sidecar", completed.stderr)
        self.assertIn("--symbols", completed.stderr)

    def test_unknown_input_schema_fails_with_viewer_guidance(self):
        with tempfile.TemporaryDirectory() as temporary:
            profile, symbols = fixture_documents(1, 1, 1)
            profile["meta"]["preprocessedProfileVersion"] = 50
            profile_path = self.write_documents(
                Path(temporary), "unknown-schema", profile, symbols
            )
            completed = subprocess.run(
                [sys.executable, str(SCRIPT), "summary", str(profile_path)],
                check=False,
                capture_output=True,
                text=True,
            )

        self.assertEqual(completed.returncode, 2)
        self.assertIn("unsupported Firefox processed-profile schema 24/50", completed.stderr)
        self.assertIn("make profile-engine-open", completed.stderr)

    def test_mismatched_consumed_column_fails_closed(self):
        with tempfile.TemporaryDirectory() as temporary:
            profile, symbols = fixture_documents(1, 1, 1)
            profile["threads"][0]["stackTable"]["prefix"].pop()
            profile_path = self.write_documents(
                Path(temporary), "short-prefix", profile, symbols
            )
            completed = subprocess.run(
                [sys.executable, str(SCRIPT), "summary", str(profile_path)],
                check=False,
                capture_output=True,
                text=True,
            )

        self.assertEqual(completed.returncode, 2)
        self.assertIn("stackTable.prefix must contain 4 values", completed.stderr)

    def test_out_of_range_sidecar_mapping_fails_closed(self):
        with tempfile.TemporaryDirectory() as temporary:
            profile, symbols = fixture_documents(1, 1, 1)
            symbols["data"][0]["known_addresses"][0][1] = 99
            profile_path = self.write_documents(
                Path(temporary), "bad-sidecar", profile, symbols
            )
            completed = subprocess.run(
                [sys.executable, str(SCRIPT), "summary", str(profile_path)],
                check=False,
                capture_output=True,
                text=True,
            )

        self.assertEqual(completed.returncode, 2)
        self.assertIn("invalid address mapping", completed.stderr)

    def test_valid_unknown_resource_and_optional_inline_data_are_accepted(self):
        with tempfile.TemporaryDirectory() as temporary:
            profile, symbols = fixture_documents(2, 3, 4)
            profile["threads"][0]["funcTable"]["resource"][2] = -1
            symbols["data"][0]["symbol_table"][0]["frames"] = []
            symbols["data"][0]["symbol_table"][1]["frames"][0]["function"] = None
            profile_path = self.write_documents(
                Path(temporary), "optional-symbol-data", profile, symbols
            )
            result = self.run_json("summary", profile_path, "--top", 20)

        self.assertEqual(result["sample_weight"], 5)
        self.assertEqual(
            result["attributed_self"][0],
            {
                "name": "<penta::game::Game>::legal_actions",
                "share_percent": 100.0,
                "weight": 5,
            },
        )
        self.assertEqual(
            result["system_attribution"]["allocator"][0]["name"],
            "<penta::game::Game>::legal_actions",
        )


if __name__ == "__main__":
    unittest.main()
