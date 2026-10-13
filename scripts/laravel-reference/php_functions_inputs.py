#!/usr/bin/env python3
"""Writes tests/fixtures/php-functions/inputs.json, the strings
php-functions.php answers for: hand-picked cases plus random strings over
the characters PHP's strip_tags state machine reacts to. Deterministic.

    python3 -I scripts/laravel-reference/php_functions_inputs.py
    scripts/laravel-reference/run.sh php scripts/laravel-reference/php-functions.php \\
        < tests/fixtures/php-functions/inputs.json > tests/fixtures/php-functions/answers.json
"""

import base64
import json
import os
import random

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.path.join(REPO, "tests/fixtures/php-functions")

PICKED = [
    "", "plain", "  spaced  ", "\t\n\r\x0b\x0c\x00 mixed \x00\n", "\u00a0nbsp\u00a0", "\u0085nel\u0085",
    "\u1680ogham\u1680", "\u180emongolian\u180e", "\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a",
    "\u2028line\u2029", "\u202fnarrow\u205f", "\u3000ideographic\u3000", "\ufeffbom\ufeff", "\u200bzero\u200b",
    "\u200eltr\u200e", "\x1cfs\x1f", "<b>bold</b>", "a < b", "a <b", "a<", "<", ">", "a > b", "<<b>>", "<a href=\"x>y\">t</a>",
    "<a title='it''s'>t</a>", "<!-- c -->after", "<!-- c > still -->after", "<!--->x-->y", "<?php echo 1; ?>after",
    "<?xml version=\"1.0\"?>after", "<!DOCTYPE html>after", "<!doctype>x", "<x\ny>z", "<\nb>z", "<b\x00>z", "a\x00b",
    "<font size=10>to</font>", "</font>", "Hits</font>", "&lt;TICK&gt;", "x<y>z<w", "\"<b>\"", "'<b>'",
    "<b \"unclosed>x", "<b 'q'>x</b>", "<b>\"</b>tail", "<a <b> c> d", "<a >> b", "<<<", ">>>", "<!a>b", "<!-x>y",
    "<?x>y", "<? a ? > b ?>c", "<?(>)?>d", "x <? \"?>\" ?> y", "<!--a--b-->c", "<!--a-- >c-->d", "<script>a<b</script>",
    "123", "0", "00", "000123", "4294967295", "4294967296", "9223372036854775807", "9223372036854775808",
    "99999999999999999999999", "my-fight.txt", "combat.txt", "Kämpfe.txt", "fight.TXT", "fight.Log", "fight.log.txt",
    "noext", ".txt", "a.txt.", "C:\\fakepath\\fight.txt", "dir/sub/fight.txt", "dir\\fight.log", "with space.txt",
    "quote\"name.txt", "semi;colon.txt", "per%cent.txt", "tab\tname.txt", "emoji🚀.txt", "日本語.txt", "Ωmega.log",
    "naïve café.txt", "Ærø.txt", "ß.txt", "ǅ.txt", "łódź.txt", "Привет.txt", "ÿ.txt", "™.txt", "x.TxT", "x.LOG",
    "x.txt ", " x.txt", "x.tx", "x.pdf", "x.", ".", "..", "-", "_", "a'b.txt", "a`b.txt", "{x}.txt", "[x].txt",
    "(x).txt", "a,b.txt", "a=b.txt", "a@b.txt", "a#b.txt", "a!b.txt", "a&b.txt", "a+b.txt", "a~b.txt", "a^b.txt",
    "a|b.txt", "a*b.txt", "a?b.txt", "a<b>.txt", "a\x7fb.txt", "dpsDealt, neutOut", "a,,b", " , ", "0,1", " logiDealt ",
]
ALPHABET = list("<>!?-\"'\\ ab/=x\n\t\x00()lmLMeEpytcdo[]") + ["\u00a0", "ü"]


def main():
    rng = random.Random(3287248)
    strings = list(PICKED)
    for _ in range(600):
        strings.append("".join(rng.choice(ALPHABET) for _ in range(rng.randrange(1, 24))))
    os.makedirs(OUT, exist_ok=True)
    with open(os.path.join(OUT, "inputs.json"), "w", encoding="utf-8") as handle:
        json.dump([base64.b64encode(s.encode("utf-8")).decode() for s in strings], handle, indent=1)
        handle.write("\n")
    print(f"{len(strings)} inputs")


main()
