"""Subset IBM Plex into the renamed WOFF fonts and glyph metrics the README art embeds.

Usage: python3 docs/readme/fonts/subset.py DIR, where DIR holds IBM Plex TTFs
(Google Fonts static files). Requires fontTools. Run from the repository root.
"""
import json
import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

# OFL reserves the name "Plex", so modified (subset) copies take new names.
FONTS = {
    'display': ('IBMPlexSansCondensed-SemiBold.ttf', 'Readme Display'),
    'body': ('IBMPlexSans-Regular.ttf', 'Readme Body'),
    'mono': ('IBMPlexMono-Medium.ttf', 'Readme Mono'),
}
TEXT = ''.join(map(chr, range(0x20, 0x7F))) + '·–—’…→'

out = Path(__file__).parent
metrics = {}
for key, (file, family) in FONTS.items():
    font = TTFont(Path(sys.argv[1]) / file)
    options = subset.Options(flavor='woff', layout_features=['kern', 'liga'], hinting=False, desubroutinize=True)
    subsetter = subset.Subsetter(options)
    subsetter.populate(text=TEXT)
    subsetter.subset(font)
    names = font['name']
    for record in list(names.names):
        if record.nameID in (1, 3, 4, 6, 16, 17):
            names.removeNames(nameID=record.nameID)
    for name_id, value in ((1, family), (4, family), (6, family.replace(' ', '-')), (3, family)):
        names.setName(value, name_id, 3, 1, 0x409)
    font.flavor = 'woff'
    font.save(out / f'{key}.woff')
    # Advance widths in em units, for wrapping and placing text in the SVGs.
    cmap, hmtx, upm = font.getBestCmap(), font['hmtx'], font['head'].unitsPerEm
    metrics[key] = {chr(c): round(hmtx[g][0] / upm, 4) for c, g in sorted(cmap.items())}
(out / 'metrics.json').write_text(json.dumps(metrics, ensure_ascii=False, separators=(',', ':')) + '\n')
