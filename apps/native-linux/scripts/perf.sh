#!/usr/bin/env bash
# Time Mdow for Linux starting up, opening documents and being used: scrolling, jumping, find,
# zoom, a theme change and switching tabs, over four documents (a README, a long plan full of
# code, one with diagrams and formulas, and a few megabytes of everything).
#
#   scripts/perf.sh                      # build, then run without a display
#   scripts/perf.sh --display            # run on this desktop instead: windows will open
#   scripts/perf.sh --runs 5             # medians over 5 runs of each (default 3)
#   scripts/perf.sh --against old-binary # the same runs for another build, side by side
#   BIN=path/to/mdow-linux scripts/perf.sh
#
# Without a display it runs on GTK's Broadway backend with a headless Chromium (or Chrome) as
# its client. That draws through the CPU and paces frames through the browser, so read "main
# thread" there: the CPU time Mdow's main thread spent between a request and the frame showing
# its result, which is the same on any display. "on screen" is the wall-clock time for the same
# thing and "scroll frame" gaps are only worth reading with --display.
#
# The app logs these itself: MDOW_PERF=1 for timings as you use it, MDOW_PERF_RUN=scroll,find,…
# to run scenarios and quit. This script only starts it and takes medians.
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
ROOT_DIR="$(cd "$APP_DIR/../.." && pwd -P)"
RUNS=3
DISPLAY_MODE=headless
AGAINST=""
while (($#)); do
  case "$1" in
    --display) DISPLAY_MODE=display ;;
    --runs) RUNS="$2"; shift ;;
    --against) AGAINST="$2"; shift ;;
    -h | --help) sed -n '2,19p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "perf.sh: unknown option $1" >&2; exit 2 ;;
  esac
  shift
done

if [[ -z "${BIN:-}" ]]; then
  cargo build --release --locked --manifest-path "$APP_DIR/Cargo.toml"
  BIN="${CARGO_TARGET_DIR:-$APP_DIR/target}/release/mdow-linux"
fi
[[ -x "$BIN" ]] || { echo "perf.sh: $BIN is not a program" >&2; exit 1; }
[[ -z "$AGAINST" || -x "$AGAINST" ]] || { echo "perf.sh: $AGAINST is not a program" >&2; exit 1; }

WORK="$(mktemp -d "${TMPDIR:-/tmp}/mdow-perf.XXXXXX")"
PIDS=()
cleanup() {
  for pid in "${PIDS[@]}"; do
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  done
  # The browser's helpers may still be writing their profile.
  rm -rf "$WORK" 2>/dev/null || { sleep 1; rm -rf "$WORK" 2>/dev/null || true; }
}
trap cleanup EXIT

# The documents. Real ones from this repository, so they hold what people actually write.
DOCS="$WORK/docs"
mkdir -p "$DOCS"
cp "$APP_DIR/README.md" "$DOCS/readme.md"
mapfile -t ALL < <(find "$ROOT_DIR/docs" "$ROOT_DIR/apps" -name '*.md' \
  -not -path '*/node_modules/*' -not -path '*/target/*' -not -path '*/.build/*' | sort)
PLAN="$(ls -S "${ALL[@]}" | head -1)"
cp "$PLAN" "$DOCS/plan.md"
for _ in 1 2 3 4 5; do cat "${ALL[@]}"; done >"$DOCS/big.md"
for i in $(seq 1 12); do
  cat <<EOF
# Mixed content $i

