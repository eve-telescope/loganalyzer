#!/usr/bin/env python3
"""Writes the synthetic parser fixtures: the edge cases
(tests/fixtures/parser/edge-*.txt) and the generated corpus
(tests/fixtures/parser/generated/inputs.json), many small logs with the
line formats varied in spacing, markup, numbers and names. Deterministic
(fixed seed), so running it again writes the same files.

    python3 -I scripts/laravel-reference/parser_corpus.py
    python3 -I scripts/laravel-reference/parser_expected.py   (PHP's answers)
"""

import base64
import json
import os
import random

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.path.join(REPO, "tests/fixtures/parser")

HEADER = (
    "------------------------------------------------------------\n"
    "  Gamelog\n"
    "  Listener: {listener}\n"
    "  Session Started: 2026.01.01 12:00:00\n"
    "------------------------------------------------------------\n"
)


def header(listener="Test Pilot"):
    return HEADER.format(listener=listener)


def ts(second, minute=0):
    return f"[ 2026.01.01 12:{minute:02d}:{second:02d} ]"


def hit(second, damage, way, target, weapon, quality="Hits"):
    color = "0xff00ffff" if way == "to" else "0xffcc0000"
    return (
        f"{ts(second)} (combat) <color={color}><b>{damage}</b> <color=0x77ffffff><font size=10>{way}</font> "
        f"<b><color=0xffffffff>{target}</b><font size=10><color=0x77ffffff> - {weapon} - {quality}"
    )


def logi_in_rich(second, amount, ship, pilot, module, kind="shield boosted"):
    return (
        f"{ts(second)} (combat) <color=0xffccff66><b>{amount}</b><color=0x77ffffff><font size=10> remote {kind} by </font>"
        f"<b><color=0xffffffff><font size=10><color=0xFFFF7040><b>{ship}</b></color></font> <font size=10>{pilot}</font></b>"
        f"<color=0x77ffffff><font size=10> - {module}</font>"
    )


def logi_in_plain(second, amount, target, module, kind="shield boosted"):
    return (
        f"{ts(second)} (combat) <color=0xffccff66><b>{amount}</b><color=0x77ffffff><font size=10> remote {kind} by </font>"
        f"<b><color=0xffffffff>{target}</b><color=0x77ffffff><font size=10> - {module}</font>"
    )


def logi_out_rich(second, amount, ship, pilot, corp, module, kind="armor repaired", newline="\n"):
    return (
        f"{ts(second)} (combat) <color=0xffccff66><b>{amount}</b><color=0x77ffffff><font size=10> remote {kind} to </font>"
        f"<b><color=0xffffffff><color=0xff04f300><b><fontsize=12>{ship}{newline}</color><color=0xffffffff></b></fontsize>"
        f"<fontsize=10>{pilot}</color><color=0xff04f300><b>{corp}</b><color=0x77ffffff><font size=10> - {module}</font>"
    )


def logi_out_plain(second, amount, target, module, kind="shield boosted"):
    return (
        f"{ts(second)} (combat) <color=0xffccff66><b>{amount}</b><color=0x77ffffff><font size=10> remote {kind} to </font>"
        f"<b><color=0xffffffff>{target}</b><color=0x77ffffff><font size=10> - {module}</font>"
    )


def neut_rich(second, amount, verb, prep, ship, brackets, module):
    word = f"energy {verb}" + (f" {prep}" if prep else "")
    inner = (
        f"<font size=12><color=0xFFFFFFFF><b>{ship}</b></color></font>"
        + "".join(f"<font size=11> [{b}]</font>" for b in brackets[:-1])
        + f" <font size=11>[{brackets[-1]}] -</font>"
    )
    return (
        f"{ts(second)} (combat) <color=0xff7fffff><b>{amount} GJ</b><color=0x77ffffff><font size=10> {word} </font>"
        f"<b><color=0xffffffff>{inner}</b><color=0x77ffffff><font size=10> - {module}</font>"
    )


def neut_plain(second, amount, verb, prep, ship, module, fontsize=False):
    word = f"energy {verb}" + (f" {prep}" if prep else "")
    target = (
        f"<b><color=0xffFF4500><fontsize=13>{ship}</fontsize></color></b> "
        if fontsize
        else f"<fontsize=12><color=0xFF2ecc71><b>{ship}</b></color></fontsize>"
    )
    return (
        f"{ts(second)} (combat) <color=0xffe57f7f><b>{amount} GJ</b><color=0x77ffffff><font size=10> {word} </font>"
        f"<b><color=0xffffffff>{target}</b><color=0x77ffffff><font size=10> - {module}</font>"
    )


