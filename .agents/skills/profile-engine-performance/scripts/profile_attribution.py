"""Attribute and compare saved Samply profiles for Penta engine workloads."""

from __future__ import annotations

import argparse
import gzip
import json
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable


class ProfileError(RuntimeError):
    """A saved profile cannot be analyzed reliably."""


@dataclass(frozen=True)
class Frame:
    library: str
    function: str
    file: str | None = None
    line: int | None = None


@dataclass
class Attribution:
    profile_path: Path
    symbols_path: Path | None
    product: str
    threads: list[str]
    duration_ms: float
    total_weight: float
    raw_leaf_libraries: Counter[str] = field(default_factory=Counter)
    raw_leaf_functions: Counter[str] = field(default_factory=Counter)
    owned_self: Counter[str] = field(default_factory=Counter)
    attributed_self: Counter[str] = field(default_factory=Counter)
    inclusive: Counter[str] = field(default_factory=Counter)
    native_attribution: Counter[str] = field(default_factory=Counter)
    system_attribution: dict[str, Counter[str]] = field(
        default_factory=lambda: {
            "allocator": Counter(),
            "memory": Counter(),
            "kernel": Counter(),
        }
    )
    callers: dict[str, Counter[str]] = field(default_factory=dict)
    sites: Counter[tuple[str, str | None, int | None]] = field(
        default_factory=Counter
    )


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        raise ProfileError(f"file not found: {path}")
    opener = gzip.open if path.suffix == ".gz" else open
    try:
        with opener(path, "rt", encoding="utf-8") as source:
            document = json.load(source)
    except (OSError, json.JSONDecodeError) as error:
        raise ProfileError(f"could not read {path}: {error}") from error
    if not isinstance(document, dict):
        raise ProfileError(f"expected a JSON object in {path}")
    return document


def symbol_candidates(profile_path: Path) -> list[Path]:
    candidates: list[Path] = []
    text = str(profile_path)
    if text.endswith(".json.gz"):
        candidates.append(Path(text[: -len(".gz")] + ".syms.json"))
    if text.endswith(".json"):
        candidates.append(Path(text[: -len(".json")] + ".syms.json"))
    candidates.extend(
        [Path(text + ".syms.json"), profile_path.with_suffix(".syms.json")]
    )
    return list(dict.fromkeys(candidates))


def load_symbols(
    profile_path: Path, explicit_path: Path | None
) -> tuple[Path | None, dict[str, Any] | None]:
    if explicit_path is not None:
        return explicit_path, load_json(explicit_path)
    for candidate in symbol_candidates(profile_path):
        if candidate.is_file():
            return candidate, load_json(candidate)
    return None, None


def table_value(table: dict[str, Any], key: str, index: int) -> Any:
    values = table.get(key, [])
    if not isinstance(values, list) or index >= len(values):
        return None
    return values[index]


def string_value(strings: list[Any], index: Any, fallback: str = "unknown") -> str:
    if isinstance(index, int) and 0 <= index < len(strings):
        value = strings[index]
        if isinstance(value, str):
            return value
    return fallback


def normalized_binary_name(value: str) -> str:
    name = Path(value).name
    for suffix in (".dylib", ".so", ".dll", ".exe"):
        if name.lower().endswith(suffix):
            name = name[: -len(suffix)]
            break
    return re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")


