#!/usr/bin/env python3
"""End-to-end key-binding smoke test for beeswax.

Launches the real binary in a pseudo-terminal, sends keystrokes, and checks the
resulting model by reading the saved .bwx file. Checking the save file rather
than the screen keeps the assertions about behaviour instead of pixels.

    python3 scripts/keys_smoke.py [--bin path/to/beeswax] [-v]

Exits 0 if every check passes, 1 otherwise.

Two things make this harness work, and both are easy to get wrong:

1. beeswax blocks at startup until the terminal answers crossterm's
   `supports_keyboard_enhancement()` probe. It writes `CSI ? u`, then `CSI c`,
   and waits. A harness that never replies captures a blank screen and looks
   like a broken binding rather than a stalled process. Reply with:
     * `\\x1b[?1;2c`             -> plain-terminal path. Alt+<key> arrives as a
                                    raw ESC prefix; Ctrl+H/Ctrl+J collapse to
                                    Backspace/Enter.
     * `\\x1b[?1u\\x1b[?1;2c`     -> enhanced path. Keys go as CSI-u, e.g.
                                    `\\x1b[111;4u` for Alt+Shift+o. Both replies
                                    are needed; the `?1u` alone is not enough.

2. Several dialogs have multi-stage confirms that are tedious to drive, so
   fixtures are built by editing the save file directly: a 9-byte header
   (b"BWX\\0" + u32 LE schema version + plain/encrypted flag) then JSON.
"""

import argparse
import fcntl
import json
import os
import pty
import re
import signal
import struct
import sys
import tempfile
import termios
import threading
import time

SCHEMA_VERSION = 3
REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

ENTER = "\r"
ESC = "\x1b"
TAB = "\t"
F3 = "\x1b[13~"
F6 = "\x1b[17~"
INS = "\x1b[2~"
DEL = "\x1b[3~"
DOWN = "\x1b[B"
F8 = "\x1b[19~"
F9 = "\x1b[20~"
CTRL_S = "\x13"
GG = ["g", "g"]


def alt(ch):
    """Alt+<ch> as a plain terminal sends it: ESC prefix."""
    return ESC + ch


def csi_u(code, mods):
    """A key in the enhanced (Kitty) encoding. mods = 1 + shift(1) + alt(2) + ctrl(4)."""
    return "\x1b[%d;%du" % (code, mods)


class Beeswax:
    def __init__(self, binary):
        self.binary = binary

    def run(self, path, keys, enhanced=False, keep=False, delay=0.10, boot=0.5):
        """Send `keys`, save, and kill. Returns the raw terminal output."""
        if os.path.exists(path) and not keep:
            os.remove(path)
        pid, fd = pty.fork()
        if pid == 0:                                    # child
            os.environ["TERM"] = "xterm-256color"
            try:
                os.execv(self.binary, [self.binary, path])
            except OSError as exc:                      # pragma: no cover
                os.write(2, str(exc).encode())
                os._exit(1)
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 140, 0, 0))

        chunks = []
        stop = threading.Event()

        def reader():
            while not stop.is_set():
                try:
                    data = os.read(fd, 65536)
                except OSError:
                    break
                if not data:
                    break
                chunks.append(data)

        thread = threading.Thread(target=reader, daemon=True)
        thread.start()
        time.sleep(0.3)
        os.write(fd, b"\x1b[?1u\x1b[?1;2c" if enhanced else b"\x1b[?1;2c")
        time.sleep(boot)
        for key in keys:
            os.write(fd, key.encode() if isinstance(key, str) else key)
            time.sleep(delay)
        os.write(fd, CTRL_S.encode())
        time.sleep(0.4)
        stop.set()
        os.kill(pid, signal.SIGKILL)
        os.waitpid(pid, 0)
        try:
            os.close(fd)
        except OSError:
            pass
        return b"".join(chunks)


ANSI = re.compile(
    r"\x1b\[[0-9;?]*[a-zA-Z]|\x1b\][^\x07]*\x07|\x1b[=>()][0-9A-Za-z]?|\x1b[a-zA-Z]")