def miss_in(second, weapon, pilot, tail=None):
    return f"{ts(second)} (combat) {weapon} belonging to {pilot} misses you completely - {tail or weapon}"


def miss_out(second, weapon, pilot, tail=None):
    return f"{ts(second)} (combat) Your {weapon} misses {pilot} completely - {tail or weapon}"


def write(name, data):
    path = os.path.join(OUT, f"edge-{name}.txt")
    with open(path, "wb") as handle:
        handle.write(data if isinstance(data, bytes) else data.encode("utf-8"))


def edges():
    basic = "\n".join([
        hit(1, 500, "to", "Enemy Pilot[CORP](Drake)", "Heavy Missile"),
        hit(2, 120, "from", "Enemy Pilot[CORP](Drake)", "Heavy Missile", "Smashes"),
        miss_out(3, "Heavy Missile", "Enemy Pilot"),
    ])
    write("empty", b"")
    write("header-only", header())
    write("header-only-crlf", header().replace("\n", "\r\n"))
    write("no-header", basic + "\n")
    write("invalid-utf8", (header() + basic + "\n").encode() + b"\xff\xfe broken\n")
    write("invalid-utf8-in-header", b"  Listener: Bad \xc3\x28 Name\n" + basic.encode() + b"\n")
    chat = (
        "\n\n---------------------------------------------------------------\n\n"
        "  Channel ID:      local\n  Channel Name:    Local\n  Listener:        Chat Pilot\n"
        "  Session started: 2026.01.01 12:00:00\n"
        "---------------------------------------------------------------\n\n"
        "[ 2026.01.01 12:00:05 ] Chat Pilot > o7\n"
    )
    write("utf16le-chat-bom", b"\xff\xfe" + chat.encode("utf-16-le"))
    write("utf16le-chat-no-bom", chat.encode("utf-16-le"))
    write("utf8-bom", b"\xef\xbb\xbf" + (header() + basic + "\n").encode())
    write("crlf", (header() + basic + "\n").replace("\n", "\r\n"))
    write("cr-only", (header() + basic + "\n").replace("\n", "\r"))
    write("no-final-newline", header() + basic)
    write("mixed-channels", header() + "\n".join([
        f"{ts(0)} (notify) Ship stopping",
        hit(1, 77, "to", "Enemy Pilot[CORP](Drake)", "Heavy Missile"),
        f"{ts(1)} (hint) Attempting to join a channel",
        f"{ts(2)} (question) Are you sure?",
        hit(3, 88, "from", "Enemy Pilot[CORP](Drake)", "Heavy Missile", "Grazes"),
        f"{ts(4)} (combat) Warp scramble attempt from <b>Enemy</b> to <b>you!</b>",
        f"{ts(5)} (combat) Warp disruption attempt from Enemy to you!",
        f"{ts(6)} (combat) <color=0xffffffff>Your target is jammed",
        f"{ts(7)} (None) Undocking from station",
        "[ 2026.01.01 12:00:08 ](combat) missing space after the bracket",
        "[ 2026.1.1 12:00:09 ] (combat) short date",
        hit(10, 99, "to", "Enemy Pilot[CORP](Drake)", "Heavy Missile", "Wrecks"),
    ]) + "\n")
    write("multiline-then-notify", header("Hunter Hughes") + "\n".join([
        logi_out_rich(1, 382, "Crow", "TinkerbeI", "[LPPLS]", "'Peace' Large Remote Armor Repairer"),
        f"{ts(2)} (notify) Your cloak deactivates due to proximity",
        logi_out_rich(3, 401, "Crow", "TinkerbeI", "[LPPLS]", "'Peace' Large Remote Armor Repairer", newline="\r\n"),
        f"{ts(4)} (combat) continuation line",
        "  that runs over two lines - Hits",
    ]) + "\n")
    write("ticker-entity", header() + "\n".join([
        logi_out_rich(1, 300, "Guardian", "Some Pilot", "&lt;TICK&gt;", "Large Remote Armor Repairer II"),
        logi_out_rich(2, 310, "Guardian", "Other Pilot", "[CORP]", "Large Remote Armor Repairer II"),
        logi_out_rich(3, 320, "Guardian", "Third Pilot", "&lt;&gt;", "Large Remote Armor Repairer II"),
        logi_out_rich(4, 330, "Guardian", "Fourth Pilot", "[]", "Large Remote Armor Repairer II"),
    ]) + "\n")
    names = [
        "Pilot [with] brackets",
        "Pilot (paren)",
        "Pilot - dash",
        "Pilot < less",
        "Pilot <b>bold</b>",
        "Zoë Ångström",
        "No Break Space",
        " Leading NBSP",
        "Trailing NBSP ",
        "Tab\tInside",
        "Mongolian᠎",
        "　Ideographic",
        " Em Space",
        "日本語のパイロット",
        "Emoji 🚀 Pilot",
        "Quote \"Pilot\"",
        "Apostrophe's Pilot",
        "a<b c>d",
        "<!-- comment --> Pilot",
        "Pilot <? php ?>",
        "Amp &amp; Pilot",
        "0",
        "00",
    ]
    lines = []
    for index, name in enumerate(names):
        second = index % 60
        lines.append(hit(second, 10 + index, "to", f"{name}[CORP](Rifter)", "Light Missile"))
        lines.append(hit(second, 20 + index, "from", name, "Small Laser"))
        lines.append(miss_out(second, "Light Missile", name))
        lines.append(miss_in(second, "Warrior II", name))
        lines.append(logi_out_plain(second, 30 + index, name, "Small Remote Shield Booster"))
        lines.append(logi_in_rich(second, 40 + index, "Bantam", name, "Small Remote Shield Booster"))
    write("tricky-names", header("Zoë Ångström") + "\n".join(lines) + "\n")
    write("corp-zero", header() + "\n".join([
        hit(1, 10, "to", "Pilot Zero[0](Rifter)", "Light Missile"),
        hit(2, 11, "to", "Pilot Empty[](Rifter)", "Light Missile"),
        hit(3, 12, "to", "Pilot Space[ ](Rifter)", "Light Missile"),
        logi_in_plain(4, 13, "Logi Zero[0](Scythe)", "Medium Remote Shield Booster"),
    ]) + "\n")
    write("gj-signs", header() + "\n".join([
        neut_plain(1, "522", "neutralized", None, "Sabre", "Heavy Energy Neutralizer"),
        neut_plain(2, "-2", "drained", "to", "Stiletto", "Small Energy Nosferatu"),
        neut_plain(3, "+12", "drained", "from", "Stiletto", "Small Energy Nosferatu"),
        neut_plain(4, "40", "neutralized", "to", "Rifter", "Small Energy Neutralizer"),
        neut_plain(5, "+40", "neutralized", None, "Rifter", "Small Energy Neutralizer"),
        neut_plain(6, "-0", "drained", None, "Rifter", "Small Energy Nosferatu"),
        neut_plain(7, "3345", "neutralized", None, "Keepstar", "Standup Energy Neutralization Burst Projector", fontsize=True),
        neut_rich(8, "27", "neutralized", None, "Huginn", ["CVA", "TLOS", "DarkOldMaker"], "Heavy Abyssal Energy Neutralizer"),
        neut_rich(9, "+269", "drained", "from", "Basilisk", ["CVA", "TLOS", "Daniel Grosny"], "Heavy Abyssal Energy Nosferatu"),
        neut_rich(10, "-5", "drained", "to", "Basilisk", ["Solo Pilot"], "Heavy Abyssal Energy Nosferatu"),
        neut_rich(11, "15", "neutralized", None, "Rifter", ["CORP", "Two Brackets"], "Small Energy Neutralizer"),
    ]) + "\n")
    write("huge-numbers", header() + "\n".join([
        hit(1, "4294967295", "to", "Big Pilot[CORP](Titan)", "Doomsday"),
        hit(2, "4294967296", "to", "Big Pilot[CORP](Titan)", "Doomsday"),
        hit(3, "9223372036854775807", "to", "Big Pilot[CORP](Titan)", "Doomsday"),
        hit(4, "99999999999999999999999", "from", "Big Pilot[CORP](Titan)", "Doomsday"),
        hit(5, "000123", "to", "Big Pilot[CORP](Titan)", "Doomsday"),
        neut_plain(6, "+99999999999999999999", "drained", "from", "Titan", "Nosferatu"),
    ]) + "\n")
    long_name = "L" * 300
    write("long-fields", header() + "\n".join([
        hit(1, 10, "to", f"{long_name}[CORP](Rifter)", "Light Missile"),
        hit(2, 10, "to", "Pilot[CORP](Rifter)", "W" * 256),
    ]) + "\n")
    drone = "\n".join([
        # Sole owner by weapon.
        hit(1, 88, "from", "Tom DaNanRen[.BOP](Cenotaph)", "Hammerhead II", "Penetrates"),
        hit(2, 270, "to", "Hammerhead II[.BOP](Hammerhead II)", "Heavy Missile"),
        # Two owners, narrowed by the corp to one.
        hit(3, 90, "from", "Pilot A[AAA](Rupture)", "Warrior II"),
        hit(4, 91, "from", "Pilot B[BBB](Rupture)", "Warrior II"),
        hit(5, 120, "to", "Warrior II[BBB](Warrior II)", "Heavy Missile"),
        # Two owners in the same corp: ambiguous.
        hit(6, 92, "from", "Pilot C[CCC](Rupture)", "Valkyrie II"),
        hit(7, 93, "from", "Pilot D[CCC](Rupture)", "Valkyrie II"),
        hit(8, 121, "to", "Valkyrie II[CCC](Valkyrie II)", "Heavy Missile"),
        # Nobody used the drone, its corp has one pilot.
        hit(9, 94, "from", "Lonely Pilot[LONE](Rifter)", "Small Laser"),
        hit(10, 122, "to", "Acolyte II[LONE](Acolyte II)", "Heavy Missile"),
        # Nobody used the drone, its corp has two pilots.
        hit(11, 95, "from", "Pilot E[TWO](Rifter)", "Small Laser"),
        hit(12, 96, "from", "Pilot F[TWO](Rifter)", "Small Laser"),
        hit(13, 123, "to", "Hornet II[TWO](Hornet II)", "Heavy Missile"),
        # Candidates outside the drone's corp only.
        hit(14, 97, "from", "Pilot G[GGG](Rifter)", "Hobgoblin II"),
        hit(15, 124, "to", "Hobgoblin II[HHH](Hobgoblin II)", "Heavy Missile"),
        # A drone target without a corp.
        hit(16, 125, "to", "Hobgoblin II[](Hobgoblin II)", "Heavy Missile"),
        # Logistics and neut rows never count as drone targets.
        logi_in_plain(17, 50, "Infiltrator II[.BOP](Infiltrator II)", "Remote Repair"),
        neut_plain(18, "30", "neutralized", None, "Infiltrator II", "Small Energy Neutralizer"),
        # A drone named by a later line.
        hit(19, 126, "to", "Ogre II[LATE](Ogre II)", "Heavy Missile"),
        hit(20, 98, "from", "Late Pilot[LATE](Myrmidon)", "Ogre II"),
    ])
    write("drone-attribution", header() + drone + "\n")
    write("numeric-pilot", header() + "\n".join([
        hit(1, 88, "from", "12345[.BOP](Cenotaph)", "Hammerhead II", "Penetrates"),
        hit(2, 270, "to", "Hammerhead II[.BOP](Hammerhead II)", "Heavy Missile"),
    ]) + "\n")
    write("numeric-pilot-ambiguous", header() + "\n".join([
        hit(1, 88, "from", "12345[.BOP](Cenotaph)", "Hammerhead II", "Penetrates"),
        hit(2, 89, "from", "007[.BOP](Cenotaph)", "Hammerhead II", "Penetrates"),
        hit(3, 270, "to", "Hammerhead II[.BOP](Hammerhead II)", "Heavy Missile"),
    ]) + "\n")
    rich_without_ship = (
        f"{ts(1)} (combat) <color=0xff7fffff><b>27 GJ</b><color=0x77ffffff><font size=10> energy neutralized </font>"
        f"<b><color=0xffffffff><font size=12>Huginn</font><font size=11> [TLOS]</font> <font size=11>[DarkOldMaker] -</font></b>"
        f"<color=0x77ffffff><font size=10> - Heavy Abyssal Energy Neutralizer</font>"
    )
    write("rich-neut-without-ship", header() + rich_without_ship + "\n")
    rich_empty_ship = (
        f"{ts(1)} (combat) <color=0xff7fffff><b>27 GJ</b><color=0x77ffffff><font size=10> energy neutralized </font>"
        f"<b><color=0xffffffff><font size=12><b></b></font> <font size=11>[DarkOldMaker] -</font></b>"
        f"<color=0x77ffffff><font size=10> - Heavy Abyssal Energy Neutralizer</font>"
    )
    write("rich-neut-empty-ship", header() + rich_empty_ship + "\n")
    rich_zero_ship = (
        f"{ts(1)} (combat) <color=0xff7fffff><b>27 GJ</b><color=0x77ffffff><font size=10> energy neutralized </font>"
        f"<b><color=0xffffffff><font size=12><b>0</b></font> <font size=11>[Zero Ship Pilot] -</font></b>"
        f"<color=0x77ffffff><font size=10> - Heavy Abyssal Energy Neutralizer</font>"
    )
    write("rich-neut-zero-ship", header() + rich_zero_ship + "\n")
    rich_bracket_in_tag = (
        f"{ts(1)} (combat) <color=0xff7fffff><b>27 GJ</b><color=0x77ffffff><font size=10> energy neutralized </font>"
        f"<b><color=0xffffffff><font size=12><b>Huginn</b></font> <font a=[x]>DarkOldMaker -</font></b>"
        f"<color=0x77ffffff><font size=10> - Heavy Abyssal Energy Neutralizer</font>"
    )
    write("rich-neut-bracket-in-tag", header() + rich_bracket_in_tag + "\n")
    rich_pilot_in_tag = (
        f"{ts(1)} (combat) <color=0xff7fffff><b>27 GJ</b><color=0x77ffffff><font size=10> energy neutralized </font>"
        f"<b><color=0xffffffff><font size=12><b>Huginn</b></font> <x [Hidden Pilot] -</b>"
        f"<color=0x77ffffff><font size=10> - Heavy Abyssal Energy Neutralizer</font>"
    )
    write("rich-neut-pilot-in-tag", header() + rich_pilot_in_tag + "\n")
    write("german-client", header() + "\n".join([
        f"{ts(1)} (combat) <color=0xff00ffff><b>500</b> <color=0x77ffffff><font size=10>an</font> <b><color=0xffffffff>Feind[CORP](Drake)</b><font size=10><color=0x77ffffff> - Schwere Rakete - Trifft",
        f"{ts(2)} (combat) Ihre Schwere Rakete verfehlt Feind völlig - Schwere Rakete",
    ]) + "\n")
    write("header-variants", "\n".join([
        "Listener:",
        "  Listener:   ",
        "\tListener: \t Tabbed Name \t",
        "Session Started:  2026.01.01 00:00:00  ",
        "  Listener: Second Listener",
    ]) + "\n")
    write("header-markup", "  Listener: <b>Bold</b> Name\r\n  Session Started: <i>2026.03.03 03:03:03</i>\r\n")
    write("trailing-markup", header() + "\n".join([
        logi_in_plain(1, 512, "Logi Pilot[CORP](Basilisk)", "Large Remote Shield Booster</font></font>"),
        logi_out_plain(2, 410, "Fleet Mate", "Medium Remote Shield Booster</font> trailing"),
        f"{ts(3)} (combat) <color=0xff00ffff><b>500</b> <color=0x77ffffff><font size=10>to</font> <b><color=0xffffffff>Enemy Pilot[CORP](Drake)</b><font size=10><color=0x77ffffff> - Heavy Missile - Hits</font> - Smashes",
        f"{ts(4)} (combat) <b>12</b> stray <font size=10>to</font> <b><color=0xffffffff>X</b> - - Hits",
    ]) + "\n")


