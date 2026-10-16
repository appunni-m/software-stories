#!/usr/bin/env python3
"""Check Codex's native staged-source boundary without an account/model request."""
import argparse
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--codex', required=True, type=Path, help='Installed Codex executable')
args = parser.parse_args()
with tempfile.TemporaryDirectory(prefix='code-atlas-permissions-') as directory:
    root = Path(directory).resolve()
    stage = root / 'stage'
    stage.mkdir()
    source = stage / 'source.txt'
    source.write_text('synthetic source\n')
    outside = root / 'outside.txt'
    outside.write_text('synthetic outside boundary\n')
    command = [str(args.codex.resolve()), 'sandbox', '-P', 'code_atlas_explainer_v1', '-C', str(stage),
        '-c', 'permissions.code_atlas_explainer_v1.filesystem={":minimal"="read",":workspace_roots"="read"}',
        '-c', 'permissions.code_atlas_explainer_v1.network.enabled=false', '--']
    for name, operation, allowed in [
        ('staged source read', ['/bin/cat', str(source)], True),
        ('outside source read', ['/bin/cat', str(outside)], False),
        ('staged source write', ['/usr/bin/touch', str(stage / 'created.txt')], False),
    ]:
        result = subprocess.run(command + operation, capture_output=True, timeout=20, check=False)
        if (result.returncode == 0) != allowed:
            raise SystemExit(f'FAIL: {name} did not enforce the expected boundary')
        print(f'PASS: {name} {"allowed" if allowed else "denied"}')
    if source.read_text() != 'synthetic source\n' or (stage / 'created.txt').exists():
        raise SystemExit('FAIL: staged content changed')
