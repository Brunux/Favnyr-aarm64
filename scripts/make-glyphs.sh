#!/bin/sh
# Regenerate the TUI alphabet: the codepoint behind every `Glyph` name.
#
# The session's Rust framework (`rust/crates/lazyos-ui`) draws its rows in
# `foot`, and a row's picture is a Nerd Font codepoint - the launcher's Font
# Awesome images belong to rofi, where Pango resolves them and nothing has to
# line up.  `Glyph` is the handle a command names (`Glyph::Bluetooth`, not
# `'\u{f293}'`); this script is where a name becomes a codepoint.
#
# The table is generated, then filtered against the font that actually draws it.
# `fonts-nerd-symbols` is DFSG-stripped: it carries the Octicons, Codicons,
# Devicons, Seti and Powerline sets and **none** of Font Awesome or the MDI
# plane 15, so eight of the session's names (bluetooth, wifi, battery, keyboard,
# mouse, calculator, qr, crop) have no codepoint in it at all.  A row for one of
# those draws its short text token instead - never a tofu box, which is a hole
# in the row that logs nothing.  That is the rule the spec states and this
# script enforces: a codepoint the font's character map does not list becomes
# `draw: None`, and the column falls back to text.
#
#   scripts/make-glyphs.sh                 rewrite the table and the pin
#   scripts/make-glyphs.sh --check         exit 1 when either file is stale
#   scripts/make-glyphs.sh --font F --charset C --out D   (the tests' form)
#
# The candidates below are Nerd Fonts v3.5.1 codepoints (`glyphnames.json` of
# the `ryanoasis/nerd-fonts` repository), restricted to the sets this package
# ships: `cod-*`, `oct-*`, `seti-*` and the one IEC symbol at U+23FB.  Nothing
# here comes from `fa-*` or `md-*`, because those planes are exactly what the
# stripped package leaves out - the fallbacks are the names that would need
# them.
#
# Regenerating needs fontconfig (`fc-match`, `fc-query`), `sha256sum` and the
# font itself (`fonts-nerd-symbols`) on the host.  None of that is a build
# dependency of the image: the generated files are committed, and the image
# carries the font as a desktop package.  `rootfs-setup.sh` ships the files with
# the rest of the source tree, and `check-system.sh` compares the pin with the
# font installed on the machine.

set -eu

# The release the candidate codepoints were read from.  Bumping it means
# re-checking the list above against upstream's `glyphnames.json`, which is the
# point of writing it down.
nerd_version=3.5.1

here=$(cd "$(dirname "$0")/.." && pwd)
out="$here/rust/crates/lazyos-ui/src/glyphs/table.rs"
sidecar="$here/rust/crates/lazyos-ui/src/glyphs/font.pin"
font=
charset_file=
check=no

die() {
	echo "make-glyphs.sh: $*" >&2
	exit 1
}

usage() {
	cat <<'EOF'
usage: scripts/make-glyphs.sh [--font FILE] [--charset FILE] [--out FILE]
                              [--sidecar FILE] [--check]

  --font FILE      the symbol font to filter against (default: fontconfig's
                   match for 'Symbols Nerd Font Mono')
  --charset FILE   read the character map from FILE instead of fc-query
  --out FILE       where to write table.rs (default: the crate's src/glyphs/)
  --sidecar FILE   where to write font.pin (default: the crate's src/glyphs/)
  --check          write nothing; exit 1 when the committed files are stale
EOF
}

while [ $# -gt 0 ]; do
	case $1 in
	--font)
		[ $# -ge 2 ] || die "--font needs a file"
		font=$2
		shift 2
		;;
	--charset)
		[ $# -ge 2 ] || die "--charset needs a file"
		charset_file=$2
		shift 2
		;;
	--out)
		[ $# -ge 2 ] || die "--out needs a file"
		out=$2
		shift 2
		;;
	--sidecar)
		[ $# -ge 2 ] || die "--sidecar needs a file"
		sidecar=$2
		shift 2
		;;
	--check)
		check=yes
		shift
		;;
	-h | --help)
		usage
		exit 0
		;;
	*)
		usage >&2
		die "unknown argument '$1'"
		;;
	esac
done