TEMPLATE_NAMES = [
    "Aria Vex", "Korr Malan", "Selene Dur", "Tovak Rin", "Logi Pilot", "Zoë Ångström",
    "Pilot-With-Dash", "Pilot (Alt)", "Name [Tag]", "X", "Hammerhead II", "Warrior II",
    "Space  Double", "Ünïcödé", "Ab<cd", "Ef - Gh", " NBSP", "12345", "Name　Wide",
]
CORPS = ["CORP", "GOON", ".BOP", "-A-", "", "Ün", "A B"]
SHIPS = ["Drake", "Hurricane", "Rupture", "Hammerhead II", "Caracal", "Keepstar", "Crow", "Ship (Navy)", ""]
WEAPONS = ["Heavy Missile", "Hammerhead II", "Warrior II", "720mm Howitzer Artillery II", "Small Laser - II", "Odd<Weapon"]
QUALITIES = ["Hits", "Penetrates", "Glances Off", "Grazes", "Smashes", "Wrecks", "Misses", "hits"]


def target(rng):
    name = rng.choice(TEMPLATE_NAMES)
    shape = rng.random()
    if shape < 0.6:
        return f"{name}[{rng.choice(CORPS)}]({rng.choice(SHIPS) or 'Rifter'})"
    if shape < 0.8:
        return name
    return f"{name}[{rng.choice(CORPS)}]"


