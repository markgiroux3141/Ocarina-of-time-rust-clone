#!/usr/bin/env python3
"""Builds the decomp name map (ADR 0031): the old decomp commit's names paired with the new
commit's, by address and by offset. Both commits build the same ROM (gc-eu-mq-dbg), so every
function and data symbol sits at the same address in both builds, and every struct field at
the same offset.

    python scripts/name_map.py build --old-elf OLD.elf --new-elf NEW.elf \
        --old-decomp DIR --new-decomp DIR [--out docs/name-map]
    python scripts/name_map.py find NAME [--map docs/name-map/name_map.tsv]

`build` reads:
- each ELF's `.symtab` (functions, globals, the assets' symbols per segment) and its merged
  ECOFF `.mdebug` (IDO keeps `static` functions and data only there);
- each checkout's C (`include/`, `src/`): the structs' `/* 0xNN */` offset comments, the enums.

It writes `<out>/name_map.tsv` (only the names that changed, names and addresses only) and
`<out>/unpaired.tsv` (old names with no partner, and pairings with more than one candidate).
The two ELFs come from building each checkout (`make setup && make` with its own baserom); see
docs/GAME-05-deku-tree.md, milestone 1. Nothing here reads the ROM.

`find` looks a name up both ways (old or new, any kind).
"""

import argparse
import os
import re
import struct
import sys
from collections import defaultdict

# --------------------------------------------------------------------------------------------
# ELF symbols
# --------------------------------------------------------------------------------------------


def _read_elf(path):
    data = open(path, "rb").read()
    if data[:4] != b"\x7fELF" or data[4] != 1 or data[5] != 2:
        sys.exit(f"{path}: not a 32-bit big-endian ELF")
    shoff, = struct.unpack_from(">I", data, 0x20)
    shentsize, shnum, shstrndx = struct.unpack_from(">HHH", data, 0x2E)
    secs = []
    for i in range(shnum):
        name, typ, flags, addr, off, size, link, info, align, entsize = struct.unpack_from(">IIIIIIIIII", data, shoff + i * shentsize)
        secs.append({"name_off": name, "type": typ, "flags": flags, "addr": addr, "off": off, "size": size, "link": link})

    def cstr(off):
        return data[off:data.index(b"\0", off)].decode("latin-1")

    for s in secs:
        s["name"] = cstr(secs[shstrndx]["off"] + s["name_off"])
    return data, secs, cstr


def _scope(section):
    """A symbol's scope: its output section (the ROM file) without the `..` and `.bss`."""
    s = section
    if s.startswith(".."):
        s = s[2:]
    for suffix in (".bss", ".data", ".rodata", ".text"):
        if s.endswith(suffix):
            s = s[: -len(suffix)]
    return s


# Linker-made names (segment bounds and sizes), never cited.
_LINKER = re.compile(r"^_.*Segment.*(Start|End|Size)$|^_.*(VRAM|VRAM_END|RomStart|RomEnd)$|^\.|^\$|^_gp$|^__")


