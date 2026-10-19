#!/usr/bin/env bash
# Preparation only: invoke with bash, once, after integration is settled.
set -uo pipefail
cd /Users/indo/code/project/omon-gateway-wt/dori-work || exit 2
export LC_ALL=C RUSTC_WRAPPER= CARGO_TARGET_DIR="$PWD/target-w4"
export CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true CARGO_TERM_COLOR=never
E="$PWD/.omo/evidence/dori-grade-agent-env"
BASE=/Users/indo/code/project/omon-gateway-wt/dori-grade-agent-env/.omo/evidence/dori-grade-agent-env/task-2-failing.txt
# Persistent claim prevents both concurrent and accidental second full passes.
mkdir "$E/final-verify.claim" || { printf '%s\n' 'Final pass already claimed; do not rerun.' >&2; exit 2; }
export TMPDIR
TMPDIR=$(mktemp -d /tmp/dori-final-verify.XXXXXX) || exit 2
printf '%s\n' "$TMPDIR" > "$E/final-verify-tmpdir.txt"
# Cheap local readiness gate; never starts or interrupts another cargo process.
until [[ -f "$E/final-integration.md" ]] && ! pgrep -f "target-w4" >/dev/null; do sleep 1; done
[[ -r "$BASE" ]] || { printf 'Missing baseline: %s\n' "$BASE" >&2; exit 2; }
cp "$BASE" "$E/final-verify-baseline.txt" || exit 2
# The helper only writes evidence, never source. TMPDIR is retained, even on failure.
python3 - "$E" <<'PY'
import hashlib, os, pathlib, sys
root = pathlib.Path.cwd()
paths = [p for d in ('src', 'tests', 'migrations', 'bot_profiles') for p in (root / d).rglob('*') if p.is_file()]
paths += [root / 'Cargo.toml', root / 'Cargo.lock']
paths.sort(key=lambda p: os.fsencode(str(p.relative_to(root))))
with open(pathlib.Path(sys.argv[1]) / 'final-verify-pre.sha256', 'w') as out:
    for p in paths:
        out.write(hashlib.sha256(p.read_bytes()).hexdigest() + '  ' + str(p.relative_to(root)) + '\n')
PY
pre=$?
[[ $pre == 0 ]] || exit "$pre"
# cargo-fmt has no -j/--offline flags; equivalent environment is set above.
cargo fmt --all -- --check > "$E/final-verify-fmt.log" 2>&1
fmt=$?
cargo clippy --offline --locked -j4 --all-targets --all-features -- -D warnings > "$E/final-verify-clippy.log" 2>&1
clippy=$?
# One invocation, no filters, no reruns. Default test selection includes doctests.
cargo test --offline --locked -j4 --workspace --all-features --no-fail-fast -- --format pretty > "$E/final-verify-tests.log" 2>&1
tests=$?
build=null; post=null; source_cmp=null; copy=null; binary_sha=null; binary_path=null
if [[ $fmt == 0 && $clippy == 0 ]]; then
    cargo build --offline --locked -j4 --bin omo-gateway > "$E/final-verify-build.log" 2>&1
    build=$?
fi
# Always capture post state to expose edits during even a failed batch.
python3 - "$E" <<'PY'
import hashlib, os, pathlib, sys
root = pathlib.Path.cwd()
paths = [p for d in ('src', 'tests', 'migrations', 'bot_profiles') for p in (root / d).rglob('*') if p.is_file()]
paths += [root / 'Cargo.toml', root / 'Cargo.lock']
paths.sort(key=lambda p: os.fsencode(str(p.relative_to(root))))
with open(pathlib.Path(sys.argv[1]) / 'final-verify-post.sha256', 'w') as out:
    for p in paths:
        out.write(hashlib.sha256(p.read_bytes()).hexdigest() + '  ' + str(p.relative_to(root)) + '\n')
PY
post=$?
if [[ $post == 0 ]]; then
    cmp "$E/final-verify-pre.sha256" "$E/final-verify-post.sha256" > "$E/final-verify-cmp.log" 2>&1
    source_cmp=$?
fi
if [[ $build == 0 && $source_cmp == 0 ]]; then
    binary_sha=$(shasum -a 256 "$CARGO_TARGET_DIR/debug/omo-gateway")
    hash=$?
    if [[ $hash == 0 ]]; then
        binary_sha=${binary_sha%% *}
        binary_path="$CARGO_TARGET_DIR/release-bins/$binary_sha/omo-gateway"
        mkdir -p "$(dirname "$binary_path")" && cp -p "$CARGO_TARGET_DIR/debug/omo-gateway" "$binary_path"
        copy=$?
    else
        copy=$hash; binary_sha=null
    fi
fi
python3 - "$E" "$pre" "$fmt" "$clippy" "$tests" "$build" "$post" "$source_cmp" "$copy" "$binary_path" "$binary_sha" "$TMPDIR" <<'PY'
import hashlib, json, pathlib, re, sys
E = pathlib.Path(sys.argv[1])
keys = ['pre_manifest', 'fmt', 'clippy', 'tests', 'build', 'post_manifest', 'source_cmp', 'binary_copy']
codes = dict(zip(keys, [None if s == 'null' else int(s) for s in sys.argv[2:10]]))
backend = {'test_omo_backend_e2e_thread_lifecycle_and_streaming', 'test_omo_backend_aborts_turn_on_repeated_approval_denials', 'test_omo_backend_deadline_interrupts_looping_turn'}
baseline = {}
for line in (E / 'final-verify-baseline.txt').read_text().splitlines():
    source, name = line.split(': ', 1)
    baseline.setdefault(pathlib.Path(source).stem, set()).add(name.strip())