def screen(data, rows=40, cols=140):
    """Replay the terminal writes into a character grid.

    ratatui only emits the cells that changed, so the visible screen is the
    accumulation of every write in order rather than any single frame.
    """
    text = data.decode("utf-8", "replace")
    grid = [[" "] * cols for _ in range(rows)]
    row = col = 0
    i = 0
    while i < len(text):
        char = text[i]
        if char == "\x1b":
            match = ANSI.match(text, i)
            if match:
                token = match.group(0)
                if token.endswith("H"):          # CSI row;col H -- cursor move
                    parts = token[2:-1].split(";")
                    row = (int(parts[0]) - 1) if parts[0] else 0
                    col = (int(parts[1]) - 1) if len(parts) > 1 and parts[1] else 0
                i = match.end()
                continue
            i += 1
            continue
        if char == "\r":
            col = 0
        elif char == "\n":
            row += 1
            col = 0
        else:
            if 0 <= row < rows and 0 <= col < cols:
                grid[row][col] = char
            col += 1
        i += 1
    return "\n".join("".join(r).rstrip() for r in grid)


def read_save(path):
    raw = open(path, "rb").read()
    if raw[:4] != b"BWX\0":
        raise AssertionError("not a .bwx file: %r" % raw[:16])
    return json.loads(raw[9:].decode("utf-8"))


def write_save(path, data):
    with open(path, "wb") as handle:
        handle.write(b"BWX\0" + struct.pack("<I", SCHEMA_VERSION) + b"\x00")
        handle.write(json.dumps(data).encode("utf-8"))


def flat_categories(cats, depth=0, acc=None):
    """The category tree as indented lines, in Category Manager order."""
    if acc is None:
        acc = []
    for cat in cats:
        acc.append("  " * depth + cat["name"])
        flat_categories(cat.get("children", []), depth + 1, acc)
    return acc


class Fixtures:
    """Save files with extra views/sections, built by editing JSON directly."""

    def __init__(self, app, workdir):
        self.app = app
        self.workdir = workdir
        self._base = None

    def base(self):
        if self._base is None:
            path = os.path.join(self.workdir, "_base.bwx")
            self.app.run(path, [])
            self._base = read_save(path)
        return json.loads(json.dumps(self._base))

    def make(self, path, views=1, sections=1):
        data = self.base()
        next_id = data["next_id"]
        view = data["views"][0]
        template = view["sections"][0]
        for i in range(1, sections):
            section = json.loads(json.dumps(template))
            section["id"] = next_id
            next_id += 1
            section["name"] = "Section%d" % (i + 1)
            view["sections"].append(section)
        for i in range(1, views):
            extra = json.loads(json.dumps(view))
            extra["id"] = next_id
            next_id += 1
            extra["name"] = "View%d" % (i + 1)
            data["views"].append(extra)
        data["next_id"] = next_id
        write_save(path, data)
        return data


