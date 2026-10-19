"""Regenerate historical benchmark SVGs: python plot.py (requires Matplotlib)."""
import csv
from pathlib import Path

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.patches import Patch

ROOT = Path(__file__).resolve().parent
with (ROOT / '20261019.csv').open() as source:
    rows = list(csv.DictReader(source))
plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 11,
                     'svg.fonttype': 'none', 'axes.spines.top': False,
                     'axes.spines.right': False})
colors = ['#94a3b8', '#2563eb']
modes = ['io_uring buffered', 'direct, managed']
metrics = [('mib_per_second', 'Throughput · higher is better', 'MiB/s', 1800, 1),
           ('cpu_ms', 'Process CPU · lower is better', 'ms per 64 MiB', 45, 1),
           ('max_record_us', 'Max record latency · lower is better', 'ms · maximum of 64 samples', 19, .001)]
for platform in ['x86_64', 'ARM64']:
    fig, axes = plt.subplots(1, 3, figsize=(14, 4.7))
    fig.patch.set_facecolor('#ffffff')
    for ax, (field, title, unit, limit, scale) in zip(axes, metrics):
        ax.set_title(title, loc='left', fontsize=12, pad=15, weight='bold')
        ax.set_ylabel(unit)
        ax.set_ylim(0, limit)
        ax.set_axisbelow(True)
        ax.grid(axis='y', color='#e2e8f0')
        for mode, color, shift in zip(modes, colors, [-.2, .2]):
            subset = [r for r in rows if r['platform'] == platform and r['mode'] == mode]
            vals = [float(r[field]) * scale for r in subset]
            bars = ax.bar([i + shift for i in range(3)], vals, width=.36, color=color)
            labels = [f'{v:.0f}' if field == 'mib_per_second' else f'{v:.1f}' if field == 'cpu_ms' else f'{v:.2f}' for v in vals]
            ax.bar_label(bars, labels=labels, padding=4, fontsize=9)
        ax.set_xticks(range(3), ['64 KiB', '128 KiB', '256 KiB'])
        ax.set_xlabel('Initial pool allocation size')
    fig.suptitle(f'{platform}  |  Historical io_uring measurements', x=.055, ha='left', fontsize=18, weight='bold')
    fig.legend([Patch(color=c) for c in colors], ['Buffered io_uring (historical)', 'Direct · managed, pre-registered'],
               loc='upper left', bbox_to_anchor=(.045, .91), ncol=2, frameon=False)
    fig.text(.055, .025, '64 × 1-MiB records · debug build · zero warmed allocations · single run; cache/order effects apply',
             fontsize=10, color='#475569')
    fig.subplots_adjust(left=.065, right=.985, bottom=.19, top=.71, wspace=.34)
    svg = ROOT / f'20261019-{platform}.svg'
    fig.savefig(svg)
    svg.write_text('\n'.join(line.rstrip() for line in svg.read_text().splitlines()) + '\n')
    fig.savefig(f'/tmp/doramcap-benchmark-{platform}.png', dpi=120)
    plt.close(fig)