baseline['test_omo_backend'] = backend
results = {}; current = None; chunks = {}; errors = []
for line in (E / 'final-verify-tests.log').read_text(errors='replace').splitlines():
    m = re.match(r'\s*Running (?:unittests )?(\S+) \(', line)
    doc = re.match(r'\s*Doc-tests (\S+)', line)
    if m:
        source = m[1]
        current = pathlib.Path(source).stem if source.startswith('tests/') else 'bin:omo-gateway' if source == 'src/entry.rs' else 'lib:omon_gateway'
        results[current] = {'source': source, 'passed': None, 'failed': None, 'ignored': None, 'failure_names': []}
        chunks[current] = []
    elif doc:
        current = 'doc:' + doc[1]
        results[current] = {'source': doc[1], 'passed': None, 'failed': None, 'ignored': None, 'failure_names': []}
        chunks[current] = []
    if current:
        chunks[current].append(line)
        summary = re.search(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', line)
        if summary:
            r = results[current]
            # Last summary is the parent harness; child helper totals are not added.
            r.update(status=summary[1], passed=int(summary[2]), failed=int(summary[3]), ignored=int(summary[4]))
            r['failure_names'] = sorted(set(re.findall(r'^test (\S+) \.\.\. FAILED$', '\n'.join(chunks[current]), re.M)))
            # Doctest labels include source paths and line numbers.
            if current.startswith('doc:'):
                r['failure_names'] = sorted(set(re.findall(r'^test (.+) \.\.\. FAILED$', '\n'.join(chunks[current]), re.M)))
expected = {'lib:omon_gateway', 'bin:omo-gateway'}
expected.update(p.stem for p in pathlib.Path('tests').glob('*.rs'))
manifest = pathlib.Path('Cargo.toml').read_text()
for section in re.split(r'(?m)^\[\[test\]\]\s*$', manifest)[1:]:
    name = re.search(r'(?m)^name\s*=\s*"([^"]+)"', section.split('[', 1)[0])
    if name:
        expected.add(name[1])
for target in sorted(expected - results.keys()):
    errors.append('Missing test target: ' + target)
for target, r in results.items():
    log = E / ('final-verify-target-' + re.sub(r'[^A-Za-z0-9_.-]', '_', target) + '.log')
    log.write_text('\n'.join(chunks[target]) + '\n')
    r['log'] = str(log)
    if r['passed'] is None or r['failed'] != len(r['failure_names']):
        errors.append('Incomplete or ambiguous result: ' + target)
new = [{'target': t, 'name': n} for t, r in results.items() for n in r['failure_names'] if n not in baseline.get(t, set())]
comparison = 0 if not errors and not new else 1
codes['baseline_comparison'] = comparison
if codes['tests'] not in (0, 101) or (codes['tests'] != 0 and not any(r['failure_names'] for r in results.values())):
    errors.append('Cargo test failed without a complete named-test failure verdict')
comparison = codes['baseline_comparison'] = 0 if not errors and not new else 1
profile = json.loads(pathlib.Path('bot_profiles/omomeow.json').read_text())
prompt = profile['system_prompt']
codes['todo20_sentinels'] = 0 if '[LANE_EVENT]' in prompt and 'omo-gateway lane start' in prompt else 1
codes['todo29_sentinel'] = 0 if 'omo-gateway sessions list' in prompt else 1
def artifact(path):
    return {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()} if path.is_file() else None
binary = pathlib.Path(sys.argv[10]) if sys.argv[10] != 'null' else None
binary_info = artifact(binary) if binary else None
if binary_info and binary_info['sha256'] != sys.argv[11]:
    errors.append('Copied binary hash differs from build artifact')
    codes['binary_copy'] = 1
ok = comparison == 0 and all(codes[k] == 0 for k in ('pre_manifest', 'fmt', 'clippy', 'build', 'post_manifest', 'source_cmp', 'binary_copy', 'todo20_sentinels', 'todo29_sentinel'))
codes['final'] = 0 if ok else 1
report = {'status': 'baseline-bounded-pass' if ok else 'failed', 'exit_codes': codes,
          'test_command': 'cargo test --offline --locked -j4 --workspace --all-features --no-fail-fast -- --format pretty',
          'targets': results, 'new_failures': new, 'report_errors': errors,
          'backend_canonical_baseline': sorted(backend), 'baseline': artifact(E / 'final-verify-baseline.txt'),
          'binary': binary_info, 'pre_manifest': artifact(E / 'final-verify-pre.sha256'),
          'post_manifest': artifact(E / 'final-verify-post.sha256'), 'tmpdir': sys.argv[12],
          'tmpdir_retained': True, 'integration_receipt': artifact(E / 'final-integration.md')}
(E / 'final-verify.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'status': report['status'], 'new_failures': new, 'report_errors': errors}))
sys.exit(codes['final'])
PY
exit "$?"