class SymbolResolver:
    def __init__(
        self, profile: dict[str, Any], symbols: dict[str, Any] | None
    ) -> None:
        self.libs = profile.get("libs", [])
        self.symbol_strings: list[Any] = []
        self.by_identity: dict[tuple[str, str], dict[int, dict[str, Any]]] = {}
        self.by_name: dict[str, dict[int, dict[str, Any]]] = {}
        if symbols is None:
            return

        self.symbol_strings = symbols.get("string_table", [])
        for item in symbols.get("data", []):
            if not isinstance(item, dict):
                continue
            name = str(item.get("debug_name", "unknown"))
            code_id = str(item.get("code_id", "")).upper()
            table = item.get("symbol_table", [])
            addresses: dict[int, dict[str, Any]] = {}
            for pair in item.get("known_addresses", []):
                if not isinstance(pair, list) or len(pair) != 2:
                    continue
                address, symbol_index = pair
                if not isinstance(address, int) or not isinstance(symbol_index, int):
                    continue
                if not isinstance(table, list) or not 0 <= symbol_index < len(table):
                    continue
                entry = table[symbol_index]
                if isinstance(entry, dict):
                    addresses[address] = entry
            self.by_identity[(name, code_id)] = addresses
            self.by_name[name] = addresses

    def library_record(
        self, thread: dict[str, Any], resource_index: Any
    ) -> dict[str, Any]:
        if not isinstance(resource_index, int):
            return {}
        resource_table = thread.get("resourceTable", {})
        lib_index = table_value(resource_table, "lib", resource_index)
        if isinstance(lib_index, int) and 0 <= lib_index < len(self.libs):
            record = self.libs[lib_index]
            if isinstance(record, dict):
                return record
        name_index = table_value(resource_table, "name", resource_index)
        return {"debugName": string_value(thread.get("stringArray", []), name_index)}

    def resolve(self, thread: dict[str, Any], frame_index: int) -> list[Frame]:
        frame_table = thread.get("frameTable", {})
        func_table = thread.get("funcTable", {})
        strings = thread.get("stringArray", [])
        function_index = table_value(frame_table, "func", frame_index)
        if not isinstance(function_index, int):
            return [Frame("unknown", "unknown")]

        resource_index = table_value(func_table, "resource", function_index)
        library = self.library_record(thread, resource_index)
        library_name = str(
            library.get("debugName") or library.get("name") or "unknown"
        )
        raw_name = string_value(
            strings, table_value(func_table, "name", function_index), "unknown"
        )
        file_name = string_value(
            strings, table_value(func_table, "fileName", function_index), ""
        ) or None
        line = table_value(frame_table, "line", frame_index)
        if line is None:
            line = table_value(func_table, "lineNumber", function_index)

        code_id = str(library.get("codeId", "")).upper()
        address = table_value(frame_table, "address", frame_index)
        address_map = self.by_identity.get((library_name, code_id))
        if address_map is None:
            address_map = self.by_name.get(library_name, {})
        entry = address_map.get(address) if isinstance(address, int) else None
        if entry is None:
            return [Frame(library_name, raw_name, file_name, line)]

        expanded = entry.get("frames")
        if isinstance(expanded, list) and expanded:
            frames = []
            for item in expanded:
                if not isinstance(item, dict):
                    continue
                frames.append(
                    Frame(
                        library_name,
                        string_value(self.symbol_strings, item.get("function")),
                        string_value(
                            self.symbol_strings, item.get("file"), ""
                        )
                        or None,
                        item.get("line") if isinstance(item.get("line"), int) else None,
                    )
                )
            if frames:
                return frames

        symbol = entry.get("symbol")
        return [
            Frame(
                library_name,
                string_value(self.symbol_strings, symbol, raw_name),
                file_name,
                line,
            )
        ]


def has_inline_application_symbols(profile: dict[str, Any], product: str) -> bool:
    prefixes = application_prefixes(product)
    for thread in profile.get("threads", []):
        if not isinstance(thread, dict):
            continue
        strings = thread.get("stringArray", [])
        for index in thread.get("funcTable", {}).get("name", []):
            name = string_value(strings, index, "")
            if name.startswith(prefixes):
                return True
    return False


def application_prefixes(product: str) -> tuple[str, ...]:
    namespace = normalized_binary_name(product)
    return (
        "penta::",
        "<penta::",
        f"{namespace}::",
        f"<{namespace}::",
    )


def is_application_function(function: str, product: str) -> bool:
    return function.startswith(application_prefixes(product))


def is_owned_library(library: str, product: str) -> bool:
    return normalized_binary_name(library) == normalized_binary_name(product)