command -v awk >/dev/null 2>&1 || die "awk is needed to build the table"
command -v sha256sum >/dev/null 2>&1 ||
	die "sha256sum is needed to pin the font (coreutils)"

# Resolve the font and its character map.  `--charset` exists for the crate's
# integration test: it runs this script against a two-line fixture and must not
# depend on which fonts the machine has.
if [ -z "$font" ]; then
	command -v fc-match >/dev/null 2>&1 ||
		die "fontconfig is needed (fc-match): install fontconfig, and fonts-nerd-symbols for the font it resolves"
	command -v fc-query >/dev/null 2>&1 ||
		die "fontconfig is needed (fc-query): install fontconfig"
	font=$(fc-match -f '%{file}' 'Symbols Nerd Font Mono' 2>/dev/null) || font=
	[ -n "$font" ] && [ -r "$font" ] ||
		die "fontconfig resolves no readable 'Symbols Nerd Font Mono': install fonts-nerd-symbols on this host"
fi
[ -r "$font" ] || die "cannot read the font '$font'"

if [ -n "$charset_file" ]; then
	[ -r "$charset_file" ] || die "cannot read the character map '$charset_file'"
	charset=$(cat "$charset_file")
	family=${MAKE_GLYPHS_FAMILY:-fixture}
else
	command -v fc-query >/dev/null 2>&1 ||
		die "fontconfig is needed (fc-query): install fontconfig"
	charset=$(fc-query --format='%{charset}' "$font" 2>/dev/null) || charset=
	[ -n "$charset" ] ||
		die "fc-query returned no character map for '$font' (fontconfig cannot read the font)"
	family=$(fc-query --format='%{family[0]}' "$font" 2>/dev/null) || family=
fi

# A fontconfig match for a family that is not installed answers another family,
# and a table filtered against the wrong font is a table of holes: refuse.
if [ -z "$charset_file" ] && [ "${family#*Symbols Nerd Font}" = "$family" ]; then
	die "'$font' is ${family:-no family}, not a Symbols Nerd Font: install fonts-nerd-symbols"
fi

sum=$(sha256sum "$font")
sum=${sum%% *}
base=$(basename "$font")

tmp=$(mktemp -d "${TMPDIR:-/tmp}/make-glyphs.XXXXXX")
trap 'rm -rf "$tmp"' EXIT INT TERM HUP