def vary(line, rng):
    """Spacing and markup noise in about a third of the lines."""
    roll = rng.random()
    if roll < 0.08:
        return line.replace("</b>", "</b> ", 1)
    if roll < 0.14:
        return line.replace("<font size=10>", "<font size=11>", 1)
    if roll < 0.2:
        return line.replace(" - ", " -  ", 1)
    if roll < 0.24:
        return line + "</font>"
    if roll < 0.28:
        return line + "  "
    if roll < 0.31:
        return line.replace("<b>", "<B>", 1)
    if roll < 0.34:
        return line.replace("(combat)", "(Combat)")
    return line


def random_line(rng, second):
    kind = rng.randrange(11)
    damage = rng.choice([0, 1, 7, 42, 512, 9999, 123456, rng.randrange(1, 3000)])
    if kind == 0:
        return hit(second, damage, rng.choice(["to", "from"]), target(rng), rng.choice(WEAPONS), rng.choice(QUALITIES))
    if kind == 1:
        return logi_in_rich(second, damage, rng.choice(SHIPS) or "Scimitar", rng.choice(TEMPLATE_NAMES), rng.choice(WEAPONS),
                            rng.choice(["shield boosted", "armor repaired", "hull repaired"]))
    if kind == 2:
        return logi_in_plain(second, damage, target(rng), rng.choice(WEAPONS), rng.choice(["shield boosted", "armor repaired"]))
    if kind == 3:
        corp = rng.choice(["[CORP]", "&lt;TICK&gt;", "[A B]", "[]"])
        return logi_out_rich(second, damage, rng.choice(SHIPS) or "Guardian", rng.choice(TEMPLATE_NAMES), corp,
                             rng.choice(WEAPONS), newline=rng.choice(["\n", "\r\n", " "]))
    if kind == 4:
        return logi_out_plain(second, damage, target(rng), rng.choice(WEAPONS), rng.choice(["shield boosted", "hull repaired"]))
    if kind == 5:
        brackets = rng.sample(["CVA", "TLOS", "Pilot One", "Two Words", "Ü"], rng.randrange(1, 4))
        sign = rng.choice(["", "+", "-"])
        verb, prep = rng.choice([("neutralized", None), ("drained", "from"), ("drained", "to"), ("neutralized", "to")])
        return neut_rich(second, f"{sign}{damage}", verb, prep, rng.choice(SHIPS) or "Huginn", brackets, rng.choice(WEAPONS))
    if kind == 6:
        sign = rng.choice(["", "+", "-"])
        verb, prep = rng.choice([("neutralized", None), ("drained", "from"), ("drained", "to"), ("neutralized", "to")])
        return neut_plain(second, f"{sign}{damage}", verb, prep, rng.choice(SHIPS) or "Sabre", rng.choice(WEAPONS),
                          fontsize=rng.random() < 0.3)
    if kind == 7:
        return miss_in(second, rng.choice(WEAPONS), rng.choice(TEMPLATE_NAMES))
    if kind == 8:
        return miss_out(second, rng.choice(WEAPONS), rng.choice(TEMPLATE_NAMES))
    if kind == 9:
        return f"{ts(second)} ({rng.choice(['notify', 'hint', 'question', 'None'])}) Something happened"
    return f"{ts(second)} (combat) Warp scramble attempt from <b>{rng.choice(TEMPLATE_NAMES)}</b> to <b>you!</b>"


def generated():
    rng = random.Random(20261013)
    corpus = {}
    for index in range(400):
        corpus[f"generated-{index:03d}"] = base64.b64encode(random_log(rng).encode()).decode()
    os.makedirs(os.path.join(OUT, "generated"), exist_ok=True)
    with open(os.path.join(OUT, "generated/inputs.json"), "w", encoding="utf-8") as handle:
        json.dump(corpus, handle, indent=1, sort_keys=True)
        handle.write("\n")


def random_log(rng):
    """One generated log: a header and a few varied lines."""
    lines = [vary(random_line(rng, second), rng) for second in range(rng.randrange(1, 9))]
    newline = rng.choice(["\n", "\n", "\n", "\r\n"])
    text = header(rng.choice(TEMPLATE_NAMES)).replace("\n", newline) + newline.join(lines)
    if rng.random() < 0.7:
        text += newline
    return text


if __name__ == "__main__":
    edges()
    generated()
    print("written")
