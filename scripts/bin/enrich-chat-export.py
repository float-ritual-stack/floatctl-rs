#!/usr/bin/env python3
"""
Chat export enrichment v2: LLM-first, no regex content parsing.
Split by turn boundaries → claude -p haiku per turn → rebuild with metadata.
"""
import subprocess, json, sys, re
from pathlib import Path

def split_turns(lines):
    """Split on ^user:: and ^response:: only. No content parsing."""
    turns = []
    current = None
    for i, line in enumerate(lines):
        if line.startswith('user::') or line.startswith('response::'):
            if current:
                current['end'] = i - 1
                current['text'] = '\n'.join(lines[current['start']:i])[:3000]  # cap at 3K chars
                turns.append(current)
            role = 'user' if line.startswith('user::') else 'assistant'
            current = {'role': role, 'start': i, 'end': None, 'line': i + 1}
    if current:
        current['end'] = len(lines) - 1
        current['text'] = '\n'.join(lines[current['start']:current['end']+1])[:3000]
        turns.append(current)
    return turns

def enrich_turn(turn, idx, total):
    """Call claude -p haiku to metadata one turn."""
    prompt = f"""You are tagging a conversation turn with metadata. Output ONLY valid JSON, nothing else.

TURN {idx+1}/{total} ({turn['role']}, line {turn['line']}):
---
{turn['text'][:2000]}
---

Output JSON with these fields (use null if unclear):
- project: the project being discussed (e.g. "floatty", "rangle/pharmacy", "float-hub", "float-infra")
- mode: what kind of work (e.g. "building", "pondering", "meeting", "debugging", "review", "break", "exploration", "boot")
- topic: specific subject (e.g. "render-agent", "claude-mem", "explorer-artifact", "assessment-ux", "qmd-search", "agentic-loop")
- summary: one-line summary of this turn (max 80 chars)
- is_seam: true if this starts a new topic/section (## header, project shift, break)

JSON only:"""

    try:
        result = subprocess.run(
            ['claude', '-p', '--model', 'haiku', '--max-turns', '1'],
            input=prompt,
            capture_output=True, text=True, timeout=30
        )
        raw = result.stdout.strip()
        # Extract JSON from response
        json_match = re.search(r'\{[^{}]*\}', raw, re.DOTALL)
        if json_match:
            return json.loads(json_match.group())
        return {"error": "no json", "raw": raw[:200]}
    except Exception as e:
        return {"error": str(e)}

def main():
    fpath = sys.argv[1]
    lines = Path(fpath).read_text().split('\n')
    print(f"Lines: {len(lines)}")

    turns = split_turns(lines)
    print(f"Turns: {len(turns)}")

    # Only enrich user turns (assistant turns inherit from preceding user)
    user_turns = [t for t in turns if t['role'] == 'user']
    print(f"User turns to enrich: {len(user_turns)}")
    
    # Process
    results = []
    for i, turn in enumerate(user_turns):
        meta = enrich_turn(turn, i, len(user_turns))
        results.append({**turn, 'meta': meta})
        label = meta.get('summary', meta.get('error', '?'))[:60]
        proj = meta.get('project') or '?'
        mode = meta.get('mode') or '?'
        seam = ' [SEAM]' if meta.get('is_seam') else ''
        print(f"  [{i+1:>2}/{len(user_turns)}] L{turn['line']:>5}  {proj:<20} {mode:<15} {label}{seam}")

    # Build index
    seams = [r for r in results if r['meta'].get('is_seam')]
    projects = {}
    modes = {}
    topics = {}
    for r in results:
        m = r['meta']
        if m.get('project'): projects[m['project']] = projects.get(m['project'], 0) + 1
        if m.get('mode'): modes[m['mode']] = modes.get(m['mode'], 0) + 1
        if m.get('topic'): topics[m['topic']] = topics.get(m['topic'], 0) + 1

    print(f"\n--- RESULTS ---")
    print(f"Projects: {json.dumps(projects)}")
    print(f"Modes: {json.dumps(modes)}")
    print(f"Topics: {json.dumps(topics)}")
    print(f"Seams: {len(seams)}")
    for s in seams:
        print(f"  L{s['line']:>5}  {s['meta'].get('summary', '?')}")

    # Write YAML index + enriched file
    out_path = fpath.replace('.md', '.v2-enriched.md')
    
    # Build marker→line index from results
    marker_index = {}
    for r in results:
        m = r['meta']
        for key in ['project', 'mode', 'topic']:
            if m.get(key):
                mk = f"{key}::{m[key]}"
                marker_index.setdefault(mk, []).append(r['line'])

    yml_lines = [
        '---',
        f'turns: {len(turns)}',
        f'user_turns: {len(user_turns)}',
        f'lines: {len(lines)}',
        f'seams: {len(seams)}',
        f'enriched_via: claude-haiku',
        '',
        'marker_index:'
    ]
    for mk, lns in sorted(marker_index.items()):
        yml_lines.append(f'  {mk}: [{", ".join(str(l) for l in lns)}]')
    
    yml_lines.append('')
    yml_lines.append('seam_map:')
    for s in seams:
        yml_lines.append(f'  - line: {s["line"]}')
        yml_lines.append(f'    summary: "{s["meta"].get("summary", "?")}"')
        yml_lines.append(f'    project: {s["meta"].get("project", "?")}')
    
    yml_lines.append('')
    yml_lines.append('turn_summaries:')
    for r in results:
        m = r['meta']
        yml_lines.append(f'  - line: {r["line"]}')
        yml_lines.append(f'    summary: "{m.get("summary", "?")}"')
    
    yml_lines.append('---')

    # Write enriched file with inline metadata
    turn_starts = {t['start']: t for t in results}
    with open(out_path, 'w') as f:
        f.write('\n'.join(yml_lines) + '\n\n')
        for i, line in enumerate(lines):
            if i in turn_starts:
                t = turn_starts[i]
                m = t['meta']
                tags = []
                for key in ['project', 'mode', 'topic']:
                    if m.get(key): tags.append(f'[{key}::{m[key]}]')
                if tags:
                    f.write(f'{line}  {" ".join(tags)}\n')
                    continue
            f.write(line + '\n')

    print(f"\nWritten: {out_path}")
    
    # Also write JSON for downstream use
    json_path = fpath.replace('.md', '.enrichment.json')
    with open(json_path, 'w') as f:
        json.dump({
            'source': fpath,
            'turns': len(turns),
            'user_turns': len(user_turns),
            'seams': [{'line': s['line'], **s['meta']} for s in seams],
            'results': [{'line': r['line'], 'role': r['role'], **r['meta']} for r in results],
            'marker_index': marker_index,
        }, f, indent=2)
    print(f"Written: {json_path}")

if __name__ == '__main__':
    main()