# ---------------------------------------------------------------------------
# The candidates: one `Variant codepoint` pair per line, in the order the enum
# lists them.  A `-` is a name the stripped symbol set has no codepoint for at
# all; the row shows its text token (see `Glyph::token`).
#
# Nerd Fonts v3.5.1 names are in the comment, so the next person can re-check a
# choice against upstream's table instead of guessing what a glyph is.
# ---------------------------------------------------------------------------
cat >"$tmp/candidates" <<'EOF'
# files and places
Folder ea83			# cod-folder
File ea7b			# cod-file
Document ebaf			# cod-notebook
Pdf eaeb			# cod-file_pdf
Html eae9			# cod-file_code
Archive eaef			# cod-file_zip
Executable eae8			# cod-file_binary
Hidden eae7			# cod-eye_closed
BrokenLink f529			# oct-unlink
Image f4e5			# oct-image
Video f52c			# oct-video
Music ec1b			# cod-music
Library eb9c			# cod-library
Home eb06			# cod-home
Desktop f4a9			# oct-device_desktop
Terminal ea85			# cod-terminal
# the machine
Window eb7f			# cod-window
Disk f472			# oct-database (the closest thing to a drive this set has)
Gear eaf8			# cod-gear
Book eaa4			# cod-book
People f4fd			# oct-people
Globe eb01			# cod-globe
Link eb15			# cod-link
Volume eb75			# cod-unmute
Bell eaa2			# cod-bell
BellOff ec08			# cod-bell_slash
Clock f43a			# oct-clock
Calendar eab0			# cod-calendar
History ea82			# cod-history
Trash ea81			# cod-trash
Bug eaaf			# cod-bug
Mic ec12			# cod-mic
Sliders f462			# oct-sliders
Lock ea75			# cod-lock
Unlock eb74			# cod-unlock
Eye ea70			# cod-eye
Moon f4ee			# oct-moon
Brightness f522			# oct-sun
Keyboard -			# no codicon; Font Awesome's f11c only
Wifi -				# Font Awesome's f1eb only
Bluetooth -			# Font Awesome brands' f293 only
Battery -			# Font Awesome's f240 only
Mouse -				# Font Awesome's f245 only
Network eb34			# cod-radio_tower
Server eb50			# cod-server
Database eace			# cod-database
Chip ec19			# cod-chip
Display eb4d			# cod-screen_normal
Power 23fb			# iec-power
Logout ea6e			# cod-sign_out
Reboot ead2			# cod-debug_restart
Rotate eb37			# cod-refresh
# actions
Search ea6d			# cod-search
Warning ea6c			# cod-warning
Info ea74			# cod-info
Check eab2			# cod-check
Undo eae2			# cod-discard
Save eb4b			# cod-save
Sync ea77			# cod-sync
Download f409			# oct-download
Capture eb4c			# cod-screen_full
Camera eada			# cod-device_camera
Crop -				# Font Awesome's f125 only
Qr -				# Font Awesome's f029 only
Clipboard ebcc			# cod-copy (the set has no clipboard proper)
Smile eb54			# cod-smiley
Filter eaf1			# cod-filter
Tag ea66			# cod-tag
Key eb11			# cod-key
Map ec05			# cod-map
Theme eb5c			# cod-symbol_color
Brush f48f			# oct-paintbrush
Box eb29			# cod-package
Calculator -			# Font Awesome's f1ec only
Play eb2c			# cod-play
Pause ead1			# cod-debug_pause
Stop ead7			# cod-debug_stop
Next eab6			# cod-chevron_right
Prev eab5			# cod-chevron_left
ArrowUp eaa1			# cod-arrow_up
ArrowDown ea9a			# cod-arrow_down
ArrowLeft ea9b			# cod-arrow_left
ArrowRight ea9c			# cod-arrow_right
Target ebf8			# cod-target
Star eb59			# cod-star_full
Heart eb05			# cod-heart
Pin eb2b			# cod-pin
Lightbulb ea61			# cod-lightbulb
Cloud ebaa			# cod-cloud
# the tools and the agents
Code eac4			# cod-code
Agent eb08			# cod-hubot
Sparkle ec10			# cod-sparkle
Git f418			# oct-git_branch
Github ea84			# cod-github
Docker e7b0			# dev-docker
Rust e68b			# seti-rust
Python e606			# seti-python
Node e719			# dev-nodejs
Linux e712			# dev-linux
Debian e77d			# dev-debian
Shield eb53			# cod-shield
EOF

CHARSET=$charset awk -v stats="$tmp/stats" '
function dec(h,   i, v) {
	v = 0
	for (i = 1; i <= length(h); i++)
		v = v * 16 + index("0123456789abcdef", tolower(substr(h, i, 1))) - 1
	return v
}
BEGIN {
	n = split(ENVIRON["CHARSET"], token, /[ \t\n]+/)
	count = 0
	for (i = 1; i <= n; i++) {
		if (token[i] == "") continue
		if (index(token[i], "-")) {
			split(token[i], range, "-")
			lo[count] = dec(range[1])
			hi[count] = dec(range[2])
		} else {
			lo[count] = dec(token[i])
			hi[count] = dec(token[i])
		}
		count++
	}
	if (count == 0) {
		print "make-glyphs.sh: the character map is empty" > "/dev/stderr"
		exit 1
	}
	drawn = 0
	fallback = 0
}
$1 == "" || $1 == "#" { next }
{
	variant = $1
	cp = $2
	if (cp == "-") {
		printf "    Entry {\n        glyph: Glyph::%s,\n        draw: None,\n    },\n", variant
		fallback++
		next
	}
	value = dec(cp)
	covered = 0
	for (i = 0; i < count; i++)
		if (value >= lo[i] && value <= hi[i]) {
			covered = 1
			break
		}
	if (!covered) {
		printf "    Entry {\n        glyph: Glyph::%s,\n        draw: None,\n    },\n", variant
		fallback++
		next
	}
	printf "    Entry {\n        glyph: Glyph::%s,\n        draw: Some(\"\\u{%s}\"),\n    },\n", variant, tolower(cp)
	drawn++
}
END {
	print drawn, fallback > stats
}
' "$tmp/candidates" >"$tmp/entries"

