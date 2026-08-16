#!/usr/bin/env bash
# Runs the deferred simulation sweeps -- the #[ignore] tier -- and, when one
# fails, prints the narrowest command that reproduces it.
#
# These play whole games and take minutes, which is why they are not in the
# per-push gate. That makes the failure report the important part: whoever
# reads it is looking at a nightly log, disconnected from the change that
# broke it, and their first question is how to get the failure back locally
# without waiting out the whole sweep. So each target runs separately, and a
# failure names the exact test and the one command that reruns just it.
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root" || exit 1

# label|cargo target selector
targets=(
  "engine library|-p penta --lib"
  "engine integration|-p penta --test engine"
  "policy|-p penta --test policy"
)

failed_tests=()
failed_any=0

for entry in "${targets[@]}"; do
  label="${entry%%|*}"
  selector="${entry#*|}"

  printf '\n== %s ==\n' "$label"
  output_file="$(mktemp)"

  # shellcheck disable=SC2086 # selector is a deliberate multi-word argument list.
  if cargo test --locked --profile quick-test $selector -- --ignored 2>&1 |
    tee "$output_file"; then
    rm -f "$output_file"
    continue
  fi

  failed_any=1
  # cargo lists each failure on its own indented line under "failures:".
  while IFS= read -r test_name; do
    [ -n "$test_name" ] && failed_tests+=("$test_name")
  done < <(
    awk '/^failures:$/ { collecting = 1; next }
         /^test result:/ { collecting = 0 }
         collecting && /^    [^ ]/ { print $1 }' "$output_file" | sort -u
  )
  rm -f "$output_file"
done

if [ "$failed_any" -eq 0 ]; then
  printf '\nAll deferred sweeps passed.\n'
  exit 0
fi

printf '\n%s\n' "----------------------------------------------------------------"
printf 'The nightly sweep failed. To reproduce locally, run just the\n'
printf 'failing test rather than the whole sweep:\n\n'

if [ "${#failed_tests[@]}" -eq 0 ]; then
  # A target can fail without naming a test: a panic outside a test, or a
  # build error. Say so instead of printing an empty list.
  printf '  make test-rust-slow\n\n'
  printf 'No individual test was named above, so the failure is likely a\n'
  printf 'build error or a panic outside a test body. Read the target log.\n'
else
  for test_name in "${failed_tests[@]}"; do
    printf '  make test-rust-slow FILTER=%s\n' "$test_name"
  done
  printf '\nEach command compiles once and then runs only that test.\n'
fi

printf '%s\n' "----------------------------------------------------------------"
exit 1