class Checks:
    def __init__(self, verbose=False):
        self.passed = 0
        self.failed = []
        self.verbose = verbose

    def section(self, name):
        print("\n%s" % name)

    def check(self, label, actual, expected):
        ok = actual == expected
        if ok:
            self.passed += 1
        else:
            self.failed.append(label)
        print("  %-44s %s" % (label, "PASS" if ok else "FAIL"))
        if not ok or self.verbose:
            print("      expected: %r" % (expected,))
            print("      actual:   %r" % (actual,))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin", default=os.path.join(REPO, "target", "debug", "beeswax"))
    parser.add_argument("-v", "--verbose", action="store_true")
    args = parser.parse_args()

    if not os.path.exists(args.bin):
        sys.exit("binary not found: %s\nrun `cargo build` first" % args.bin)

    app = Beeswax(args.bin)
    checks = Checks(args.verbose)
    workdir = tempfile.mkdtemp(prefix="beeswax-keys-")
    fixtures = Fixtures(app, workdir)
    path = lambda name: os.path.join(workdir, name)

    def cats(keys, enhanced=False):
        target = path("cats.bwx")
        app.run(target, keys, enhanced=enhanced)
        return flat_categories(read_save(target)["categories"])

    # The default tree is MAIN > (Entry, When, Done, Initial Section); `gg`
    # puts the cursor on MAIN, so one `j` reaches Entry and two reach When.
    checks.section("Category Manager -- create (vi o/O, as in VimOutliner)")
    checks.check("o  -> sibling below",
                 cats([F9] + GG + ["j", "j", "o"] + list("ZZ") + [ENTER])[3], "  ZZ")
    checks.check("O  -> sibling above",
                 cats([F9] + GG + ["j", "O"] + list("ZZ") + [ENTER])[1], "  ZZ")
    checks.check("o on a root -> first child",
                 cats([F9] + GG + ["o"] + list("ZZ") + [ENTER])[1], "  ZZ")
    checks.check("O on a root -> first child too",
                 cats([F9] + GG + ["O"] + list("ZZ") + [ENTER])[1], "  ZZ")

    checks.section("Category Manager -- level and order")
    checks.check(">> demote", cats([F9] + GG + ["j", "j", ">", ">"])[2], "    When")
    checks.check("<< after >> round trips",
                 cats([F9] + GG + ["j", "j", ">", ">", "<", "<"])[2], "  When")
    checks.check("Alt+l demote", cats([F9] + GG + ["j", "j", alt("l")])[2], "    When")
    checks.check("Alt+h after Alt+l round trips",
                 cats([F9] + GG + ["j", "j", alt("l"), alt("h")])[2], "  When")
    checks.check("Alt+Right demote",
                 cats([F9] + GG + ["j", "j", "\x1b[1;3C"])[2], "    When")
    checks.check("Alt+j move down", cats([F9] + GG + ["j", alt("j")])[1], "  When")
    checks.check("Alt+k move up", cats([F9] + GG + ["j", "j", alt("k")])[1], "  When")
    checks.check("Alt+k on the first sibling is inert",
                 cats([F9] + GG + ["j", alt("k")])[1], "  Entry")

    # Terminals disagree on how a shifted letter is reported: the shifted
    # codepoint, the base codepoint plus a SHIFT flag, or base:shifted. All
    # three have to reach the same binding.
    checks.section("Category Manager -- shift encodings for O")
    for label, key in [("shifted code  (CSI 79;2u)", csi_u(79, 2)),
                       ("base + SHIFT  (CSI 111;2u)", csi_u(111, 2)),
                       ("base:shifted  (CSI 111:79;2u)", "\x1b[111:79;2u")]:
        checks.check(label,
                     cats([F9] + GG + ["j", key] + list("ZZ") + [ENTER], enhanced=True)[1],
                     "  ZZ")

    checks.section("View screen -- sections and columns")

    def insert_field(keys, enhanced=False):
        out = app.run(path("view.bwx"), keys, enhanced=enhanced)
        for line in screen(out).split("\n"):
            if "Insert:" in line:
                return line.split("Insert:")[1].split()[0]
        return "NO DIALOG"

    def titles(keys):
        """Dialog frame titles currently on screen."""
        text = screen(app.run(path("view.bwx"), keys))
        found = []
        for name in ("Section Add", "Choose Category", "Column Add", "Choices"):
            if name in text:
                found.append(name)
        return found

    checks.check("Alt+o opens Section Add + its category picker",
                 titles([alt("o")]), ["Section Add", "Choose Category"])
    checks.check("Alt+c opens Column Add + its head picker",
                 titles([alt("c")]), ["Column Add", "Choices"])
    # Esc closes the picker back to the form rather than cancelling the dialog,
    # which is also what makes the Insert field readable again.
    checks.check("Alt+o then Esc leaves Insert on Below",
                 insert_field([alt("o"), ESC]), "Below")
    checks.check("Alt+o then Esc,Down,Right sets Insert=Above",
                 insert_field([alt("o"), ESC, "\x1b[B", "\x1b[C"]), "Above")

    def columns(keys):
        target = path("cols.bwx")
        # Alt+c -> Column Add with the head picker open: pick a category, then
        # Enter to confirm the dialog.
        add = ([alt("c"), "j", ENTER, ENTER] + [alt("c"), "j", "j", ENTER, ENTER])
        app.run(target, add + keys)
        return [col["name"] for col in read_save(target)["views"][0]["columns"]]

    checks.check("Alt+c adds columns", columns([]), ["Entry", "When"])
    checks.check("Alt+h moves a column left",
                 columns(["l", "l", alt("h")]), ["When", "Entry"])
    checks.check("Alt+Left moves a column left",
                 columns(["l", "l", "\x1b[1;3D"]), ["When", "Entry"])
    checks.check("Alt+l moves a column right",
                 columns(["h", alt("l")]), ["When", "Entry"])
    checks.check("Alt+Right moves a column right",
                 columns(["h", "\x1b[1;3C"]), ["When", "Entry"])

    # `O` anchors the cursor one row up so confirm() inserts before it. Backing
    # out of the create has to undo that anchor, on both exit paths, or the
    # cursor creeps upward every time.
    checks.section("View screen -- O leaves the cursor where it was")

    THREE = (["o"] + list("AAA") + [ENTER] + ["o"] + list("BBB") + [ENTER]
             + ["o"] + list("CCC") + [ENTER])

    def cursor_after(keys):
        target = path("above.bwx")
        app.run(target, THREE + keys)
        view = read_save(target)["views"][0]
        return (view["cursor_section"], view["cursor_item"])

    def items_after(keys):
        target = path("above.bwx")
        app.run(target, THREE + keys)
        return [i["text"] for i in read_save(target)["items"]]

    checks.check("k puts the cursor on the middle item", cursor_after(["k"]), (0, 1))
    checks.check("O then Esc restores it", cursor_after(["k", "O", ESC]), (0, 1))
    checks.check("O then Enter on an empty buffer restores it",
                 cursor_after(["k", "O", ENTER]), (0, 1))
    # The first item anchors on the section head rather than an item.
    checks.check("O then Esc restores it on the first item",
                 cursor_after(["k", "k", "O", ESC]), (0, 0))
    # Confirming still inserts above and lands on the new item.
    checks.check("O then text inserts above",
                 items_after(["k", "O"] + list("NEW") + [ENTER]),
                 ["AAA", "NEW", "BBB", "CCC"])
    checks.check("O then text leaves the cursor on the new item",
                 cursor_after(["k", "O"] + list("NEW") + [ENTER]), (0, 1))

    # The column-head picker is the Choices overlay on the Column Add form. It
    # carries the affordances the retired quick-add picker had: search, inline
    # create, rename, properties, delete.
    checks.section("Column-head picker (Alt+c, or F3 on the form)")

    def picker(keys, enhanced=False):
        return screen(app.run(path("pick.bwx"), keys, enhanced=enhanced))

    def picker_cats(keys):
        target = path("pick.bwx")
        app.run(target, keys)
        return flat_categories(read_save(target)["categories"])

    checks.check("offers the category list",
                 "Select category for column head" in picker([alt("c")]), True)
    checks.check("/ opens a search prompt",
                 "Search for: Wh" in picker([alt("c"), "/"] + list("Wh")), True)
    checks.check("Ins creates a category inline",
                 "  TMP" in picker_cats([alt("c"), "G", INS] + list("TMP") + [ENTER]), True)
    checks.check("Del prompts for a deletable category",
                 "Discard Category?" in picker(
                     [alt("c"), "G", INS] + list("TMP") + [ENTER, DEL]), True)
    # Deleting the current section's head category, or a protected one, would
    # pull the view out from under the cursor, so Del is inert there.
    checks.check("Del inert on the section's head category",
                 "Discard Category?" in picker([alt("c"), "G", DEL]), False)
    checks.check("Del inert on a protected category",
                 "Discard Category?" in picker([alt("c"), "j", DEL]), False)
    checks.check("Esc returns to the form, not out of the dialog",
                 "Column Add" in picker([alt("c"), ESC]), True)
    checks.check("F3 on the Position field opens the position chooser",
                 "Right of current column" in picker([alt("c"), ESC, DOWN, F3]), True)

    checks.section("View Manager -- reorder")

    def views(keys):
        target = path("views.bwx")
        fixtures.make(target, views=3)
        app.run(target, [F8] + keys, keep=True)
        return [view["name"] for view in read_save(target)["views"]]

    checks.check("baseline", views([]), ["Initial View", "View2", "View3"])
    checks.check("Alt+j move down", views([alt("j")]),
                 ["View2", "Initial View", "View3"])
    checks.check("Alt+Down move down", views(["\x1b[1;3B"]),
                 ["View2", "Initial View", "View3"])
    checks.check("Alt+j then Alt+k round trips",
                 views([alt("j"), alt("k")]), ["Initial View", "View2", "View3"])
    checks.check("Ctrl+Down still works (legacy alias)",
                 views(["\x1b[1;5B"]), ["View2", "Initial View", "View3"])

    # vmgr_props_sec_move_* is inert unless the Sections field is active, which
    # is one Tab from the top of the View Properties dialog.
    checks.section("View Properties -- reorder sections")

    def sections(keys):
        target = path("secs.bwx")
        fixtures.make(target, sections=3)
        app.run(target, [F8, F6, TAB] + keys + [ENTER], keep=True)
        return [sec["name"] for sec in read_save(target)["views"][0]["sections"]]

    checks.check("baseline", sections([]),
                 ["Initial Section", "Section2", "Section3"])
    checks.check("Alt+j move down", sections([alt("j")]),
                 ["Section2", "Initial Section", "Section3"])
    checks.check("Alt+Down move down", sections(["\x1b[1;3B"]),
                 ["Section2", "Initial Section", "Section3"])
    checks.check("Alt+j then Alt+k round trips", sections([alt("j"), alt("k")]),
                 ["Initial Section", "Section2", "Section3"])

    total = checks.passed + len(checks.failed)
    print("\n%d/%d passed" % (checks.passed, total))
    if checks.failed:
        print("failed: %s" % ", ".join(checks.failed))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