def system_category(library: str, function: str) -> str | None:
    lowered_library = library.lower()
    lowered_function = function.lower()
    if any(
        token in lowered_library
        for token in ("malloc", "jemalloc", "mimalloc", "snmalloc")
    ) or any(
        token in lowered_function
        for token in ("malloc", "calloc", "realloc", "__rdl_alloc", "__rdl_dealloc")
    ):
        return "allocator"
    if "libsystem_platform" in lowered_library or any(
        token in lowered_function for token in ("memcpy", "memmove", "memset", "bcopy")
    ):
        return "memory"
    if "kernel" in lowered_library or lowered_function in {
        "mach_absolute_time",
        "clock_gettime",
    }:
        return "kernel"
    return None


def select_threads(profile: dict[str, Any], product: str) -> list[dict[str, Any]]:
    candidates = [
        thread
        for thread in profile.get("threads", [])
        if isinstance(thread, dict)
        and int(thread.get("samples", {}).get("length", 0) or 0) > 0
    ]
    product_name = normalized_binary_name(product)
    selected = [
        thread
        for thread in candidates
        if normalized_binary_name(str(thread.get("processName", ""))) == product_name
        or normalized_binary_name(str(thread.get("name", ""))) == product_name
    ]
    if not selected:
        selected = [thread for thread in candidates if thread.get("isMainThread")]
    if not selected:
        selected = candidates
    if not selected:
        raise ProfileError("profile contains no sampled threads")
    return selected


def sample_weights(samples: dict[str, Any]) -> Iterable[tuple[Any, float, Any]]:
    stacks = samples.get("stack", [])
    weights = samples.get("weight")
    times = samples.get("time", [])
    for index, stack in enumerate(stacks):
        weight = 1.0
        if isinstance(weights, list) and index < len(weights) and weights[index] is not None:
            weight = float(weights[index])
        time = times[index] if index < len(times) else None
        yield stack, weight, time


def analyze(
    profile_path: Path,
    symbols_path: Path | None,
    focuses: list[str],
) -> Attribution:
    profile = load_json(profile_path)
    product = str(profile.get("meta", {}).get("product") or "penta-match")
    resolved_symbols_path, symbols = load_symbols(profile_path, symbols_path)
    if (
        symbols is None
        and not profile.get("meta", {}).get("symbolicated", False)
        and not has_inline_application_symbols(profile, product)
    ):
        candidates = ", ".join(str(path) for path in symbol_candidates(profile_path))
        raise ProfileError(
            "profile is not symbolicated and no symbol sidecar was found; "
            f"keep Samply's sidecar beside the capture or pass --symbols (looked for {candidates})"
        )

    threads = select_threads(profile, product)
    resolver = SymbolResolver(profile, symbols)
    result = Attribution(
        profile_path=profile_path,
        symbols_path=resolved_symbols_path,
        product=product,
        threads=[str(thread.get("name") or thread.get("tid") or "unknown") for thread in threads],
        duration_ms=0.0,
        total_weight=0.0,
        callers={focus: Counter() for focus in focuses},
    )
    observed_times: list[float] = []

    for thread in threads:
        stack_table = thread.get("stackTable", {})
        stack_frames = stack_table.get("frame", [])
        stack_prefixes = stack_table.get("prefix", [])
        for stack_index, weight, sample_time in sample_weights(thread.get("samples", {})):
            result.total_weight += weight
            if isinstance(sample_time, (int, float)):
                observed_times.append(float(sample_time))
            if not isinstance(stack_index, int):
                result.raw_leaf_libraries["unknown"] += weight
                result.raw_leaf_functions["unknown :: unknown"] += weight
                continue

            frames: list[Frame] = []
            current: int | None = stack_index
            first_physical = True
            leaf_library = "unknown"
            leaf_function = "unknown"
            while isinstance(current, int):
                if not 0 <= current < len(stack_frames):
                    raise ProfileError(f"invalid stack index {current} in {profile_path}")
                frame_index = stack_frames[current]
                if not isinstance(frame_index, int):
                    raise ProfileError(f"invalid frame index at stack {current} in {profile_path}")
                resolved = resolver.resolve(thread, frame_index)
                if first_physical:
                    leaf_library = resolved[0].library
                    leaf_function = resolved[0].function
                    result.raw_leaf_libraries[leaf_library] += weight
                    result.raw_leaf_functions[
                        f"{leaf_library} :: {leaf_function}"
                    ] += weight
                    first_physical = False
                frames.extend(resolved)
                current = stack_prefixes[current] if current < len(stack_prefixes) else None

            owned_frames = [
                frame for frame in frames if is_owned_library(frame.library, product)
            ]
            if owned_frames:
                result.owned_self[owned_frames[0].function] += weight

            application_frames = [
                frame
                for frame in frames
                if is_application_function(frame.function, product)
            ]
            if not application_frames:
                continue
            nearest = application_frames[0]
            result.attributed_self[nearest.function] += weight
            result.sites[(nearest.function, nearest.file, nearest.line)] += weight
            if not is_owned_library(leaf_library, product):
                result.native_attribution[
                    f"{leaf_library} -> {nearest.function}"
                ] += weight
            category = system_category(leaf_library, leaf_function)
            if category is not None:
                result.system_attribution[category][nearest.function] += weight

            application_functions = [frame.function for frame in application_frames]
            for function in set(application_functions):
                result.inclusive[function] += weight
            for focus, callers in result.callers.items():
                matching_indexes = [
                    index
                    for index, function in enumerate(application_functions)
                    if focus.lower() in function.lower()
                ]
                target_index = next(
                    (
                        index
                        for index in matching_indexes
                        if application_functions[index].lower().endswith(focus.lower())
                    ),
                    matching_indexes[0] if matching_indexes else None,
                )
                if target_index is None:
                    continue
                target = application_functions[target_index]
                caller = next(
                    (
                        function
                        for function in application_functions[target_index + 1 :]
                        if function != target
                    ),
                    "<root>",
                )
                callers[caller] += weight

    if result.total_weight <= 0:
        raise ProfileError("profile contains no positive sample weight")
    if observed_times:
        result.duration_ms = max(observed_times) - min(observed_times)
    if not result.attributed_self:
        raise ProfileError(
            "symbols were loaded, but no Penta application frames could be attributed"
        )
    return result