def elf_symbols(path):
    """[(name, addr, size, kind, scope, file)] with kind 'func' or 'data'."""
    data, secs, cstr = _read_elf(path)
    by_name = {s["name"]: s for s in secs}
    st = by_name[".symtab"]
    strs = secs[st["link"]]
    out = []
    for i in range(st["size"] // 16):
        name, value, size, info, other, shndx = struct.unpack_from(">IIIBBH", data, st["off"] + i * 16)
        typ = info & 0xF
        if typ in (3, 4):  # STT_SECTION, STT_FILE
            continue
        n = cstr(strs["off"] + name) if name else ""
        if not n or _LINKER.match(n):
            continue
        if not (0 < shndx < len(secs)):
            continue  # absolute symbols: enum-like values, sizes
        out.append((n, value, size, "func" if typ == 2 else "data", _scope(secs[shndx]["name"]), ""))
    # Statics from .mdebug, scoped by the section their address falls in.
    vram = sorted((s["addr"], s["addr"] + s["size"], _scope(s["name"])) for s in secs if s["addr"] >= 0x80000000 and s["size"] and s["flags"] & 2)
    import bisect
    starts = [v[0] for v in vram]

    def scope_of(addr):
        i = bisect.bisect_right(starts, addr) - 1
        return vram[i][2] if i >= 0 and vram[i][0] <= addr < vram[i][1] else None

    md = by_name.get(".mdebug")
    if md:
        h = struct.unpack_from(">hh" + "i" * 23, data, md["off"])
        if h[0] != 0x7009:
            sys.exit(f"{path}: .mdebug has magic {h[0]:#x}")
        cbSymOffset, cbSsOffset, cbSsExtOffset, ifdMax, cbFdOffset = h[2 + 8], h[2 + 14], h[2 + 16], h[2 + 17], h[2 + 18]
        iextMax, cbExtOffset = h[2 + 21], h[2 + 22]

        def ss(off):
            return data[off:data.index(b"\0", off)].decode("latin-1")

        fnames = []
        for fi in range(ifdMax):
            fd = struct.unpack_from(">IiiiiiiiiiHhiiiiIii", data, cbFdOffset + fi * 72)
            fnames.append(ss(cbSsOffset + fd[2] + fd[1]) if fd[1] >= 0 else "")
        # The externals name the file each global comes from.
        ext_file = {}
        for ei in range(iextMax):
            flags, ifd, iss, value, bits = struct.unpack_from(">HhiiI", data, cbExtOffset + ei * 16)
            if 0 <= ifd < len(fnames) and iss >= 0:
                ext_file.setdefault((ss(cbSsExtOffset + iss), value & 0xFFFFFFFF), fnames[ifd])
        out = [(n, v, sz, k, sc, f or ext_file.get((n, v), "")) for (n, v, sz, k, sc, f) in out]
        for fi in range(ifdMax):
            fd = struct.unpack_from(">IiiiiiiiiiHhiiiiIii", data, cbFdOffset + fi * 72)
            adr, rss, issBase, cbSs, isymBase, csym = fd[:6]
            fname = fnames[fi]
            for si in range(isymBase, isymBase + csym):
                iss, value, bits = struct.unpack_from(">iiI", data, cbSymOffset + si * 12)
                stype, sclass = bits >> 26, (bits >> 21) & 0x1F
                # stStatic (2) with a data class, stStaticProc (14), stProc (6) with scText
                if stype == 2 and sclass in (2, 3, 13, 14, 15):
                    kind = "data"
                elif stype in (6, 14) and sclass == 1:
                    kind = "func"
                else:
                    continue
                name = ss(cbSsOffset + issBase + iss) if iss >= 0 else ""
                value &= 0xFFFFFFFF
                sc = scope_of(value)
                if not name or sc is None or _LINKER.match(name):
                    continue
                out.append((name, value, 0, kind, sc, fname))
    return out


def pair_symbols(old_syms, new_syms):
    """Pairs old names with new ones at the same address (and, below the KSEG0 range, the same
    ROM file). Returns (changed, unpaired, ambiguous)."""

    def key(s):
        # VRAM addresses are unique across code and the overlays; segmented ones (the assets,
        # 0x0X000000) repeat per file.
        return ("vram", s[1]) if s[1] >= 0x80000000 else (s[4], s[1])

    new_at = defaultdict(list)
    new_names = defaultdict(set)
    for s in new_syms:
        if not any(t[0] == s[0] and t[3] == s[3] for t in new_at[key(s)]):
            new_at[key(s)].append(s)
        new_names[s[0]].add(key(s))
    old_seen = set()
    changed, unpaired, ambiguous = [], [], []
    kept = []
    for s in old_syms:
        k = key(s)
        if (s[0], k) in old_seen:
            continue
        old_seen.add((s[0], k))
        cands = new_at.get(k, [])
        if any(c[0] == s[0] for c in cands):
            kept.append(s)  # unchanged
            continue
        same_kind = [c for c in cands if c[3] == s[3]] or cands
        # A sized symbol (OBJECT/FUNC) wins over a bare label at the same address.
        sized = [c for c in same_kind if c[2]] or same_kind
        names = sorted({c[0] for c in sized})
        if not names:
            unpaired.append(s)
        elif len(names) == 1:
            changed.append((s, names[0]))
        else:
            # Several names at one address (aliases): prefer one that isn't address-named.
            named = [n for n in names if not re.match(r"^(func|D|B|jtbl)_[0-9A-F]{8}", n)]
            if len(named) == 1:
                changed.append((s, named[0]))
            else:
                ambiguous.append((s, names))
                changed.append((s, names[0]))
    # A name renamed in one file but kept in another (sTunicColors: En_Sth's became
    # sShirtColors, z_player_lib.c's didn't): the kept ones are listed too, so the name isn't
    # renamed where its file isn't known.
    renamed_names = {s[0] for s, _ in changed}
    keep = [(s, s[0]) for s in kept if s[0] in renamed_names]
    return changed, unpaired, ambiguous, keep


# --------------------------------------------------------------------------------------------
# C structs and enums
# --------------------------------------------------------------------------------------------


def c_files(root):
    for top in ("include", "src"):
        for d, _, fs in os.walk(os.path.join(root, top)):
            for f in fs:
                if f.endswith((".h", ".c")):
                    yield os.path.join(d, f)


_AGG_OPEN = re.compile(r"^\s*(typedef\s+)?(struct|union)\s+(\w+)?\s*\{\s*(//.*)?$")
# `/* 0x18 */`, or `/* 0x18  0x0034 */` (the offset in this struct, then in an outer one).
_FIELD = re.compile(r"^\s*/\*\s*0x([0-9A-Fa-f]+)(?:\s+0x[0-9A-Fa-f]+)*\s*\*/\s*(.*)$")
_INLINE_AGG = re.compile(r"^(struct|union)\b[^;{]*\{\s*(//.*)?$")


def _decl(decl):
    """(name, type, is_array) of a one-line member declaration (`s16 unk_850;`, `Vec3f pos[2];`,
    `void (*func)(...);`, `u8 flag : 1;`), or None."""
    d = decl.split("//")[0].strip()
    if not d.endswith(";"):
        return None
    d = d[:-1].strip()
    m = re.search(r"\(\s*\*\s*(\w+)\s*\)", d)  # function pointer
    if m:
        return m.group(1), "", False
    arr = "[" in d
    d = re.sub(r"\[[^\]]*\]", "", d).split(":")[0].strip()
    m = re.match(r"^(?:(?:const|volatile|struct|union|unsigned|signed)\s+)*(\w+)[\s*]+(\w+)$", d)
    if not m:
        return None
    ptr = "*" in d
    return m.group(2), ("" if ptr else m.group(1)), arr


def _body(lines, i):
    """The lines of the `{ ... }` opened on line i, the closing line last; and the next index."""
    depth, j, body = lines[i].count("{") - lines[i].count("}"), i + 1, []
    while j < len(lines) and depth > 0:
        depth += lines[j].count("{") - lines[j].count("}")
        body.append(lines[j])
        j += 1
    return body, j


def _members(body):
    """[(offset or None, name, type, is_array)] of an aggregate's body; inline aggregates become
    their members as `outer.inner` (at the outer's offset when they have none of their own)."""
    out, k = [], 0
    while k < len(body) - 1:
        line = body[k]
        fm = _FIELD.match(line)
        off, rest = (int(fm.group(1), 16), fm.group(2)) if fm else (None, line.strip())
        if _INLINE_AGG.match(rest):
            inner, k2 = _body(body, k)
            cm = re.match(r"^\s*\}\s*(\w+)", inner[-1]) if inner else None
            outer = cm.group(1) if cm else None
            for (o2, n2, t2, a2) in _members(inner):
                path = f"{outer}.{n2}" if outer else n2
                # Inner offsets are the outer struct's when they're past the aggregate's own,
                # else relative to it; members with none share the aggregate's (a union's).
                if off is None:
                    o = o2
                elif o2 is None:
                    o = off
                else:
                    o = o2 if o2 >= off else off + o2
                out.append((o, path, t2, a2))
            k = k2
            continue
        d = _decl(rest)
        if d:
            out.append((off, d[0], d[1], d[2]))
        k += 1
    return out


def parse_structs(root):
    """{struct name: [(offset, path, type, is_array)]} and {struct name: file}, for the structs
    whose members carry `/* 0xNN */` offsets."""
    structs, where = {}, {}
    for path in c_files(root):
        lines = open(path, encoding="utf-8", errors="replace").read().split("\n")
        i = 0
        while i < len(lines):
            m = _AGG_OPEN.match(lines[i])
            if not m or lines[i].startswith((" ", "\t")):
                i += 1
                continue
            body, j = _body(lines, i)
            mm = re.match(r"^\s*\}\s*(\w+)\s*;", body[-1]) if body else None
            name = mm.group(1) if mm else m.group(3)
            mem = _members(body)
            # Union members all sit at offset 0 of the union.
            if m.group(2) == "union":
                mem = [(0 if o is None else o, n, t, a) for (o, n, t, a) in mem]
            mem = [x for x in mem if x[0] is not None]
            if name and mem and name not in structs:
                structs[name] = mem
                where[name] = os.path.relpath(path, root).replace("\\", "/")
            i = j
    return structs, where


# The vector and colour types' members (declared without offsets).
_SMALL = {
    "Vec3s": [(0, "x"), (2, "y"), (4, "z")], "Vec3us": [(0, "x"), (2, "y"), (4, "z")], "Vec3f": [(0, "x"), (4, "y"), (8, "z")],
    "Vec3i": [(0, "x"), (4, "y"), (8, "z")], "Vec2s": [(0, "x"), (2, "z")], "Vec2f": [(0, "x"), (4, "z")],
    "Color_RGB8": [(0, "r"), (1, "g"), (2, "b")], "Color_RGBA8": [(0, "r"), (1, "g"), (2, "b"), (3, "a")],
}


def flatten(structs, name, depth=0):
    """{offset: [(path, is_leaf)]} of a struct, through its non-array struct members."""
    out = defaultdict(list)
    if name in _SMALL and name not in structs:
        for o, m in _SMALL[name]:
            out[o].append((m, True))
        return out
    for off, path, ty, arr in structs.get(name, []):
        nested = (ty in structs or ty in _SMALL) and not arr and depth < 6 and ty != name
        out[off].append((path, not nested))
        if nested:
            for o2, items in flatten(structs, ty, depth + 1).items():
                for p2, leaf in items:
                    out[off + o2].append((f"{path}.{p2}", leaf))
    return out


_ENUM_RE = re.compile(r"(typedef\s+)?enum\s+(\w+)?\s*\{(.*?)\}\s*(\w+)?\s*;", re.S)


def _strip_c_comments(s):
    return re.sub(r"//[^\n]*|/\*.*?\*/", "", s, flags=re.S)


def _eval(expr, known):
    e = expr.strip()
    for n in sorted(known, key=len, reverse=True):
        if n in e:
            e = re.sub(rf"\b{re.escape(n)}\b", str(known[n]), e)
    if not re.fullmatch(r"[0-9a-fA-FxX\s+\-*/()<>|&~uUlL]+", e):
        return None
    e = re.sub(r"(?<=[0-9a-fA-F])[uUlL]+\b", "", e)
    try:
        return int(eval(e, {"__builtins__": {}}, {}))
    except Exception:
        return None


def parse_enums(root):
    """{enum name: [(value, member)]} for every named enum (typedef or tag)."""
    enums, where = {}, {}
    for path in c_files(root):
        text = preprocess(open(path, encoding="utf-8", errors="replace").read())
        for m in _ENUM_RE.finditer(text):
            name = m.group(4) or m.group(2)
            if not name:
                continue
            body = "\n".join(l for l in _strip_c_comments(m.group(3)).split("\n") if not l.strip().startswith("#"))
            vals, known, nxt = [], {}, 0
            for item in body.split(","):
                item = item.strip()
                if not re.match(r"^\w+", item):
                    continue
                if "=" in item:
                    n, e = item.split("=", 1)
                    v = _eval(e, known)
                    if v is None:
                        break
                else:
                    n, v = item, nxt
                n = n.strip()
                known[n] = v
                vals.append((v, n))
                nxt = v + 1
            if vals and name not in enums:
                enums[name] = vals
                where[name] = os.path.relpath(path, root).replace("\\", "/")
    return enums, where


def _stem(path):
    s = os.path.splitext(os.path.basename(path))[0]
    return s[3:] if s.startswith("z64") else s


def pair_types(old_slots, new_slots, old_where, new_where):
    """Old type names missing from the new commit, paired with the new type (not in the old
    commit) whose slots (offsets or values) and member names match best. {old: (new, score)}."""
    fresh = [n for n in new_slots if n not in old_slots]
    out = {}
    for name, (oslots, onames) in old_slots.items():
        if name in new_slots:
            continue
        best, score, second = None, 0.0, 0.0
        for c in fresh:
            nslots, nnames = new_slots[c]
            s_slot = len(oslots & nslots) / max(len(oslots | nslots), 1)
            s_name = len(onames & nnames) / max(len(onames | nnames), 1)
            s = 0.5 * s_slot + 0.5 * s_name + (0.1 if _stem(old_where[name]) == _stem(new_where[c]) else 0)
            if s > score:
                best, score, second = c, s, score
            elif s > second:
                second = s
        if best and score >= 0.55 and score - second >= 0.05:
            out[name] = (best, round(score, 2))
    return out


def pair_fields(os_, ns, ow, nw, manual_types=None):
    """Fields by offset. Returns (changed rows, unpaired rows, type renames). `manual_types`:
    hand-checked renames (manual.tsv) for the types the slots can't pair."""
    sig = lambda st: (set(o for o, *_ in st), set(p.split(".")[-1] for _, p, *_ in st))
    renamed = pair_types({n: sig(s) for n, s in os_.items()}, {n: sig(s) for n, s in ns.items()}, ow, nw)
    changed, unpaired = [], []
    for old, (new, score) in sorted(renamed.items()):
        changed.append(("struct", "", old, new, f"{score}"))
    for old, new in sorted((manual_types or {}).items()):
        if old in os_ and new in ns and old not in renamed and old not in ns:
            renamed[old] = (new, "manual")
    for name, mem in sorted(os_.items()):
        tname = renamed.get(name, (name,))[0]
        if tname not in ns:
            unpaired.append(("struct", "", name, "", ow[name]))
            continue
        flat = flatten(ns, tname)
        scope = name if tname == name else f"{name}>{tname}"
        for off, path, ty, arr in mem:
            leaf_old = not ((ty in os_ or ty in _SMALL) and not arr)
            last = path.split(".")[-1]
            cands = flat.get(off, [])
            if any(p == path for p, _ in cands):
                continue
            same = [p for p, leaf in cands if p.split(".")[-1] == last]
            kind_ok = [p for p, leaf in cands if leaf == leaf_old] or [p for p, _ in cands]
            pick = sorted(same, key=len)[0] if same else (kind_ok[0] if kind_ok else None)
            if pick is None:
                unpaired.append(("field", name, path, "", f"0x{off:X}"))
                continue
            others = [p for p in kind_ok if p != pick and not same]
            note = ("|".join([pick] + others[:5])) if len(others) else ""
            changed.append(("field", scope, path, pick, f"0x{off:X}" + (" " + note if note else "")))
    return changed, unpaired, renamed


def pair_enums(oe, ne, ow, nw):
    sig = lambda vals: (set(v for v, _ in vals), set(n for _, n in vals))
    renamed = pair_types({n: sig(s) for n, s in oe.items()}, {n: sig(s) for n, s in ne.items()}, ow, nw)
    changed, unpaired = [], []
    for old, (new, score) in sorted(renamed.items()):
        changed.append(("enum-type", "", old, new, f"{score}"))
    for name, vals in sorted(oe.items()):
        tname = renamed.get(name, (name,))[0]
        if tname not in ne:
            unpaired.append(("enum-type", "", name, "", ow[name]))
            continue
        by_val = defaultdict(list)
        for v, n in ne[tname]:
            by_val[v].append(n)
        new_value = {n: v for v, n in ne[tname]}
        scope = name if tname == name else f"{name}>{tname}"
        seen = defaultdict(int)
        for v, n in vals:
            k = seen[v]
            seen[v] += 1
            # Unchanged only with its value: CS_CMD_CAM_EYE was 1 and is 7 (1 is CS_CMD_CAM_EYE_SPLINE).
            if new_value.get(n) == v:
                continue
            c = by_val.get(v)
            if not c:
                unpaired.append(("enum", name, n, "", str(v)))
                continue
            changed.append(("enum", scope, n, c[min(k, len(c) - 1)], str(v) + (" " + "|".join(c) if len(c) > 1 else "")))
    return changed, unpaired, renamed


# --------------------------------------------------------------------------------------------

# --------------------------------------------------------------------------------------------
# The preprocessor's conditionals, for gc-eu-mq-dbg
# --------------------------------------------------------------------------------------------

# The build's command-line defines for gc-eu-mq-dbg (the new decomp's Makefile), and what
# include/versions.h and include/region.h derive from them. The old commit has no conditionals
# on these, so the same set reads both.
VERSION_DEFINES = {
    "PLATFORM_N64": 0, "PLATFORM_GC": 1, "PLATFORM_IQUE": 0,
    "NTSC_1_0": 1, "NTSC_1_1": 2, "PAL_1_0": 3, "NTSC_1_2": 4, "PAL_1_1": 5, "GC_JP": 6, "GC_JP_MQ": 7, "GC_US": 8,
    "GC_US_MQ": 9, "GC_EU_DBG_2": 10, "GC_EU_MQ_DBG": 11, "GC_EU_DBG": 12, "GC_EU": 13, "GC_EU_MQ": 14, "GC_JP_CE": 15, "IQUE_CN": 16,
    "OOT_VERSION": 11, "OOT_REVISION": 15, "REGION_NULL": 0, "REGION_JP": 1, "REGION_US": 2, "REGION_EU": 3, "REGION_CN": 4, "OOT_REGION": 3,
    "LIBULTRA_VERSION_D": 4, "LIBULTRA_VERSION_E": 5, "LIBULTRA_VERSION_F": 6, "LIBULTRA_VERSION_G": 7, "LIBULTRA_VERSION_H": 8,
    "LIBULTRA_VERSION_I": 9, "LIBULTRA_VERSION_J": 10, "LIBULTRA_VERSION_K": 11, "LIBULTRA_VERSION_L": 12, "LIBULTRA_VERSION": 12, "LIBULTRA_PATCH": 0,
    "DEBUG_FEATURES": 1, "F3DEX_GBI_2": 1, "F3DEX_GBI_PL": 1, "GBI_DOWHILE": 1, "GBI_DEBUG": 1, "_LANGUAGE_C": 1, "__sgi": 1, "_MIPS_SZLONG": 32,
    "OOT_NTSC": 0, "OOT_PAL": 1, "OOT_PAL_N64": 0, "OOT_MQ": 1, "DEBUG_ASSETS": 1,
}


def _cond(expr, defines):
    e = expr.split("//")[0]
    e = re.sub(r"/\*.*?\*/", "", e)
    e = re.sub(r"defined\s*\(\s*(\w+)\s*\)", lambda m: "1" if m.group(1) in defines else "0", e)
    e = re.sub(r"defined\s+(\w+)", lambda m: "1" if m.group(1) in defines else "0", e)
    e = e.replace("&&", " and ").replace("||", " or ")
    e = re.sub(r"!(?!=)", " not ", e)
    e = re.sub(r"\b[A-Za-z_]\w*\b", lambda m: m.group(0) if m.group(0) in ("and", "or", "not") else str(defines.get(m.group(0), 0)), e)
    e = re.sub(r"(?<=\d)[uUlL]+\b", "", e)
    try:
        return bool(eval(e, {"__builtins__": {}}, {}))
    except Exception:
        return False


def preprocess(text, defines=VERSION_DEFINES):
    """Drops the lines of `#if` branches not taken for gc-eu-mq-dbg (the directives too)."""
    out, stack = [], []  # (taking now, any branch taken, parent taking)
    for line in text.split("\n"):
        s = line.strip()
        m = re.match(r"#\s*(if|ifdef|ifndef|elif|else|endif)\b(.*)", s)
        if m:
            d, rest = m.group(1), m.group(2)
            parent = all(t for t, _, _ in stack) if stack else True
            if d in ("if", "ifdef", "ifndef"):
                c = _cond(rest, defines) if d == "if" else ((rest.strip().split() or [""])[0] in defines) == (d == "ifdef")
                stack.append((c, c, parent))
            elif d == "elif" and stack:
                t, taken, p = stack.pop()
                c = not taken and _cond(rest, defines)
                stack.append((c, taken or c, p))
            elif d == "else" and stack:
                t, taken, p = stack.pop()
                stack.append((not taken, True, p))
            elif d == "endif" and stack:
                stack.pop()
            out.append("")
            continue
        out.append(line if all(t for t, _, _ in stack) else "")
    return "\n".join(out)


# --------------------------------------------------------------------------------------------
# Table enums, #defines, files
# --------------------------------------------------------------------------------------------


def _define_rows(text):
    rows = []
    for line in _strip_c_comments(preprocess(text)).split("\n"):
        l = line.strip()
        m = re.match(r"^(DEFINE_\w+)\s*\((.*)\)\s*$", l)
        if m:
            args, depth, cur = [], 0, ""
            for ch in m.group(2):
                if ch == "(":
                    depth += 1
                elif ch == ")":
                    depth -= 1
                if ch == "," and depth == 0:
                    args.append(cur.strip())
                    cur = ""
                else:
                    cur += ch
            args.append(cur.strip())
            rows.append((m.group(1), args))
    return rows


def _table_enum(macro, args):
    """The enum member a table row defines."""
    for a in args:
        if re.match(r"^(SCENE|ENTR|ACTOR|OBJECT|NA_SE|NA_BGM|EFFECT_SS|GAMESTATE)_\w+$", a):
            return a
    return None


TABLES = ["include/tables/scene_table.h", "include/tables/entrance_table.h", "include/tables/actor_table.h", "include/tables/object_table.h",
          "include/tables/effect_ss_table.h", "include/tables/sequence_table.h"]


def table_enums(old_root, new_root):
    """Members of the table-generated enums (scenes, entrances, actors, objects, the sound
    effects per bank...), paired by row."""
    pairs, unpaired = [], []
    files = list(TABLES)
    for d in (os.path.join(old_root, "include/tables/sfx"), os.path.join(new_root, "include/tables/sfx")):
        if os.path.isdir(d):
            files += ["include/tables/sfx/" + f for f in os.listdir(d)]
    for rel in sorted(set(files)):
        po, pn = os.path.join(old_root, rel), os.path.join(new_root, rel)
        if not (os.path.exists(po) and os.path.exists(pn)):
            continue
        ro = [_table_enum(m, a) for m, a in _define_rows(open(po, encoding="utf-8").read())]
        rn = [_table_enum(m, a) for m, a in _define_rows(open(pn, encoding="utf-8").read())]
        if len(ro) != len(rn):
            unpaired.append(("enum", os.path.basename(rel), f"({len(ro)} rows)", f"({len(rn)} rows)", "row counts differ"))
            continue
        for i, (o, n) in enumerate(zip(ro, rn)):
            if o and n and o != n:
                pairs.append(("enum", os.path.splitext(os.path.basename(rel))[0], o, n, f"row 0x{i:X}"))
    return pairs, unpaired


def parse_defines(root):
    """{name: (value, file)} of the object-like integer #defines in include/ and src/."""
    raw, where = {}, {}
    for path in c_files(root):
        text = preprocess(open(path, encoding="utf-8", errors="replace").read())
        rel = os.path.relpath(path, root).replace("\\", "/")
        for line in text.split("\n"):
            m = re.match(r"^\s*#\s*define\s+([A-Za-z_]\w*)\s+(.+?)\s*$", line)
            if m and m.group(1) not in raw:
                raw[m.group(1)] = _strip_c_comments(m.group(2)).strip()
                where[m.group(1)] = rel
    known, out = {}, {}
    for _ in range(4):
        for n, e in raw.items():
            if n in known:
                continue
            v = _eval(e, known)
            if v is not None:
                known[n] = v
    for n, v in known.items():
        out[n] = (v, where[n])
    return out


def _family(name):
    return name.rsplit("_", 1)[0] + "_" if "_" in name else ""


def pair_files(old_syms, new_syms, old_where_types, new_where_types, old_defs, new_defs):
    """C files by the functions and statics they define (.mdebug names each one's file);
    headers by the types and #defines they define."""
    votes = defaultdict(lambda: defaultdict(int))
    new_file_at = {s[1]: s[5] for s in new_syms if s[5]}
    for s in old_syms:
        if s[5] and s[1] in new_file_at:
            votes[s[5]][new_file_at[s[1]]] += 1
    for ow, nw in old_where_types:
        for name, f in ow.items():
            g = nw.get(name)
            if g:
                votes[f][g] += 1
    # #defines only say where things spread to (a header of macros goes many ways), not renames.
    define_votes = defaultdict(lambda: defaultdict(int))
    for name, vf in old_defs.items():
        if name in new_defs and name != "__root__":
            define_votes[vf[1]][new_defs[name][1]] += 1
    out, split = {}, []
    # Files with nothing to vote with (data only, macros only): the same name elsewhere, the
    # name without `z64`, or the `.inc.c` it became.
    for root_old, root_new in ((old_defs.get("__root__"), new_defs.get("__root__")),):
        if not root_old:
            break
        new_files = {os.path.relpath(p, root_new).replace("\\", "/") for p in c_files(root_new)}
        by_base = defaultdict(list)
        for nf in new_files:
            by_base[os.path.basename(nf)].append(nf)
        for of in sorted(os.path.relpath(p, root_old).replace("\\", "/") for p in c_files(root_old)):
            if of in new_files or of in votes:
                continue
            base = os.path.basename(of)
            tries = [base, base[3:] if base.startswith("z64") else None, base[:-2] + ".inc.c" if base.endswith(".c") else None]
            for t in tries:
                if t and len(by_base.get(t, [])) == 1:
                    out[of] = by_base[t][0]
                    break
            else:
                split.append(("file", "", of, "", "gone"))
    root_new = new_defs.get("__root__")
    for f, vs in list(votes.items()):
        # Only C sources and headers that are gone, to the same kind of file.
        keep = {g: c for g, c in vs.items() if g.startswith(("include/", "src/")) and os.path.splitext(g)[1] == os.path.splitext(f)[1] or g.endswith(".inc.c")}
        if not f.startswith(("include/", "src/")) or (root_new and os.path.exists(os.path.join(root_new, f))) or not keep:
            del votes[f]
        else:
            votes[f] = keep
    for f, vs in votes.items():
        total = sum(vs.values())
        ranked = sorted(vs.items(), key=lambda kv: -kv[1])
        best = ranked[0]
        if best[0] == f:
            continue
        # A file whose contents went several ways (z64.h) isn't a rename.
        if best[1] >= (0.75 if f.endswith(".h") else 0.6) * total:
            out[f] = best[0]
        else:
            split.append(("file", "", f, ",".join(f"{n} ({100 * c // total}%)" for n, c in ranked[:5]), "split"))
    # Where each old file's contents went (any file with a tenth of its votes), for #defines.
    spread = defaultdict(set)
    for vs_by in (votes, define_votes):
        for f, vs in vs_by.items():
            spread[f] |= {g for g, c in vs.items() if c >= 0.1 * sum(vs.values())}
    return out, split, spread


def pair_defines(old_defs, new_defs, file_map, spread, new_tokens):
    """Renamed #defines: in the same family (prefix) with the same value, or, for a renamed
    family, a new name with the same value in the paired header and the same last part."""
    pairs, unpaired = [], []
    new_by_value = defaultdict(list)
    new_by_file = defaultdict(lambda: defaultdict(list))
    for n, (v, f) in new_defs.items():
        new_by_value[v].append(n)
        new_by_file[f][v].append(n)
    helper = re.compile(r"_(INDEX|MASK|SHIFT|MAX)(_|$)")
    for n, (v, f) in sorted(old_defs.items()):
        if n.startswith("_") or n.endswith("_H"):
            continue
        # Unchanged: still there with its value (or still used, if its value isn't a number now).
        if n in new_defs and new_defs[n][0] == v or n in new_tokens and n not in new_defs:
            continue
        fam = _family(n)
        nfs = {file_map.get(f, f)} | spread.get(f, set())
        # The same family prefix and value (ACTOR_FLAG_0 -> ACTOR_FLAG_ATTENTION_ENABLED), in a header
        # the old one's contents went to.
        cands = [c for c in new_by_value.get(v, []) if c.startswith(fam) and c not in old_defs and new_defs[c][1] in nfs]
        if len(cands) > 1:
            cands = [c for c in cands if not helper.search(c)] or cands
        if not cands:
            # A renamed family (TOUCH_ON -> ATELEM_ON): the same value and last part in the paired header.
            last = n.split("_")[-1]
            cands = [c for nf in sorted(nfs) for c in new_by_file.get(nf, {}).get(v, []) if c not in old_defs and c.split("_")[-1] == last]
        if len(cands) == 1:
            pairs.append(("define", fam, n, cands[0], f"{v:#x}"))
        else:
            unpaired.append(("define", fam, n, ",".join(cands[:6]), f"{v:#x}"))
    # A renamed family's ties (TOUCH_ON: ATELEM_ON or ACELEM_ON?) go the way its other members went.
    fam_votes = defaultdict(lambda: defaultdict(int))
    for _, fam, o, n, _ in pairs:
        if not n.startswith(fam):
            fam_votes[fam][n.rsplit("_", 1)[0] + "_"] += 1
    still = []
    for row in unpaired:
        _, fam, o, cands, v = row
        to = max(fam_votes[fam].items(), key=lambda kv: kv[1])[0] if fam_votes.get(fam) else None
        pick = [c for c in cands.split(",") if c and to and c.startswith(to)]
        if len(pick) == 1:
            pairs.append(("define", fam, o, pick[0], v))
        else:
            still.append(row)
    return pairs, still


def build(a):
    os.makedirs(a.out, exist_ok=True)
    print("reading symbols...", file=sys.stderr)
    old_syms, new_syms = elf_symbols(a.old_elf), elf_symbols(a.new_elf)
    changed, unpaired, ambiguous, keep = pair_symbols(old_syms, new_syms)
    print(f"symbols: {len(old_syms)} old, {len(new_syms)} new; {len(changed)} renamed, {len(unpaired)} unpaired, {len(ambiguous)} ambiguous", file=sys.stderr)
    print("reading structs and enums...", file=sys.stderr)
    os_, ow = parse_structs(a.old_decomp)
    ns, nw = parse_structs(a.new_decomp)
    # The hand-checked type renames (manual.tsv), for their fields.
    manual_types = {}
    if a.manual and os.path.exists(a.manual):
        for line in open(a.manual, encoding="utf-8"):
            r = line.rstrip("\n").split("\t")
            if not line.startswith("#") and len(r) >= 2 and re.fullmatch(r"[A-Z][A-Za-z0-9]+", r[0]) and re.fullmatch(r"[A-Za-z_]\w*", r[1]):
                manual_types[r[0]] = r[1]
    f_changed, f_unpaired, s_renamed = pair_fields(os_, ns, ow, nw, manual_types)
    oe, oew = parse_enums(a.old_decomp)
    ne, new_ = parse_enums(a.new_decomp)
    e_changed, e_unpaired, e_renamed = pair_enums(oe, ne, oew, new_)
    print(f"structs: {len(os_)} old, {len(ns)} new; {len(s_renamed)} renamed; {len(f_changed) - len(s_renamed)} field renames, {len(f_unpaired)} unpaired", file=sys.stderr)
    print(f"enums: {len(oe)} old, {len(ne)} new; {len(e_renamed)} renamed; {len(e_changed) - len(e_renamed)} member renames, {len(e_unpaired)} unpaired", file=sys.stderr)

    t_changed, t_unpaired = table_enums(a.old_decomp, a.new_decomp)
    od, nd = parse_defines(a.old_decomp), parse_defines(a.new_decomp)
    od["__root__"], nd["__root__"] = a.old_decomp, a.new_decomp
    files, f_split, spread = pair_files(old_syms, new_syms, [(ow, nw), (oew, new_)], [], od, nd)
    new_tokens = set()
    for path in c_files(a.new_decomp):
        new_tokens.update(re.findall(r"[A-Za-z_]\w*", open(path, encoding="utf-8", errors="replace").read()))
    del od["__root__"], nd["__root__"]
    d_changed, d_unpaired = pair_defines(od, nd, files, spread, new_tokens)
    print(f"tables: {len(t_changed)} member renames; defines: {len(d_changed)} renames, {len(d_unpaired)} unpaired; files: {len(files)} moved", file=sys.stderr)

    rows = [(s[3], s[4], s[0], n, f"{s[1]:08X}") for s, n in changed + keep]
    rows.extend(f_changed)
    rows.extend(e_changed)
    rows.extend(t_changed)
    rows.extend(d_changed)
    rows.extend(("file", "", o, n, "") for o, n in sorted(files.items()))
    order = {"file": -1, "func": 0, "data": 1, "struct": 2, "field": 3, "enum-type": 4, "enum": 5, "define": 6}
    rows.sort(key=lambda r: (order.get(r[0], 9), r[1], r[2]))
    with open(os.path.join(a.out, "name_map.tsv"), "w", newline="\n", encoding="utf-8") as f:
        f.write(f"# The decomp name map, {a.old_label} -> {a.new_label} (gc-eu-mq-dbg): only the names that changed. Generated by\n")
        f.write("# scripts/name_map.py (docs/adr/0031-decomp-main.md); don't edit. kind: func/data (paired by address; scope =\n")
        f.write("# the ROM file), struct/enum-type (renamed types), field (by offset; scope = struct, old>new if renamed; where =\n")
        f.write("# offset, then every candidate when several share it), enum (by value).\n")
        f.write("# kind\tscope\told\tnew\twhere\n")
        for r in rows:
            f.write("\t".join(r) + "\n")
    with open(os.path.join(a.out, "unpaired.tsv"), "w", newline="\n", encoding="utf-8") as f:
        f.write("# Old names the map couldn't pair (no symbol, field or member at the same address, offset or value), and\n")
        f.write("# symbol pairings with more than one candidate (the first is in name_map.tsv). Generated by scripts/name_map.py.\n")
        f.write("# status\tkind\tscope\told\tcandidates\twhere\n")
        for s in unpaired:
            f.write(f"unpaired\t{s[3]}\t{s[4]}\t{s[0]}\t\t{s[1]:08X}\n")
        for s, names in ambiguous:
            f.write(f"ambiguous\t{s[3]}\t{s[4]}\t{s[0]}\t{','.join(names)}\t{s[1]:08X}\n")
        for r in f_split + f_unpaired + e_unpaired + t_unpaired + d_unpaired:
            f.write("unpaired\t" + "\t".join(r) + "\n")
    print(f"wrote {a.out}/name_map.tsv ({len(rows)} rows) and unpaired.tsv", file=sys.stderr)


# --------------------------------------------------------------------------------------------
# Renaming with the map, and comparing what the two commits' names produced
# --------------------------------------------------------------------------------------------


class Renamer:
    """Old C names to new ones in text: symbols (by their ROM file when given), types, enum
    members, #defines and the hand-checked names. Plain lowercase words aren't touched."""

    def __init__(self, map_path, manual_path):
        self.sym = defaultdict(set)
        self.scoped = {}
        self.types = {}
        self.consts = defaultdict(set)
        for line in open(map_path, encoding="utf-8"):
            if line.startswith("#"):
                continue
            k, scope, o, n, w = line.rstrip("\n").split("\t")[:5]
            if k in ("func", "data"):
                self.sym[o].add(n)
                self.scoped[(scope, o)] = n
            elif k in ("struct", "enum-type"):
                self.types[o] = n
            elif k in ("enum", "define"):
                self.consts[o].add(n)
        if manual_path and os.path.exists(manual_path):
            for line in open(manual_path, encoding="utf-8"):
                if line.startswith("#"):
                    continue
                r = line.rstrip("\n").split("\t")
                if len(r) >= 2 and re.fullmatch(r"[A-Za-z_]\w*", r[1] or "-"):
                    (self.types.__setitem__(r[0], r[1]) if re.match(r"^[A-Z][a-z]", r[0]) and "_" not in r[0] else self.consts.__setitem__(r[0], {r[1]}))
        self.unresolved = defaultdict(int)

    def name(self, t, scope=None):
        up = t
        if re.fullmatch(r"func_[0-9a-f]{8}|d_[0-9a-f]{8}", t):
            up = ("func_" if t[0] == "f" else "D_") + t[t.index("_") + 1:].upper()
        if up in self.sym and not re.fullmatch(r"[a-z]+", up):
            if scope and (scope, up) in self.scoped:
                return self.scoped[(scope, up)]
            if len(self.sym[up]) == 1:
                return next(iter(self.sym[up]))
            self.unresolved[up] += 1
            return t
        if t in self.types:
            return self.types[t]
        c = self.consts.get(t)
        if c and len(c) == 1:
            return next(iter(c))
        return t

    def text(self, s, scope=None):
        return re.sub(r"[A-Za-z_][A-Za-z0-9_]*", lambda m: self.name(m.group(0), scope), s)

    def key(self, k):
        """A pack record name `kind/<file>/<symbol>...`: its symbols looked up in <file>."""
        parts = k.split("/")
        scope = parts[1] if len(parts) > 2 else None
        return "/".join([parts[0]] + [self.text(p, scope) for p in parts[1:]])

    def blob(self, b):
        """bincode strings (u64 little-endian length, then the bytes) renamed in a record."""
        out, i, last = [], 0, 0
        for m in re.finditer(rb"(?s)[\x01-\xff]\x00{7}", b):
            i = m.start()
            if i < last:
                continue
            n = b[i]
            s = b[i + 8:i + 8 + n]
            if len(s) != n or not re.fullmatch(rb"[\x20-\x7e]+", s):
                continue
            t = s.decode()
            r = self.text(t)
            if r != t:
                out.append(b[last:i])
                out.append(len(r).to_bytes(8, "little") + r.encode())
                last = i + 8 + n
        out.append(b[last:])
        return b"".join(out)


def _records(d):
    out = {}
    for root, _, fs in os.walk(d):
        for f in fs:
            if f.endswith(".bin"):
                rel = os.path.relpath(os.path.join(root, f), d).replace("\\", "/")[:-4]
                out[rel] = os.path.join(root, f)
    return out


# Enums whose members the pack stores without their prefix (Player's tables: `ap_names`,
# `model_group_names`...): (old prefix, new prefix, the records holding them).
STRIPPED = [("PLAYER_AP_", "PLAYER_IA_"), ("PLAYER_MODELGROUP_", "PLAYER_MODELGROUP_"), ("PLAYER_MODELTYPE_", "PLAYER_MODELTYPE_"),
            ("PLAYER_MWA_", "PLAYER_MWA_"), ("PLAYER_LIMB_", "PLAYER_LIMB_"), ("PLAYER_SHIELD_", "PLAYER_SHIELD_"), ("PLAYER_TUNIC_", "PLAYER_TUNIC_")]
STRIPPED_IN = ("table/player", "table/player_rules")


def _bincode_str(s):
    b = s.encode()
    return len(b).to_bytes(8, "little") + b


def _bincode_strings(b):
    out, last = [], 0
    for m in re.finditer(rb"(?s)[\x01-\xff]\x00{7}", b):
        i = m.start()
        if i < last:
            continue
        n = b[i]
        s = b[i + 8:i + 8 + n]
        if len(s) == n and re.fullmatch(rb"[\x20-\x7e]+", s):
            out.append(s.decode())
            last = i + 8 + n
    return out


def script_renames(old, new, ren):
    """The old pack's offset-keyed scene scripts (`cutscene/link_home_scene/0x15D0`) paired with
    the new pack's names for them: where a scene layer names one, by its place in the scene's
    record (the same layer of the same scene); else by content, its name aside. Returns (old key
    -> new key, every string to rename: the keys, and the offsets alone where one stands for
    one name)."""
    key_renames = {}
    for k in old:
        if not k.startswith("scene/") or k not in new:
            continue
        so = _bincode_strings(open(old[k], "rb").read())
        sn = _bincode_strings(open(new[k], "rb").read())
        if len(so) == len(sn):
            for o, n in zip(so, sn):
                if re.match(r"^cutscene/[^/]+/0x[0-9A-F]+$", o) and o != n:
                    key_renames.setdefault(o, n)
    named_new = defaultdict(list)
    for k in new:
        m = re.match(r"^cutscene/([^/]+)/([A-Za-z_]\w*)$", k)
        if m:
            named_new[m.group(1)].append((k, m.group(2)))
    for k in sorted(old):
        m = re.match(r"^cutscene/([^/]+)/(0x[0-9A-F]+)$", k)
        if not m or ren.key(k) in new or k in key_renames:
            continue
        ob = open(old[k], "rb").read()
        for nk, name in named_new[m.group(1)]:
            if ob.replace(_bincode_str(m.group(2)), _bincode_str(name)) == open(new[nk], "rb").read():
                key_renames[k] = nk
                break
    strings = dict(key_renames)
    bare = defaultdict(set)
    for k, v in key_renames.items():
        bare[k.rsplit("/", 1)[1]].add(v.rsplit("/", 1)[1])
    strings.update({o: next(iter(n)) for o, n in bare.items() if len(n) == 1})
    return key_renames, strings


def compare_pack(a):
    """Two loose packs (`oot import --loose`), the old commit's against the new one's: each old
    record under its renamed name, its bytes with their strings renamed.

    Two kinds of names aren't the decomp's symbols, so they're paired here: the enum members
    Player's tables store without their prefix (`STICK` for `PLAYER_AP_STICK`, now
    `DEKU_STICK`), and the scene layers' scripts the old XMLs didn't name, keyed by their offset
    (`cutscene/link_home_scene/0x15D0`): each is paired with the new named script of the same
    file that has its bytes, its name aside."""
    ren = Renamer(a.map, a.manual)
    old, new = _records(a.old), _records(a.new)
    # Prefix-stripped enum members.
    suffix = {}
    for line in open(a.map, encoding="utf-8"):
        r = line.rstrip("\n").split("\t")
        if line.startswith("#") or r[0] != "enum":
            continue
        for op, np in STRIPPED:
            if r[2].startswith(op) and r[3].startswith(np):
                suffix[r[2][len(op):]] = r[3][len(np):]
    key_renames, strings = script_renames(old, new, ren)

    def renamed_blob(k, b):
        b = ren.blob(b)
        for o, n in strings.items():
            b = b.replace(_bincode_str(o), _bincode_str(n))
        if k in STRIPPED_IN:
            out, last = [], 0
            for m in re.finditer(rb"(?s)[\x01-\x40]\x00{7}", b):
                i = m.start()
                if i < last:
                    continue
                n = b[i]
                s = b[i + 8:i + 8 + n]
                if len(s) == n and re.fullmatch(rb"[A-Z0-9_]+", s) and s.decode() in suffix:
                    out += [b[last:i], _bincode_str(suffix[s.decode()])]
                    last = i + 8 + n
            b = b"".join(out + [b[last:]])
        return b

    if a.dump:
        os.makedirs(a.dump, exist_ok=True)
    seen, same, renamed, differ, missing = set(), 0, 0, [], []
    for k in sorted(old):
        k2 = key_renames.get(k) or ren.key(k)
        if k2 not in new:
            missing.append((k, k2))
            continue
        seen.add(k2)
        ob, nb = open(old[k], "rb").read(), open(new[k2], "rb").read()
        if ob == nb:
            same += 1
        elif renamed_blob(k, ob) == nb:
            renamed += 1
        else:
            differ.append((k, k2, len(ob), len(nb)))
            if a.dump:
                d = os.path.join(a.dump, k2.replace("/", "__"))
                open(d + ".old", "wb").write(renamed_blob(k, ob))
                open(d + ".new", "wb").write(nb)
    extra = sorted(set(new) - seen)
    print(f"{len(old)} old records, {len(new)} new")
    print(f"  {same} the same bytes (", sum(1 for k in old if (key_renames.get(k) or ren.key(k)) != k and (key_renames.get(k) or ren.key(k)) in new), "of them under a new name)")
    print(f"  {renamed} the same once their strings are renamed ({len(key_renames)} offset-keyed scripts named by the new XMLs)")
    print(f"  {len(differ)} differ, {len(missing)} missing, {len(extra)} new")
    for k, k2, n1, n2 in differ[: a.limit]:
        print(f"  differ  {k}" + (f" -> {k2}" if k2 != k else "") + f"  ({n1} -> {n2} bytes)")
    for k, k2 in missing[: a.limit]:
        print(f"  missing {k}" + (f" (looked for {k2})" if k2 != k else ""))
    for k in extra[: a.limit]:
        print(f"  new     {k}")
    if ren.unresolved:
        print("  names the map has several successors for:", dict(list(ren.unresolved.items())[:20]))
    return 0 if not (differ or missing or extra) else 1


def compare_trace(a):
    """A golden trace recorded on the old names against one recorded on the new: the old one's
    C names renamed through the map, then the bytes compared."""
    ren = Renamer(a.map, a.manual)
    ob = open(a.old, "rb").read()
    nb = open(a.new, "rb").read()
    mapped = ren.text(ob.decode("utf-8"))
    # The scene scripts the old XMLs didn't name (`"script": "0x15D0"`), from the two packs.
    if a.packs:
        _, strings = script_renames(_records(a.packs[0]), _records(a.packs[1]), ren)
        mapped = re.sub(r'"(0x[0-9A-F]+)"', lambda m: '"' + strings.get(m.group(1), m.group(1)) + '"', mapped)
    mapped = mapped.encode("utf-8")
    if mapped == nb:
        print(f"{a.new}: the same bytes as {a.old} with its names renamed" + (" (no names changed)" if ob == nb else ""))
        return 0
    i = next((j for j in range(min(len(mapped), len(nb))) if mapped[j] != nb[j]), min(len(mapped), len(nb)))
    print(f"{a.new}: differs from the renamed {a.old} at byte {i}: {mapped[max(0, i - 60):i + 60]!r} vs {nb[max(0, i - 60):i + 60]!r}")
    return 1


def find(a):
    hits = 0
    with open(a.map, encoding="utf-8") as f:
        for line in f:
            if line.startswith("#"):
                continue
            r = line.rstrip("\n").split("\t")
            if a.name in (r[2], r[3]) or a.name in r[3].split(".") or a.name in r[2].split("."):
                print("\t".join(r))
                hits += 1
    if not hits:
        print(f"{a.name}: not in the map (unchanged, or unknown)")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build")
    b.add_argument("--old-elf", required=True)
    b.add_argument("--new-elf", required=True)
    b.add_argument("--old-decomp", required=True)
    b.add_argument("--new-decomp", required=True)
    b.add_argument("--old-label", default="2f4c25d")
    b.add_argument("--new-label", default="new")
    b.add_argument("--out", default=os.path.join(os.path.dirname(__file__), "..", "docs", "name-map"))
    b.add_argument("--manual", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "docs", "name-map", "manual.tsv"))
    here = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "docs", "name-map")
    fd = sub.add_parser("find")
    fd.add_argument("name")
    fd.add_argument("--map", default=os.path.join(here, "name_map.tsv"))
    for name, fn in (("compare-pack", compare_pack), ("compare-trace", compare_trace)):
        c = sub.add_parser(name)
        c.add_argument("old")
        c.add_argument("new")
        c.add_argument("--map", default=os.path.join(here, "name_map.tsv"))
        c.add_argument("--manual", default=os.path.join(here, "manual.tsv"))
        c.add_argument("--limit", type=int, default=40)
        c.add_argument("--dump", help="compare-pack: write each differing record (the old one renamed, and the new) here")
        c.add_argument("--packs", nargs=2, metavar=("OLD", "NEW"), help="compare-trace: the two loose packs, for the scene scripts the old XMLs didn't name")
    a = p.parse_args()
    sys.exit({"build": build, "find": find, "compare-pack": compare_pack, "compare-trace": compare_trace}[a.cmd](a) or 0)


if __name__ == "__main__":
    main()