A paragraph with **bold**, *emphasis*, \`inline code\`, a [link](https://example.com) and a
footnote[^n$i]. Inline math \$e^{i\\pi} + $i = 0\$ sits in the text, and more after it:
\$\\sum_{k=$i}^{n} k = \\frac{n(n+1)}{2}\$.

> [!NOTE]
> An alert with a list:
> - one
> - two

\`\`\`mermaid
graph TD
  A[Open file $i] --> B{Markdown?}
  B -- yes --> C[Parse]
  B -- no --> D[Convert HTML]
  C --> E[Rows]
  D --> E
\`\`\`

\$\$
\\int_{-\\infty}^{\\infty} e^{-x^{$((i + 1))}}\\,dx = \\sqrt{\\pi}
\$\$

| Feature | Linux | Mac | Notes |
| :------ | :---: | :-: | ----: |
| Tabs | yes | yes | in the header |
| Mermaid | yes | yes | no browser |
| Math | yes | yes | typeset natively |

- [x] Parse
- [ ] Render
  - [ ] Nested, with \`code\`

\`\`\`rust
fn main() {
    for i in 0..$i {
        println!("hello {i}"); // comment
    }
}
\`\`\`

\`\`\`typescript
export async function load(path: string): Promise<Doc> {
  return parse(await fs.readFile(path, 'utf8'))
}
\`\`\`

[^n$i]: Note $i.

EOF
done >"$DOCS/mixed.md"

# Somewhere to draw.
ENV=()
if [[ "$DISPLAY_MODE" == headless ]]; then
  BROWSER=""
  for name in chromium chromium-browser google-chrome-stable google-chrome; do
    if command -v "$name" >/dev/null; then BROWSER="$name"; break; fi
  done
  command -v gtk4-broadwayd >/dev/null && [[ -n "$BROWSER" ]] || {
    echo "perf.sh: running without a display needs gtk4-broadwayd and Chromium or Chrome;" >&2
    echo "         or pass --display to use this desktop" >&2
    exit 1
  }
  N=40
  while [[ -e "${XDG_RUNTIME_DIR:-/run/user/$UID}/broadway$((N + 1)).socket" ]]; do N=$((N + 1)); done
  gtk4-broadwayd ":$N" >"$WORK/broadwayd.log" 2>&1 &
  PIDS+=($!)
  sleep 0.5
  # A page in the background or behind an extension's tab stops answering, and frames with it.
  "$BROWSER" --headless=new --ozone-platform=headless --disable-gpu --disable-extensions \
    --no-first-run --disable-background-timer-throttling --disable-renderer-backgrounding \
    --disable-backgrounding-occluded-windows --user-data-dir="$WORK/browser" \
    --window-size=1400,900 "http://127.0.0.1:$((8080 + N))/" >"$WORK/browser.log" 2>&1 &
  PIDS+=($!)
  sleep 3
  ENV=(GDK_BACKEND=broadway "BROADWAY_DISPLAY=:$N")
fi

# One run: a fresh settings file, so nothing is restored and the user's own are left alone.
# MDOW_OMARCHY is passed through if set (off: Mdow's own look rather than the Omarchy theme).
run() { # binary, scenarios, documents…
  local bin="$1" scenarios="$2"
  shift 2
  local state="$WORK/state-$RANDOM.json"
  env "${ENV[@]}" MDOW_STATE="$state" MDOW_PERF_EPOCH="$(date +%s.%N)" \
    MDOW_PERF_RUN="$scenarios" timeout 180 "$bin" "$@" 2>&1 | grep -E '^MDOW(PERF|BENCH) ' || true
  rm -f "$state"
}

RESULTS="$WORK/results.tsv"
BUILDS=("$BIN")
[[ -n "$AGAINST" ]] && BUILDS=("$AGAINST" "$BIN")
for index in "${!BUILDS[@]}"; do
  for doc in readme plan mixed big; do
    for ((i = 0; i < RUNS; i++)); do
      printf '.' >&2
      run "${BUILDS[$index]}" scroll,jump,find,zoom,theme "$DOCS/$doc.md" |
        sed "s/^/$index\t$doc.md\t/" >>"$RESULTS"
    done
  done
  for ((i = 0; i < RUNS; i++)); do
    printf '.' >&2
    run "${BUILDS[$index]}" tabs "$DOCS/readme.md" "$DOCS/plan.md" "$DOCS/mixed.md" "$DOCS/big.md" |
      grep -E 'switch tab|memory' | sed "s/^/$index\tfour tabs\t/" >>"$RESULTS"
  done
done
echo >&2

awk -F'\t' -v builds="${#BUILDS[@]}" '
function add(metric, value,    key) {
  key = $1 SUBSEP $2 SUBSEP metric
  values[key, ++count[key]] = value + 0
  if (!((($2) SUBSEP metric) in seen)) { seen[$2, metric] = 1; order[$2, ++metrics[$2]] = metric }
  if (!($2 in docs)) { docs[$2] = 1; doc_order[++n_docs] = $2 }
}
function median(key,    n, i, j, t, v) {
  n = count[key]
  if (!n) return "-"
  for (i = 1; i <= n; i++) v[i] = values[key, i]
  for (i = 2; i <= n; i++) { t = v[i]; for (j = i - 1; j >= 1 && v[j] > t; j--) v[j + 1] = v[j]; v[j + 1] = t }
  return sprintf("%.1f", n % 2 ? v[(n + 1) / 2] : (v[n / 2] + v[n / 2 + 1]) / 2)
}
{
  line = $3
  if (match(line, /^MDOWPERF startup: exec to main [0-9.]+ms/)) { split(line, f, " "); add("launch to main()", f[6]) }
  else if (match(line, /^MDOWPERF startup: first frame /)) { sub(/.*\(at /, "", line); add("first frame, since main()", line) }
  else if (match(line, /^MDOWPERF read and parse /)) { n = split(line, f, " "); add("read and parse (off the main thread)", f[n]) }
  else if (match(line, /^MDOWPERF memory /)) { split(line, f, " "); add("memory, resident MB", f[3]); add("memory, private MB", f[5]) }
  else if (match(line, /^MDOWBENCH scroll /)) {
    n = split(line, f, " ")
    for (i = 1; i <= n; i++) {
      if (f[i] == "work") { sub(/p50=/, "", f[i + 1]); sub(/p95=/, "", f[i + 2]); sub(/max=/, "", f[i + 4])
        add("scroll frame, main thread: median", f[i + 1]); add("scroll frame, main thread: p95", f[i + 2]); add("scroll frame, main thread: worst", f[i + 4]) }
      if (f[i] == "gap") { sub(/p95=/, "", f[i + 2]); add("scroll frame gap: p95", f[i + 2]) }
    }
  }
  else if (match(line, / on screen [0-9.]+ms, main thread [0-9.]+ms/)) {
    label = line; sub(/^MDOWPERF /, "", label); sub(/ on screen .*/, "", label)
    rest = substr(line, RSTART); split(rest, f, " ")
    if (label ~ /^open /) { add("open: on screen", f[3]); add("open: main thread", f[6]) }
    else if (label == "code highlighted") add("code coloured: on screen", f[3])
    else if (label != "settle" && label != "scroll") add(label ": main thread", f[6])
  }
}
END {
  for (d = 1; d <= n_docs; d++) {
    doc = doc_order[d]
    if (builds > 1) printf "\n%-42s %10s %10s\n", doc, "against", "this build"
    else printf "\n%-42s %10s\n", doc, "ms"
    for (m = 1; m <= metrics[doc]; m++) {
      metric = order[doc, m]
      printf "  %-40s", metric
      for (b = 0; b < builds; b++) printf " %10s", median(b SUBSEP doc SUBSEP metric)
      printf "\n"
    }
  }
}' "$RESULTS"