def entries(counter: Counter[str], total: float, limit: int | None = None) -> list[dict[str, Any]]:
    values = counter.most_common(limit)
    return [
        {"name": name, "weight": weight, "share_percent": 100.0 * weight / total}
        for name, weight in values
    ]


def attribution_json(result: Attribution, limit: int) -> dict[str, Any]:
    return {
        "profile": str(result.profile_path),
        "symbols": str(result.symbols_path) if result.symbols_path else None,
        "product": result.product,
        "threads": result.threads,
        "duration_ms": result.duration_ms,
        "sample_weight": result.total_weight,
        "raw_leaf_libraries": entries(result.raw_leaf_libraries, result.total_weight, limit),
        "raw_leaf_functions": entries(result.raw_leaf_functions, result.total_weight, limit),
        "owned_self": entries(result.owned_self, result.total_weight, limit),
        "attributed_self": entries(result.attributed_self, result.total_weight, limit),
        "inclusive": entries(result.inclusive, result.total_weight, limit),
        "native_attribution": entries(result.native_attribution, result.total_weight, limit),
        "system_attribution": {
            category: entries(counter, result.total_weight, limit)
            for category, counter in result.system_attribution.items()
        },
        "callers": {
            focus: entries(counter, result.total_weight, limit)
            for focus, counter in result.callers.items()
        },
        "sites": [
            {
                "function": function,
                "file": file_name,
                "line": line,
                "weight": weight,
                "share_percent": 100.0 * weight / result.total_weight,
            }
            for (function, file_name, line), weight in result.sites.most_common(limit)
        ],
    }


def format_weight(weight: float) -> str:
    return f"{weight:9.0f}" if weight.is_integer() else f"{weight:9.2f}"


def print_counter(title: str, counter: Counter[str], total: float, limit: int) -> None:
    print(f"\n{title}")
    if not counter:
        print("  (no matching samples)")
        return
    for name, weight in counter.most_common(limit):
        print(f"{format_weight(float(weight))} {100.0 * weight / total:7.2f}%  {name}")


