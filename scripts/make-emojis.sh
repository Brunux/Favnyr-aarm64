#!/bin/sh
# Regenerate the emoji table the emoji picker reads.
#
# The picker (`~/.config/bin/rofi-emoji`, Alt+Ctrl+E) is a rofi list over one
# data file: a name to type and the glyph to insert.  Deboma gets that table from
# its Quickshell emoji panel; Debian ships no emoji list at all, and the two
# packages that carry one (`fonts-noto-color-emoji` has the glyphs, not the
# names) do not overlap.
#
# So the table is generated here, once, from Python's Unicode database - the same
# source the fonts and the names come from - and committed.  The image needs no
# Python and no package for it: `rootfs-setup.sh` copies it with the rest of the
# home directory, and `rofi-emoji` reads it.
#
# The table is filtered against the font that actually draws the glyphs.  A
# codepoint the Unicode database names but the font does not carry is a row that
# inserts a tofu box, which is what the card reported: the blocks below contain
# text symbols (chess pieces, arrows, the alchemical signs) that are not emoji
# and that `fonts-noto-color-emoji` deliberately does not draw.  The font is the
# one fontconfig resolves for `emoji` (`fc-match`), and its character map
# (`fc-query --format='%{charset}'`) is what a codepoint is checked against.
# `~/.config/emojis.font` next to the table pins the font's SHA256 and family, so
# `check-system.sh` can say when a font upgrade made the table stale; the table
# itself is pure data - one `name<TAB>glyph<TAB>group` row each, no comment line.
#
#   scripts/make-emojis.sh          rewrite ~/.config/emojis.tsv and emojis.font
#
# Regenerating needs fontconfig (`fc-match`, `fc-query`) and the emoji font
# (`fonts-noto-color-emoji`) on the host.  Neither is a build dependency of the
# image: the generated files are committed, and the image carries the font as a
# desktop package.
#
# Only the emoji blocks of the standard are walked, because a table of every
# named symbol would be a Unicode dump, not a picker: Miscellaneous Symbols
# (U+2600), Dingbats (U+2700), Miscellaneous Symbols and Arrows (U+2B00),
# Miscellaneous Symbols and Pictographs (U+1F300), Emoticons (U+1F600), Transport
# and Map (U+1F680), Supplemental Symbols and Pictographs (U+1F900) and Symbols
# and Pictographs Extended-A (U+1FA70).
#
# What is left out on purpose: the skin-tone modifiers (U+1F3FB-U+1F3FF) and the
# zero-width-joiner sequences (family, profession, flag), because one row is one
# codepoint and a ZWJ sequence is several; a name is the search key and the glyph
# is inserted as-is, so a partial sequence would insert something else.

set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
out="$here/uis/sway/files/home/.config/emojis.tsv"
sidecar="$here/uis/sway/files/home/.config/emojis.font"

die() {
	echo "make-emojis.sh: $*" >&2
	exit 1
}

command -v python3 >/dev/null 2>&1 ||
	die "python3 is needed to read the Unicode database"

# fontconfig and the font are the filter.  Without either, the table would be the
# unfiltered one that ships tofu rows, so this is a hard requirement, not a
# fallback to "every named symbol".
command -v fc-match >/dev/null 2>&1 && command -v fc-query >/dev/null 2>&1 ||
	die "fontconfig is needed (fc-match and fc-query): install fontconfig, and fonts-noto-color-emoji for the font they resolve"

font=$(fc-match -f '%{file}' emoji 2>/dev/null) || font=
[ -n "$font" ] && [ -r "$font" ] ||
	die "fontconfig resolves no readable 'emoji' font: install fonts-noto-color-emoji (and fontconfig) on this host"

family=$(fc-query --format='%{family}' "$font" 2>/dev/null) || family=
case $family in
*[Ee]moji* | *[Cc]olor*) : ;;
*) die "'$font' (${family:-no family}) is not a colour emoji font: install fonts-noto-color-emoji so fontconfig's 'emoji' match resolves it" ;;
esac

charset=$(fc-query --format='%{charset}' "$font" 2>/dev/null) || charset=
[ -n "$charset" ] ||
	die "fc-query returned no character map for '$font' (fontconfig cannot read the font)"

command -v sha256sum >/dev/null 2>&1 ||
	die "sha256sum is needed to pin the font (coreutils)"
sum=$(sha256sum "$font")
sum=${sum%% *}

mkdir -p "$(dirname "$out")"

EMOJI_CHARSET=$charset python3 - "$out" <<'PY'
import os
import sys
import unicodedata

out = sys.argv[1]

# The font's character map, as fontconfig prints it: hex codepoints and ranges
# separated by whitespace.  A codepoint outside it is a row that would insert a
# box, so it is not a row.
covered = set()
for token in os.environ["EMOJI_CHARSET"].split():
    if "-" in token:
        first, last = token.split("-", 1)
        covered.update(range(int(first, 16), int(last, 16) + 1))
    else:
        covered.add(int(token, 16))
if not covered:
    sys.exit("make-emojis.sh: the font character map is empty")

# (first, last, group).  The group is the dim column of the row, so typing
# "faces" or "transport" narrows the list the way a category tab would.
blocks = [
    (0x2600, 0x26FF, "symbols"),
    (0x2700, 0x27BF, "dingbats"),
    (0x2B00, 0x2BFF, "symbols"),
    (0x1F300, 0x1F5FF, "objects"),
    (0x1F600, 0x1F64F, "faces"),
    (0x1F680, 0x1F6FF, "transport"),
    (0x1F900, 0x1F9FF, "symbols"),
    (0x1FA70, 0x1FAFF, "symbols"),
]

# Not emoji, although they sit inside those blocks: the five skin-tone
# modifiers, the keycap base and the tag characters.
skip = set(range(0x1F3FB, 0x1F400)) | {0x20E3}

rows = []
seen = set()
uncovered = 0
for first, last, group in blocks:
    for cp in range(first, last + 1):
        if cp in skip:
            continue
        ch = chr(cp)
        try:
            name = unicodedata.name(ch)
        except ValueError:
            continue
        if unicodedata.category(ch) not in ("So", "Sm", "Sk"):
            continue
        if cp not in covered:
            uncovered += 1
            continue
        # One line per name: two codepoints can carry the same name (the
        # presentation variants of the same symbol), and a duplicate would make
        # the row table ambiguous.
        if name in seen:
            continue
        seen.add(name)
        glyph = ch
        rows.append((name.lower(), glyph, group))

rows.sort(key=lambda row: row[0])

with open(out, "w", encoding="utf-8") as fh:
    for name, glyph, group in rows:
        fh.write(f"{name}\t{glyph}\t{group}\n")

print(f"make-emojis.sh: {len(rows)} emoji ({uncovered} named codepoints the font does not draw) -> {out}")
PY

# The sidecar is what check-system.sh compares with the font installed on the
# machine: a font upgrade changes this hash, and the table then has to be
# regenerated.  It is not part of the table because the picker reads that file as
# rows.
cat >"$sidecar" <<EOF
# Font the emoji table was filtered against (generated by scripts/make-emojis.sh).
sha256=$sum
family=$family
EOF

echo "make-emojis.sh: pinned $family ($sum) -> $sidecar"