read -r drawn fallback <"$tmp/stats"
[ -n "${drawn:-}" ] || die "the generator produced no counts (awk failed)"
[ $((drawn + fallback)) -gt 0 ] || die "the candidate table is empty"

# ---------------------------------------------------------------------------
# table.rs: the data the enum is looked up in.
# ---------------------------------------------------------------------------
{
	cat <<EOF
//! The alphabet table: which codepoint draws which name.
//!
//! **Generated by \`scripts/make-glyphs.sh\`; do not edit.**  The names are the
//! variants of \`Glyph\` in \`glyphs.rs\` and the codepoints are Nerd Fonts
//! v$nerd_version values filtered against the character map of
//! \`FONT_FAMILY\`; a name that map does not carry is \`draw: None\` and its row
//! shows text instead (\`Glyph::token\`).  The comments in the generator say
//! which upstream glyph each codepoint is.
//!
//! The filter is what keeps a row from being an empty box: a codepoint no
//! installed font carries draws nothing at all and logs nothing, which is how
//! the rofi alphabet's tofu shipped once already.

use super::Glyph;

/// The font the alphabet was filtered against.  \`lazyos --glyph-font\` prints
/// these values and \`scripts/check-system.sh\` compares them with the font
/// installed on the machine: a font upgrade that drops a codepoint has to fail
/// the report, not turn a row into a hole.
pub const FONT_FAMILY: &str = "$family";
pub const FONT_FILE: &str = "$base";
pub const FONT_SHA256: &str = "$sum";

/// The Nerd Fonts release the candidate codepoints come from.
pub const NERD_VERSION: &str = "$nerd_version";

/// How many names the font draws, and how many fall back to text.
pub const DRAWN: usize = $drawn;
pub const FALLBACK: usize = $fallback;

pub(crate) struct Entry {
    pub glyph: Glyph,
    /// The codepoint as an escaped Rust string (the source stays ASCII: a
    /// literal private-use character does not survive every editor, which this
    /// repository has already shipped once).
    pub draw: Option<&'static str>,
}

pub(crate) const TABLE: &[Entry] = &[
EOF
	cat "$tmp/entries"
	printf '];\n'
} >"$tmp/table.rs"

cat >"$tmp/font.pin" <<EOF
# The font the TUI alphabet is drawn from (generated by scripts/make-glyphs.sh).
# \`lazyos --glyph-font\` prints the same values, so scripts/check-system.sh can
# compare them with the font installed on the machine: a font upgrade that drops
# a codepoint turns that row into an empty box, and nothing logs it.
family=$family
file=$base
sha256=$sum
nerd=$nerd_version
drawn=$drawn
fallback=$fallback
EOF

if [ "$check" = yes ]; then
	stale=no
	if [ ! -f "$out" ] || ! cmp -s "$tmp/table.rs" "$out"; then
		echo "make-glyphs.sh: $out is stale" >&2
		diff -u "$out" "$tmp/table.rs" 2>/dev/null | sed -n '1,40p' >&2 || true
		stale=yes
	fi
	if [ ! -f "$sidecar" ] || ! cmp -s "$tmp/font.pin" "$sidecar"; then
		echo "make-glyphs.sh: $sidecar is stale" >&2
		diff -u "$sidecar" "$tmp/font.pin" 2>/dev/null | sed -n '1,40p' >&2 || true
		stale=yes
	fi
	[ "$stale" = no ] || exit 1
	echo "make-glyphs.sh: $out and $sidecar are current ($drawn drawn, $fallback text)"
	exit 0
fi

mkdir -p "$(dirname "$out")" "$(dirname "$sidecar")"
cp "$tmp/table.rs" "$out"
cp "$tmp/font.pin" "$sidecar"

echo "make-glyphs.sh: $drawn codepoints, $fallback text fallbacks from $family -> $out"
echo "make-glyphs.sh: pinned $(basename "$font") ($sum) -> $sidecar"