def print_summary(result: Attribution, limit: int) -> None:
    print(f"PROFILE {result.profile_path}")
    print(f"symbols={result.symbols_path or 'embedded'}")
    print(f"product={result.product} threads={','.join(result.threads)}")
    print(
        f"sample_weight={format_weight(result.total_weight).strip()} "
        f"duration_ms={result.duration_ms:.3f}"
    )
    print_counter("RAW LEAF LIBRARIES", result.raw_leaf_libraries, result.total_weight, limit)
    print_counter("RAW LEAF FUNCTIONS", result.raw_leaf_functions, result.total_weight, limit)
    print_counter(
        "ATTRIBUTED APPLICATION SELF", result.attributed_self, result.total_weight, limit
    )
    print_counter("APPLICATION INCLUSIVE", result.inclusive, result.total_weight, limit)
    print_counter(
        "NATIVE LEAF -> APPLICATION ATTRIBUTION",
        result.native_attribution,
        result.total_weight,
        limit,
    )
    for category, counter in result.system_attribution.items():
        print_counter(
            f"{category.upper()} LEAVES ATTRIBUTED TO APPLICATION",
            counter,
            result.total_weight,
            limit,
        )
    for focus, counter in result.callers.items():
        print_counter(
            f"IMMEDIATE APPLICATION CALLERS MATCHING {focus!r}",
            counter,
            result.total_weight,
            limit,
        )
    print("\nTOP APPLICATION SITES")
    for (function, file_name, line), weight in result.sites.most_common(limit):
        location = ""
        if file_name:
            location = f"  {file_name}"
            if line is not None:
                location += f":{line}"
        print(
            f"{format_weight(float(weight))} "
            f"{100.0 * weight / result.total_weight:7.2f}%  {function}{location}"
        )


def delta_entries(
    before: Counter[str],
    after: Counter[str],
    before_total: float,
    after_total: float,
) -> list[dict[str, Any]]:
    values = []
    for name in before.keys() | after.keys():
        before_weight = float(before[name])
        after_weight = float(after[name])
        before_share = 100.0 * before_weight / before_total
        after_share = 100.0 * after_weight / after_total
        values.append(
            {
                "name": name,
                "before_weight": before_weight,
                "after_weight": after_weight,
                "weight_delta": after_weight - before_weight,
                "before_share_percent": before_share,
                "after_share_percent": after_share,
                "share_delta_points": after_share - before_share,
            }
        )
    values.sort(
        key=lambda item: (
            abs(item["weight_delta"]),
            abs(item["share_delta_points"]),
            item["name"],
        ),
        reverse=True,
    )
    return values


def percent_change(before: float, after: float) -> float | None:
    if before == 0:
        return None
    return 100.0 * (after - before) / before


def comparison_json(
    before: Attribution, after: Attribution, limit: int
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "before": attribution_json(before, limit),
        "after": attribution_json(after, limit),
        "delta": {
            "duration_ms": after.duration_ms - before.duration_ms,
            "duration_percent": percent_change(before.duration_ms, after.duration_ms),
            "sample_weight": after.total_weight - before.total_weight,
            "sample_weight_percent": percent_change(
                before.total_weight, after.total_weight
            ),
            "raw_leaf_libraries": delta_entries(
                before.raw_leaf_libraries,
                after.raw_leaf_libraries,
                before.total_weight,
                after.total_weight,
            )[:limit],
            "raw_leaf_functions": delta_entries(
                before.raw_leaf_functions,
                after.raw_leaf_functions,
                before.total_weight,
                after.total_weight,
            )[:limit],
            "attributed_self": delta_entries(
                before.attributed_self,
                after.attributed_self,
                before.total_weight,
                after.total_weight,
            )[:limit],
            "inclusive": delta_entries(
                before.inclusive,
                after.inclusive,
                before.total_weight,
                after.total_weight,
            )[:limit],
        },
    }


def format_percent_change(value: float | None) -> str:
    return "n/a" if value is None else f"{value:+.2f}%"


