#!/usr/bin/env python3
"""Records the PHP parser's answers (scripts/laravel-reference/parse.php in
the reference checkout) as the expected outputs of the Rust parser:

- tests/fixtures/parser/<name>.expected.json for every <name>.txt,
- tests/fixtures/parser/generated/expected.json for the generated corpus.

    scripts/laravel-reference/run.sh setup   (once)
    python3 -I scripts/laravel-reference/parser_expected.py

Never rerun it to make a failing Rust test pass: the answers are the spec.
"""

import base64
import json
import os
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
PARSER = os.path.join(REPO, "tests/fixtures/parser")


def php_parse(inputs):
    result = subprocess.run(
        [os.path.join(HERE, "run.sh"), "php", os.path.join(HERE, "parse.php")],
        input=json.dumps(inputs).encode(), capture_output=True, check=True,
    )
    return json.loads(result.stdout)


def dump(path, value):
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, ensure_ascii=False)
        handle.write("\n")


files = sorted(name[:-4] for name in os.listdir(PARSER) if name.endswith(".txt"))
inputs = {}
for name in files:
    with open(os.path.join(PARSER, f"{name}.txt"), "rb") as handle:
        inputs[name] = base64.b64encode(handle.read()).decode()
for name, answer in php_parse(inputs).items():
    dump(os.path.join(PARSER, f"{name}.expected.json"), answer)

with open(os.path.join(PARSER, "generated/inputs.json"), encoding="utf-8") as handle:
    generated = json.load(handle)
dump(os.path.join(PARSER, "generated/expected.json"), php_parse(generated))
print(f"{len(files)} fixtures and {len(generated)} generated logs")
