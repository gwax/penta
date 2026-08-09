import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[4]
SCRIPT = REPO_ROOT / "scripts/benchmark_engine.py"
SPEC = importlib.util.spec_from_file_location("benchmark_engine", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
benchmark_engine = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = benchmark_engine
previous_dont_write_bytecode = sys.dont_write_bytecode
sys.dont_write_bytecode = True
try:
    SPEC.loader.exec_module(benchmark_engine)
finally:
    sys.dont_write_bytecode = previous_dont_write_bytecode


class BenchmarkEngineTests(unittest.TestCase):
    def test_u64_environment_values_are_validated(self):
        with mock.patch.dict(os.environ, {"PROFILE_GAMES": "0"}):
            with self.assertRaisesRegex(
                benchmark_engine.BenchmarkError, "PROFILE_GAMES must be a positive"
            ):
                benchmark_engine.parse_u64(
                    "PROFILE_GAMES", default=4000, positive=True
                )

        with mock.patch.dict(os.environ, {"PROFILE_SEED": str(1 << 64)}):
            with self.assertRaisesRegex(
                benchmark_engine.BenchmarkError, "64-bit integer"
            ):
                benchmark_engine.parse_u64("PROFILE_SEED", default=1, positive=False)

    def test_workload_identity_changes_with_games_and_seed(self):
        root = Path("/repo")
        first = benchmark_engine.Settings(
            root, "refs/heads/main", 2000, 1, 1, 10, root / "first.json"
        )
        more_games = benchmark_engine.Settings(
            root, "refs/heads/main", 4000, 1, 1, 10, root / "second.json"
        )
        new_seed = benchmark_engine.Settings(
            root, "refs/heads/main", 2000, 2, 1, 10, root / "third.json"
        )

        self.assertNotEqual(first.workload_identity, more_games.workload_identity)
        self.assertNotEqual(first.workload_identity, new_seed.workload_identity)

    def test_git_common_dir_resolves_across_linked_worktrees(self):
        with tempfile.TemporaryDirectory() as temporary_name:
            temporary = Path(temporary_name)
            primary = temporary / "primary"
            linked = temporary / "linked"
            subprocess.run(
                ("git", "init", "--initial-branch=main", str(primary)),
                check=True,
                capture_output=True,
            )
            subprocess.run(
                ("git", "-C", str(primary), "config", "user.name", "Test User"),
                check=True,
            )
            subprocess.run(
                (
                    "git",
                    "-C",
                    str(primary),
                    "config",
                    "user.email",
                    "test@example.com",
                ),
                check=True,
            )
            (primary / "tracked.txt").write_text("tracked\n", encoding="utf-8")
            subprocess.run(
                ("git", "-C", str(primary), "add", "tracked.txt"), check=True
            )
            subprocess.run(
                ("git", "-C", str(primary), "commit", "-m", "initial"),
                check=True,
                capture_output=True,
            )
            subprocess.run(
                (
                    "git",
                    "-C",
                    str(primary),
                    "worktree",
                    "add",
                    "--detach",
                    str(linked),
                ),
                check=True,
                capture_output=True,
            )

            self.assertEqual(
                benchmark_engine.git_common_dir(primary),
                benchmark_engine.git_common_dir(linked),
            )
            self.assertEqual(
                benchmark_engine.git_common_dir(linked), (primary / ".git").resolve()
            )

    def test_binary_cache_rejects_incomplete_and_modified_artifacts(self):
        identity = {"revision": "abc", "build_profile": "release"}
        tools = {"cargo": "cargo 1", "rustc": "rustc 1"}
        cargo_configs = [{"scope": "cargo-home", "sha256": "123"}]
        with tempfile.TemporaryDirectory() as temporary_name:
            directory = Path(temporary_name)
            binary = directory / "penta-match"
            binary.write_bytes(b"first")
            manifest = {
                "identity": identity,
                "binary_sha256": benchmark_engine.file_sha256(binary),
                "tools": tools,
                "cargo_configurations": cargo_configs,
            }
            (directory / "manifest.json").write_text(
                json.dumps(manifest), encoding="utf-8"
            )

            self.assertIsNone(
                benchmark_engine.binary_cache_valid(directory, identity)
            )
            (directory / "complete").touch()
            self.assertIsNotNone(
                benchmark_engine.binary_cache_valid(
                    directory,
                    identity,
                    expected_tools=tools,
                    expected_cargo_configs=cargo_configs,
                )
            )
            self.assertIsNone(
                benchmark_engine.binary_cache_valid(
                    directory,
                    identity,
                    expected_tools={"cargo": "cargo 2", "rustc": "rustc 1"},
                    expected_cargo_configs=cargo_configs,
                )
            )
            binary.write_bytes(b"changed")
            self.assertIsNone(
                benchmark_engine.binary_cache_valid(directory, identity)
            )

    def test_cargo_configuration_content_changes_its_fingerprint(self):
        with tempfile.TemporaryDirectory() as temporary_name:
            temporary = Path(temporary_name)
            source = temporary / "source"
            cargo_home = temporary / "cargo-home"
            source.mkdir()
            cargo_home.mkdir()
            config = cargo_home / "config.toml"
            config.write_text("[build]\nrustflags = ['-Ctarget-cpu=generic']\n")
            with mock.patch.dict(
                os.environ, {"CARGO_HOME": str(cargo_home)}, clear=False
            ):
                before = benchmark_engine.cargo_configuration_fingerprints(source)
                config.write_text("[build]\nrustflags = ['-Ctarget-cpu=native']\n")
                after = benchmark_engine.cargo_configuration_fingerprints(source)

            self.assertNotEqual(before, after)

    def test_comparison_metadata_sits_beside_hyperfine_json(self):
        output = Path("target/profiles/engine-main-compare.json")
        self.assertEqual(
            benchmark_engine.comparison_metadata_path(output),
            Path("target/profiles/engine-main-compare.metadata.json"),
        )

    def test_baseline_command_uses_the_lazy_cache_path(self):
        root = Path("/repo")
        settings = benchmark_engine.Settings(
            root, "refs/heads/main", 4000, 1, 1, 10, root / "compare.json"
        )
        artifacts = mock.Mock(
            revision="abc123",
            benchmark=Path("/cache/workloads/default/benchmark.json"),
        )
        with mock.patch.object(
            benchmark_engine, "prepare_baseline", return_value=artifacts
        ) as prepare, mock.patch("builtins.print"):
            benchmark_engine.baseline(settings)

        prepare.assert_called_once_with(settings)


if __name__ == "__main__":
    unittest.main()