def print_delta_table(title: str, values: list[dict[str, Any]], limit: int) -> None:
    print(f"\n{title}")
    if not values:
        print("  (no matching samples)")
        return
    for item in values[:limit]:
        print(
            f"{format_weight(item['before_weight'])} -> "
            f"{format_weight(item['after_weight'])}  "
            f"{item['weight_delta']:+9.0f}  "
            f"{item['share_delta_points']:+7.2f}pp  {item['name']}"
        )


def print_comparison(before: Attribution, after: Attribution, limit: int) -> None:
    print(f"COMPARISON {before.profile_path} -> {after.profile_path}")
    print(
        f"duration_ms={before.duration_ms:.3f} -> {after.duration_ms:.3f} "
        f"({after.duration_ms - before.duration_ms:+.3f}, "
        f"{format_percent_change(percent_change(before.duration_ms, after.duration_ms))})"
    )
    print(
        f"sample_weight={format_weight(before.total_weight).strip()} -> "
        f"{format_weight(after.total_weight).strip()} "
        f"({after.total_weight - before.total_weight:+.0f}, "
        f"{format_percent_change(percent_change(before.total_weight, after.total_weight))})"
    )
    tables = (
        ("RAW LEAF LIBRARY DELTAS", before.raw_leaf_libraries, after.raw_leaf_libraries),
        ("RAW LEAF FUNCTION DELTAS", before.raw_leaf_functions, after.raw_leaf_functions),
        ("ATTRIBUTED SELF DELTAS", before.attributed_self, after.attributed_self),
        ("INCLUSIVE DELTAS", before.inclusive, after.inclusive),
    )
    for title, before_counter, after_counter in tables:
        print_delta_table(
            title,
            delta_entries(
                before_counter,
                after_counter,
                before.total_weight,
                after.total_weight,
            ),
            limit,
        )
    print_counter(
        "CURRENT ATTRIBUTED APPLICATION SELF",
        after.attributed_self,
        after.total_weight,
        limit,
    )
    print_counter(
        "CURRENT APPLICATION INCLUSIVE", after.inclusive, after.total_weight, limit
    )


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be positive")
    return parsed


def add_common_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--top", type=positive_int, default=15, help="rows per table")
    parser.add_argument(
        "--caller-of",
        action="append",
        default=[],
        metavar="SUBSTRING",
        help="attribute immediate application callers; repeatable",
    )
    parser.add_argument("--json", action="store_true", help="emit stable JSON")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Attribute and compare saved Penta Samply profiles."
    )
    commands = parser.add_subparsers(dest="command", required=True)
    summary = commands.add_parser("summary", help="summarize one saved profile")
    summary.add_argument("profile", type=Path)
    summary.add_argument("--symbols", type=Path, help="explicit Samply symbol sidecar")
    add_common_arguments(summary)

    compare = commands.add_parser("compare", help="compare identical before/after workloads")
    compare.add_argument("before", type=Path)
    compare.add_argument("after", type=Path)
    compare.add_argument("--before-symbols", type=Path)
    compare.add_argument("--after-symbols", type=Path)
    add_common_arguments(compare)
    return parser


def run(arguments: argparse.Namespace) -> None:
    if arguments.command == "summary":
        result = analyze(arguments.profile, arguments.symbols, arguments.caller_of)
        if arguments.json:
            print(
                json.dumps(
                    {"schema_version": 1, **attribution_json(result, arguments.top)},
                    indent=2,
                    sort_keys=True,
                )
            )
        else:
            print_summary(result, arguments.top)
        return

    before = analyze(arguments.before, arguments.before_symbols, arguments.caller_of)
    after = analyze(arguments.after, arguments.after_symbols, arguments.caller_of)
    if arguments.json:
        print(json.dumps(comparison_json(before, after, arguments.top), indent=2, sort_keys=True))
    else:
        print_comparison(before, after, arguments.top)


def main() -> int:
    try:
        run(build_parser().parse_args())
    except (ProfileError, KeyError, TypeError, ValueError) as error:
        print(f"profile_attribution.py: error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
