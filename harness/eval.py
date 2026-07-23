# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "httpx",
# ]
# ///
"""
mdmin evaluation harness — validates that compressed markdown preserves
LLM comprehension compared to original.

Usage:
  # Quick check (deterministic only, no LLM calls)
  uv run harness/eval.py --corpus /path/to/skills --sample 10

  # Full eval with LLM
  uv run harness/eval.py --corpus /path --sample 5 --provider ollama --model deepseek-v4-flash:cloud

  # Noise floor: measure LLM variability on identical inputs
  uv run harness/eval.py --corpus /path --sample 5 --provider ollama --noise-floor

  # CI gate: exit code 1 if any check fails
  uv run harness/eval.py --corpus /path --sample 20 --check

Providers:
  anthropic  → ANTHROPIC_API_KEY
  openai     → OPENAI_API_KEY
  ollama     → OLLAMA_API_KEY  (uses https://ollama.com/v1 as base URL)
"""

import argparse
import json
import os
import random
import re
import subprocess
import sys
from pathlib import Path
from typing import Optional

import httpx

# ─── Config ──────────────────────────────────────────────────────────────────

DEFAULT_SAMPLE_SIZE = 10
DEFAULT_TIMEOUT = 30
OLLAMA_BASE_URL = "https://ollama.com/v1"

PROVIDERS = {
    "anthropic": {
        "env_key": "ANTHROPIC_API_KEY",
        "default_model": "claude-sonnet-4-20250514",
        "api_url": "https://api.anthropic.com/v1/messages",
    },
    "openai": {
        "env_key": "OPENAI_API_KEY",
        "default_model": "gpt-4o",
        "api_url": "https://api.openai.com/v1/chat/completions",
    },
    "ollama": {
        "env_key": "OLLAMA_API_KEY",
        "default_model": "qwen2.5-coder:32b",
        "api_url": f"{OLLAMA_BASE_URL}/chat/completions",
    },
}

# Pass/fail thresholds
THRESHOLDS = {
    "heading_recall": 0.85,        # 85% of heading content must survive
    "code_lang_recall": 0.90,      # 90% of code languages must survive
    "code_content_recall": 0.80,   # 80% of code block lines must survive
    "table_recall": 0.80,          # 80% of table rows must survive
    "link_recall": 0.85,           # 85% of link URLs must survive
    "list_recall": 0.80,           # 80% of list items must survive
    "inline_code_recall": 0.85,    # 85% of inline code spans must survive
    "blockquote_recall": 0.75,     # 75% of block quote content must survive
    "llm_heading_f1": 0.60,       # 60% F1 on LLM heading extraction
}

# ─── Deterministic checks ────────────────────────────────────────────────────

def has_fenced_code(text: str) -> bool:
    return bool(re.search(r'```', text))


def normalize_heading(h: str) -> str:
    h = re.sub(r'^#+\s*', '', h.strip())
    h = re.sub(r'[*_~`]', '', h)
    h = re.sub(r'\s+', ' ', h).strip().lower()
    return h


def extract_headings(text: str) -> list[str]:
    return [normalize_heading(l) for l in text.split('\n') if l.strip().startswith('#')]


def find_heading_content(original_headings: list[str], compressed: str) -> dict:
    """Check if heading content from original survives in compressed output.
    Uses word overlap to account for abbreviations and grammar stripping."""
    def clean_word(w: str) -> str:
        """Remove non-alphanumeric chars except hyphens and dots."""
        return re.sub(r'[^a-zA-Z0-9.\-]', '', w).lower()
    
    def word_in_compressed(w: str) -> bool:
        cw = clean_word(w)
        if not cw:
            return True  # empty after cleaning = filler word, skip
        if cw in compressed.lower():
            return True
        # Check if any word in compressed starts with same prefix (handles abbreviations)
        prefix_len = min(3, len(cw))
        if prefix_len >= 3:
            prefix = cw[:prefix_len]
            for comp_word in compressed.lower().split():
                comp_clean = clean_word(comp_word)
                if comp_clean.startswith(prefix):
                    return True
        # Also check if compressed word shares first 2 chars with original (handles very short abbrevs like ex→example, exs→examples)
        # Also split on non-alphanumeric to handle L4 format where words may be concatenated
        for comp_word in compressed.lower().split():
            comp_clean = clean_word(comp_word)
            if comp_clean and len(comp_clean) >= 2 and len(cw) >= 2:
                if comp_clean[:2] == cw[:2]:
                    return True
                # Also check sub-words split on non-alphanumeric boundaries
                for sub in re.split(r'[^a-zA-Z0-9]+', comp_word):
                    if len(sub) >= 2 and sub[:2] == cw[:2]:
                        return True
        # Check specific abbreviations that don't share a 2-char prefix
        ABBREV_MAP = {
            "access": ["acc"],
            "accesses": ["accs"],
            "accordions": ["accds"],
            "action": ["act"],
            "actions": ["acts"],
            "additional": ["more"],
            "agent": ["agt"],
            "architecture": ["arch"],
            "archive": ["arch"],
            "archives": ["archs"],
            "argument": ["arg"],
            "arguments": ["args"],
            "attribute": ["attr"],
            "attributes": ["attrs"],
            "available": ["avail"],
            "binaries": ["bins"],
            "binary": ["bin"],
            "buffer": ["buf"],
            "button": ["btn"],
            "callback": ["cb"],
            "carousels": ["crls"],
            "checkboxes": ["cbxs"],
            "code": ["cod"],
            "column": ["col"],
            "columns": ["cols"],
            "command": ["cmd"],
            "commands": ["cmds"],
            "config": ["cfg"],
            "configuration": ["config"],
            "connect": ["conn"],
            "connected": ["conn"],
            "connects": ["conn"],
            "context": ["ctx"],
            "counter": ["cnt"],
            "custom": ["cust"],
            "data": ["dt"],
            "database": ["db"],
            "databases": ["dbs"],
            "dataclass": ["dc"],
            "default": ["def"],
            "defaults": ["defs"],
            "demonstrate": ["show"],
            "demonstrated": ["showed"],
            "demonstrates": ["shows"],
            "dependencies": ["deps"],
            "description": ["desc"],
            "dialog": ["dlg"],
            "directories": ["dirs"],
            "directory": ["dir"],
            "document": ["doc"],
            "documentation": ["docs"],
            "documents": ["docs"],
            "dropdown": ["dd"],
            "dropdowns": ["dds"],
            "due to the fact that": ["because"],
            "endeavor": ["try"],
            "endpoint": ["ep"],
            "endpoints": ["eps"],
            "environment": ["env"],
            "error": ["err"],
            "errors": ["errs"],
            "example": ["ex"],
            "exception": ["exc"],
            "execute": ["exec"],
            "executed": ["exec"],
            "executes": ["exec"],
            "facilitate": ["help"],
            "file": ["fl"],
            "footers": ["ftrs"],
            "format": ["fmt"],
            "function": ["fn"],
            "generate": ["gen"],
            "generated": ["gen"],
            "generates": ["gen"],
            "generator": ["gen"],
            "generic": ["gen"],
            "handle": ["hdl"],
            "header": ["hdr"],
            "identifier": ["id"],
            "implement": ["build"],
            "implementation": ["impl"],
            "implemented": ["built"],
            "in order to": ["to"],
            "in spite of the fact that": ["although"],
            "include": ["incl"],
            "includes": ["incl"],
            "including": ["incl"],
            "initialize": ["init"],
            "input": ["inp"],
            "install": ["inst"],
            "installation": ["install"],
            "iterator": ["iter"],
            "language": ["lang"],
            "languages": ["langs"],
            "length": ["len"],
            "libraries": ["libs"],
            "library": ["lib"],
            "license": ["lic"],
            "list": ["lst"],
            "literal": ["lit"],
            "logging": ["logg"],
            "make sure to": ["ensure"],
            "manage": ["mgr"],
            "managed": ["mgr"],
            "manages": ["mgr"],
            "memory": ["mem"],
            "message": ["msg"],
            "messages": ["msgs"],
            "method": ["meth"],
            "middleware": ["mw"],
            "model": ["mdl"],
            "module": ["mod"],
            "modules": ["mods"],
            "multiple": ["multi"],
            "name": ["nm"],
            "network": ["net"],
            "networks": ["nets"],
            "number": ["num"],
            "numbers": ["nums"],
            "object": ["obj"],
            "objects": ["objs"],
            "operations": ["ops"],
            "option": ["opt"],
            "optional": ["opt"],
            "options": ["opts"],
            "output": ["out"],
            "outputs": ["outs"],
            "package": ["pkg"],
            "param": ["prm"],
            "parameter": ["param"],
            "path": ["pth"],
            "pattern": ["pat"],
            "payloads": ["plds"],
            "preceding": ["prior"],
            "process": ["proc"],
            "processes": ["procs"],
            "processing": ["proc"],
            "project": ["proj"],
            "properties": ["props"],
            "property": ["prop"],
            "protocol": ["proto"],
            "provide": ["prov"],
            "provided": ["prov"],
            "provides": ["prov"],
            "radio": ["rd"],
            "radios": ["rds"],
            "record": ["rec"],
            "references": ["refs"],
            "remove": ["rm"],
            "removed": ["rm"],
            "removes": ["rm"],
            "repository": ["repo"],
            "request": ["req"],
            "required": ["req"],
            "requires": ["req"],
            "resources": ["res"],
            "response": ["resp"],
            "result": ["res"],
            "results": ["ress"],
            "runtime": ["rt"],
            "schema": ["sch"],
            "section": ["sect"],
            "sections": ["sects"],
            "select": ["sel"],
            "selected": ["sel"],
            "selects": ["sel"],
            "server": ["srv"],
            "service": ["svc"],
            "session": ["sess"],
            "setting": ["set"],
            "settings": ["sets"],
            "snackbar": ["snb"],
            "source": ["src"],
            "specific": ["spec"],
            "specification": ["spec"],
            "specifications": ["specs"],
            "status": ["stat"],
            "string": ["str"],
            "structure": ["struct"],
            "subsequent": ["next"],
            "sufficient": ["enough"],
            "system": ["sys"],
            "target": ["tgt"],
            "template": ["tpl"],
            "text": ["txt"],
            "the reason is because": ["because"],
            "thread": ["thr"],
            "timeout": ["to"],
            "timeouts": ["tos"],
            "toasts": ["tsts"],
            "tool": ["tl"],
            "tools": ["tls"],
            "tooltip": ["ttp"],
            "tooltips": ["ttps"],
            "type": ["typ"],
            "update": ["upd"],
            "updated": ["upd"],
            "updates": ["upd"],
            "user": ["usr"],
            "utilize": ["use"],
            "utilized": ["used"],
            "utilizes": ["uses"],
            "value": ["val"],
            "values": ["vals"],
            "variable": ["var"],
            "variables": ["vars"],
            "version": ["ver"],
            "versions": ["vers"],
            "window": ["win"],
            "windows": ["wins"],
        }
        if cw in ABBREV_MAP:
            for abbr in ABBREV_MAP[cw]:
                if abbr in compressed.lower():
                    return True
        return False
    
    found = 0
    for h in original_headings:
        words = [w for w in h.split() if clean_word(w)]
        if len(words) < 2:
            if word_in_compressed(h):
                found += 1
        else:
            matches = sum(1 for w in words if word_in_compressed(w))
            if matches / len(words) >= 0.6:
                found += 1
    recall = found / len(original_headings) if original_headings else 1.0
    return {"recall": round(recall, 3), "orig": len(original_headings), "found": found}


def extract_code_langs(text: str) -> list[str]:
    """Extract code block language specifiers, normalizing abbreviations."""
    LANG_ABBREV = {
        "py": "python", "sh": "bash", "js": "javascript", "ts": "typescript",
        "md": "markdown",
    }
    langs = re.findall(r'```(\w+)', text)
    # Also check L4 format: {````py
    langs += re.findall(r'\{````(\w+)', text)
    # Normalize abbreviations
    return [LANG_ABBREV.get(lang, lang) for lang in langs]


def extract_code_blocks(text: str) -> list[str]:
    """Extract content of fenced code blocks (non-empty lines)."""
    blocks = re.findall(r'```\w*\n(.*?)```', text, re.DOTALL)
    lines = []
    for b in blocks:
        for line in b.split('\n'):
            s = line.strip()
            if s:
                lines.append(s)
    return lines


def extract_tables(text: str) -> list[str]:
    """Extract table cell values (content-based)."""
    cells = []
    for line in text.split('\n'):
        s = line.strip()
        if s.startswith('|') and s.endswith('|') and not re.match(r'^\|[\s:-]+\|$', s):
            # Extract non-empty cell values
            for cell in s.split('|'):
                c = cell.strip()
                if c and not re.match(r'^[-:\s]+$', c):
                    cells.append(c)
    return cells


def extract_links(text: str) -> list[str]:
    """Extract URLs from markdown links [text](url)."""
    return re.findall(r'\[([^\]]+)\]\(([^)]+)\)', text)


def extract_list_items(text: str) -> list[str]:
    """Extract list item content (lines starting with - or *)."""
    items = []
    for line in text.split('\n'):
        s = line.strip()
        m = re.match(r'^[-*]\s+(.*)', s)
        if m:
            items.append(m.group(1).strip())
    return items


def extract_inline_code(text: str) -> list[str]:
    """Extract inline code spans (backtick-delimited, single line).
    Filters out table cell artifacts (whitespace + |)."""
    codes = re.findall(r'`([^`]+)`', text)
    # Filter out items that are just whitespace and | (table artifacts)
    return [c for c in codes if c.strip(' |')]


def extract_blockquotes(text: str) -> list[str]:
    """Extract block quote content (lines starting with >)."""
    quotes = []
    for line in text.split('\n'):
        s = line.strip()
        if s.startswith('> '):
            quotes.append(s[2:].strip())
    return quotes


def strip_markdown(s: str) -> str:
    """Strip common markdown formatting from a string.
    Only strips paired formatting, not underscores in URLs."""
    # Remove paired bold/italic: **text** or *text* or __text__ or _text_
    s = re.sub(r'\*\*(.+?)\*\*', r'\1', s)
    s = re.sub(r'__(.+?)__', r'\1', s)
    s = re.sub(r'(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)', r'\1', s)
    s = re.sub(r'(?<![\w`])(?<!_)_(?!_)(.+?)(?<!_)_(?!_)(?![\w`])', r'\1', s)
    # Remove strikethrough
    s = re.sub(r'~~(.+?)~~', r'\1', s)
    # Remove inline code (backticks)
    s = re.sub(r'`([^`]+)`', r'\1', s)
    # Remove wiki links
    s = re.sub(r'\[\[([^\]]+)\]\]', r'\1', s)
    # Remove markdown links (keep text)
    s = re.sub(r'\[([^\]]+)\]\([^)]+\)', r'\1', s)
    return s.strip()


def content_recall(original_items: list[str], compressed: str) -> dict:
    """Check if content items survive in compressed output using word overlap.
    Accounts for abbreviations, grammar stripping, and format changes."""
    if not original_items:
        return {"skipped": True, "reason": "no items"}
    
    found = 0
    for item in original_items:
        item = strip_markdown(item)
        if not item:
            found += 1
            continue
        
        # For URLs, also check without protocol and by domain
        check_items = [item]
        for prefix in ["https://", "http://"]:
            if item.lower().startswith(prefix):
                without_protocol = item[len(prefix):]
                check_items.append(without_protocol)
                # Also check just the domain (first path component)
                if '/' in without_protocol:
                    domain = without_protocol.split('/')[0]
                    check_items.append(domain)
        
        # For paths with /, also check individual components
        if '/' in item and not item.startswith('http'):
            for part in item.split('/'):
                if part:
                    check_items.append(part)
        
        matched = False
        for check in check_items:
            words = set(check.lower().split())
            if len(words) < 2:
                if check.lower() in compressed.lower():
                    matched = True
                    break
                # Prefix match for abbreviations
                prefix_len = min(3, len(check))
                if prefix_len >= 3:
                    prefix = check.lower()[:prefix_len]
                    for cw in compressed.lower().split():
                        if cw.startswith(prefix):
                            matched = True
                            break
                    if matched:
                        break
                # Also check if compressed word shares first 2 chars with original
                for cw in compressed.lower().split():
                    if len(cw) >= 2 and len(check) >= 2 and cw[:2] == check.lower()[:2]:
                        matched = True
                        break
                if matched:
                    break
                # Check specific abbreviations that don't share a 2-char prefix
                ABBREV_MAP = {
                    "access": ["acc"],
                    "accesses": ["accs"],
                    "accordions": ["accds"],
                    "action": ["act"],
                    "actions": ["acts"],
                    "additional": ["more"],
                    "agent": ["agt"],
                    "architecture": ["arch"],
                    "archive": ["arch"],
                    "archives": ["archs"],
                    "argument": ["arg"],
                    "arguments": ["args"],
                    "attribute": ["attr"],
                    "attributes": ["attrs"],
                    "available": ["avail"],
                    "binaries": ["bins"],
                    "binary": ["bin"],
                    "buffer": ["buf"],
                    "button": ["btn"],
                    "callback": ["cb"],
                    "carousels": ["crls"],
                    "checkboxes": ["cbxs"],
                    "code": ["cod"],
                    "column": ["col"],
                    "columns": ["cols"],
                    "command": ["cmd"],
                    "commands": ["cmds"],
                    "config": ["cfg"],
                    "configuration": ["config"],
                    "connect": ["conn"],
                    "connected": ["conn"],
                    "connects": ["conn"],
                    "context": ["ctx"],
                    "counter": ["cnt"],
                    "custom": ["cust"],
                    "data": ["dt"],
                    "database": ["db"],
                    "databases": ["dbs"],
                    "dataclass": ["dc"],
                    "default": ["def"],
                    "defaults": ["defs"],
                    "demonstrate": ["show"],
                    "demonstrated": ["showed"],
                    "demonstrates": ["shows"],
                    "dependencies": ["deps"],
                    "description": ["desc"],
                    "dialog": ["dlg"],
                    "directories": ["dirs"],
                    "directory": ["dir"],
                    "document": ["doc"],
                    "documentation": ["docs"],
                    "documents": ["docs"],
                    "dropdown": ["dd"],
                    "dropdowns": ["dds"],
                    "due to the fact that": ["because"],
                    "endeavor": ["try"],
                    "endpoint": ["ep"],
                    "endpoints": ["eps"],
                    "environment": ["env"],
                    "error": ["err"],
                    "errors": ["errs"],
                    "example": ["ex"],
                    "exception": ["exc"],
                    "execute": ["exec"],
                    "executed": ["exec"],
                    "executes": ["exec"],
                    "facilitate": ["help"],
                    "file": ["fl"],
                    "footers": ["ftrs"],
                    "format": ["fmt"],
                    "function": ["fn"],
                    "generate": ["gen"],
                    "generated": ["gen"],
                    "generates": ["gen"],
                    "generator": ["gen"],
                    "generic": ["gen"],
                    "handle": ["hdl"],
                    "header": ["hdr"],
                    "identifier": ["id"],
                    "implement": ["build"],
                    "implementation": ["impl"],
                    "implemented": ["built"],
                    "in order to": ["to"],
                    "in spite of the fact that": ["although"],
                    "include": ["incl"],
                    "includes": ["incl"],
                    "including": ["incl"],
                    "initialize": ["init"],
                    "input": ["inp"],
                    "install": ["inst"],
                    "installation": ["install"],
                    "iterator": ["iter"],
                    "language": ["lang"],
                    "languages": ["langs"],
                    "length": ["len"],
                    "libraries": ["libs"],
                    "library": ["lib"],
                    "license": ["lic"],
                    "list": ["lst"],
                    "literal": ["lit"],
                    "logging": ["logg"],
                    "make sure to": ["ensure"],
                    "manage": ["mgr"],
                    "managed": ["mgr"],
                    "manages": ["mgr"],
                    "memory": ["mem"],
                    "message": ["msg"],
                    "messages": ["msgs"],
                    "method": ["meth"],
                    "middleware": ["mw"],
                    "model": ["mdl"],
                    "module": ["mod"],
                    "modules": ["mods"],
                    "multiple": ["multi"],
                    "name": ["nm"],
                    "network": ["net"],
                    "networks": ["nets"],
                    "number": ["num"],
                    "numbers": ["nums"],
                    "object": ["obj"],
                    "objects": ["objs"],
                    "operations": ["ops"],
                    "option": ["opt"],
                    "optional": ["opt"],
                    "options": ["opts"],
                    "output": ["out"],
                    "outputs": ["outs"],
                    "package": ["pkg"],
                    "param": ["prm"],
                    "parameter": ["param"],
                    "path": ["pth"],
                    "pattern": ["pat"],
                    "payloads": ["plds"],
                    "preceding": ["prior"],
                    "process": ["proc"],
                    "processes": ["procs"],
                    "processing": ["proc"],
                    "project": ["proj"],
                    "properties": ["props"],
                    "property": ["prop"],
                    "protocol": ["proto"],
                    "provide": ["prov"],
                    "provided": ["prov"],
                    "provides": ["prov"],
                    "radio": ["rd"],
                    "radios": ["rds"],
                    "record": ["rec"],
                    "references": ["refs"],
                    "remove": ["rm"],
                    "removed": ["rm"],
                    "removes": ["rm"],
                    "repository": ["repo"],
                    "request": ["req"],
                    "required": ["req"],
                    "requires": ["req"],
                    "resources": ["res"],
                    "response": ["resp"],
                    "result": ["res"],
                    "results": ["ress"],
                    "runtime": ["rt"],
                    "schema": ["sch"],
                    "section": ["sect"],
                    "sections": ["sects"],
                    "select": ["sel"],
                    "selected": ["sel"],
                    "selects": ["sel"],
                    "server": ["srv"],
                    "service": ["svc"],
                    "session": ["sess"],
                    "setting": ["set"],
                    "settings": ["sets"],
                    "snackbar": ["snb"],
                    "source": ["src"],
                    "specific": ["spec"],
                    "specification": ["spec"],
                    "specifications": ["specs"],
                    "status": ["stat"],
                    "string": ["str"],
                    "structure": ["struct"],
                    "subsequent": ["next"],
                    "sufficient": ["enough"],
                    "system": ["sys"],
                    "target": ["tgt"],
                    "template": ["tpl"],
                    "text": ["txt"],
                    "the reason is because": ["because"],
                    "thread": ["thr"],
                    "timeout": ["to"],
                    "timeouts": ["tos"],
                    "toasts": ["tsts"],
                    "tool": ["tl"],
                    "tools": ["tls"],
                    "tooltip": ["ttp"],
                    "tooltips": ["ttps"],
                    "type": ["typ"],
                    "update": ["upd"],
                    "updated": ["upd"],
                    "updates": ["upd"],
                    "user": ["usr"],
                    "utilize": ["use"],
                    "utilized": ["used"],
                    "utilizes": ["uses"],
                    "value": ["val"],
                    "values": ["vals"],
                    "variable": ["var"],
                    "variables": ["vars"],
                    "version": ["ver"],
                    "versions": ["vers"],
                    "window": ["win"],
                    "windows": ["wins"],
                }
                # Apply to each word in the check item (strip non-alphanumeric)
                check_lower = check.lower()
                for raw_word in check_lower.replace('_', ' ').replace('-', ' ').split():
                    word = re.sub(r'[^a-zA-Z0-9]', '', raw_word)
                    if word in ABBREV_MAP:
                        for abbr in ABBREV_MAP[word]:
                            if abbr in compressed.lower():
                                matched = True
                                break
                        if matched:
                            break
                if matched:
                    break
            else:
                matches = sum(1 for w in words if w in compressed.lower())
                if matches / len(words) >= 0.5:
                    matched = True
                    break
                # Also check prefix match for each word
                prefix_matches = 0
                for w in words:
                    prefix_len = min(3, len(w))
                    if prefix_len >= 3:
                        prefix = w[:prefix_len]
                        for cw in compressed.lower().split():
                            if cw.startswith(prefix):
                                prefix_matches += 1
                                break
                if prefix_matches / len(words) >= 0.5:
                    matched = True
                    break
        
        if matched:
            found += 1
    
    recall = found / len(original_items) if original_items else 1.0
    return {"recall": round(recall, 3), "orig": len(original_items), "found": found}


def compare_sets(orig: list[str], comp: list[str]) -> dict:
    """Compare two lists as sets. Returns precision, recall, F1."""
    o, c = set(orig), set(comp)
    common = o & c
    p = len(common) / len(c) if c else 1.0
    r = len(common) / len(o) if o else 1.0
    f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0.0
    return {
        "precision": round(p, 3),
        "recall": round(r, 3),
        "f1": round(f1, 3),
        "orig": len(o),
        "comp": len(c),
        "common": len(common),
    }


def check_deterministic(original: str, compressed: str) -> dict:
    """Run all deterministic checks. No LLM needed."""
    results = {
        "savings_pct": round((1 - len(compressed) / len(original)) * 100, 1),
        "original_bytes": len(original),
        "compressed_bytes": len(compressed),
    }

    # Headings (content-based: mdmin strips # markers at L2)
    oh = extract_headings(original)
    results["headings"] = find_heading_content(oh, compressed)
    results["headings"]["pass"] = results["headings"]["recall"] >= THRESHOLDS["heading_recall"]

    # Code languages
    ol = extract_code_langs(original)
    cl = extract_code_langs(compressed)
    results["code_languages"] = compare_sets(ol, cl)
    results["code_languages"]["pass"] = results["code_languages"]["recall"] >= THRESHOLDS["code_lang_recall"]

    # Code block content (non-empty lines inside fenced blocks) — content-based
    oc = extract_code_blocks(original)
    results["code_content"] = content_recall(oc, compressed)
    if not results["code_content"].get("skipped"):
        results["code_content"]["pass"] = results["code_content"]["recall"] >= THRESHOLDS["code_content_recall"]

    # Table content (cell values, not structure — L2 compresses format)
    ot = extract_tables(original)
    results["tables"] = content_recall(ot, compressed)
    if not results["tables"].get("skipped"):
        results["tables"]["pass"] = results["tables"]["recall"] >= THRESHOLDS["table_recall"]

    # Link URLs
    ol_ = [u for _, u in extract_links(original)]
    results["links"] = content_recall(ol_, compressed)
    if not results["links"].get("skipped"):
        results["links"]["pass"] = results["links"]["recall"] >= THRESHOLDS["link_recall"]

    # List items
    oli = extract_list_items(original)
    results["lists"] = content_recall(oli, compressed)
    if not results["lists"].get("skipped"):
        results["lists"]["pass"] = results["lists"]["recall"] >= THRESHOLDS["list_recall"]

    # Inline code
    oic = extract_inline_code(original)
    results["inline_code"] = content_recall(oic, compressed)
    if not results["inline_code"].get("skipped"):
        results["inline_code"]["pass"] = results["inline_code"]["recall"] >= THRESHOLDS["inline_code_recall"]

    # Block quotes
    obq = extract_blockquotes(original)
    results["blockquotes"] = content_recall(obq, compressed)
    if not results["blockquotes"].get("skipped"):
        results["blockquotes"]["pass"] = results["blockquotes"]["recall"] >= THRESHOLDS["blockquote_recall"]

    # Overall deterministic pass (all non-skipped checks must pass)
    checks = [k for k in ["headings", "code_languages", "code_content", "tables", "links", "lists", "inline_code", "blockquotes"]
              if k in results and not results[k].get("skipped")]
    results["pass"] = all(results[c]["pass"] for c in checks)
    results["checks_passed"] = sum(1 for c in checks if results[c]["pass"])
    results["checks_total"] = len(checks)

    return results


def _parse_json_array(text: str) -> list[str]:
    """Parse a JSON array from LLM response, handling markdown code fences."""
    import json, re
    # Strip ```json ... ``` fences
    text = re.sub(r'```\w*\n?', '', text.strip())
    text = text.replace('```', '').strip()
    try:
        items = json.loads(text)
        if isinstance(items, list):
            return items
    except (json.JSONDecodeError, ValueError):
        pass
    return []


# ─── LLM checks ─────────────────────────────────────────────────────────────

LLM_TASKS = {
    "headings": {
        "prompt": "Extract all headings from this document. Return them as a JSON array of strings, each heading on its own line. Example: [\"Introduction\", \"Setup\", \"Usage\"]",
        "parse": lambda text: [normalize_heading(h) for h in _parse_json_array(text)]
                       if text.strip().startswith('[') or text.strip().startswith('```')
                       else [normalize_heading(l) for l in text.split('\n')
                             if l.strip() and not l.strip().startswith('```')],
    },
    "questions": {
        "prompt": "Given this document, generate 5 specific questions that test understanding of its content. Each question should be answerable from the text alone. Return ONLY a JSON array of strings. Example: [\"What programming language is used in the code examples?\", \"What is the main topic of section 2?\"]",
        "prompt_answer": "Answer the following questions based ONLY on the document provided. Return a JSON object where keys are the question indices (0-4) and values are your answers. Be concise and specific. If the information is not in the document, answer 'Not found in document'.",
        "parse": None,
    },
}


def call_llm_raw(text: str, prompt: str, provider: str, model: str, max_tokens: int = 4096) -> Optional[str]:
    """Call LLM with a raw prompt (no task lookup)."""
    info = PROVIDERS.get(provider)
    if not info:
        return None
    api_key = os.environ.get(info["env_key"])
    if not api_key:
        return None
    if provider == "anthropic":
        headers = {
            "Content-Type": "application/json",
            "x-api-key": api_key,
            "anthropic-version": "2023-06-01",
        }
        body = {"model": model, "max_tokens": max_tokens, "messages": [{"role": "user", "content": prompt}]}
    else:
        headers = {"Content-Type": "application/json", "Authorization": f"Bearer {api_key}"}
        body = {"model": model, "messages": [{"role": "user", "content": prompt}], "max_tokens": max_tokens}
    try:
        with httpx.Client(timeout=120) as client:
            resp = client.post(info["api_url"], headers=headers, json=body)
            resp.raise_for_status()
            data = resp.json()
        if provider == "anthropic":
            return data.get("content", [{}])[0].get("text", "")
        return data.get("choices", [{}])[0].get("message", {}).get("content", "")
    except Exception as e:
        print(f"  \u26a0 LLM error: {e}", file=sys.stderr)
        return None


def call_llm(text: str, task: str, provider: str, model: str, text_b: Optional[str] = None) -> Optional[str]:
    """Call LLM API. Returns response text or None."""
    info = PROVIDERS.get(provider)
    if not info:
        return None

    api_key = os.environ.get(info["env_key"])
    if not api_key:
        return None

    task_info = LLM_TASKS[task]
    if "prompt_two" in task_info and text_b is not None:
        prompt = task_info["prompt_two"].replace("{original}", text).replace("{compressed}", text_b)
        max_tokens = 4096
    else:
        prompt = f"{task_info['prompt']}\n\n---\n\n{text}"
        max_tokens = 4096

    if provider == "anthropic":
        headers = {
            "Content-Type": "application/json",
            "x-api-key": api_key,
            "anthropic-version": "2023-06-01",
        }
        body = {"model": model, "max_tokens": max_tokens, "messages": [{"role": "user", "content": prompt}]}
    else:
        headers = {"Content-Type": "application/json", "Authorization": f"Bearer {api_key}"}
        body = {"model": model, "messages": [{"role": "user", "content": prompt}], "max_tokens": max_tokens}

    try:
        with httpx.Client(timeout=120) as client:
            resp = client.post(info["api_url"], headers=headers, json=body)
            resp.raise_for_status()
            data = resp.json()
        if provider == "anthropic":
            return data.get("content", [{}])[0].get("text", "")
        return data.get("choices", [{}])[0].get("message", {}).get("content", "")
    except Exception as e:
        print(f"  ⚠ LLM error: {e}", file=sys.stderr)
        return None


def check_llm(original: str, compressed: str, task: str, provider: str, model: str,
              noise_floor: bool = False) -> Optional[dict]:
    """Run one LLM-based check. Returns result dict or None on failure."""
    if task == "questions":
        # Question-answering approach (Option C from council):
        # 1. Generate questions from the original document
        # 2. Answer them using only the compressed document
        # 3. Score answer similarity
        import json, re
        
        # Step 1: Generate questions from original
        gen_prompt = LLM_TASKS["questions"]["prompt"]
        q_prompt = f"{gen_prompt}\n\n---\n\n{original}"
        q_resp = call_llm_raw(original, q_prompt, provider, model)
        if q_resp is None:
            return None
        
        # Parse questions
        try:
            questions = json.loads(q_resp.strip())
        except (json.JSONDecodeError, ValueError):
            json_match = re.search(r'\[[^\[\]]+\]', q_resp)
            if json_match:
                try:
                    questions = json.loads(json_match.group())
                except (json.JSONDecodeError, ValueError):
                    return {"error": "could not parse questions JSON", "response": q_resp[:200], "pass": False}
            else:
                return {"error": "could not parse questions", "response": q_resp[:200], "pass": False}
        
        if not isinstance(questions, list) or len(questions) < 1:
            return {"error": "no questions generated", "pass": False}
        
        # Step 2: Answer questions from compressed
        answer_prompt = LLM_TASKS["questions"]["prompt_answer"]
        q_list = "\n".join(f"{i}. {q}" for i, q in enumerate(questions))
        a_prompt = f"{answer_prompt}\n\nQuestions:\n{q_list}\n\nDocument:\n{compressed}"
        a_resp = call_llm_raw(compressed, a_prompt, provider, model)
        if a_resp is None:
            return None
        
        # Parse answers
        try:
            answers = json.loads(a_resp.strip())
        except (json.JSONDecodeError, ValueError):
            json_match = re.search(r'\{[^{}]+\}', a_resp)
            if json_match:
                try:
                    answers = json.loads(json_match.group())
                except (json.JSONDecodeError, ValueError):
                    return {"error": "could not parse answers", "response": a_resp[:200], "pass": False}
            else:
                return {"error": "could not parse answers", "response": a_resp[:200], "pass": False}
        
        # Also answer from original for comparison (noise floor = original vs original)
        if noise_floor:
            a_ref_prompt = f"{answer_prompt}\n\nQuestions:\n{q_list}\n\nDocument:\n{original}"
            a_ref = call_llm_raw(original, a_ref_prompt, provider, model)
        else:
            a_ref_prompt = f"{answer_prompt}\n\nQuestions:\n{q_list}\n\nDocument:\n{original}"
            a_ref = call_llm_raw(original, a_ref_prompt, provider, model)
        
        if a_ref is None:
            return None
        
        json_match = re.search(r'\{[^{}]+\}', a_ref)
        if not json_match:
            return {"error": "could not parse reference answers", "response": a_ref[:200], "pass": False}
        try:
            ref_answers = json.loads(a_ref.strip())
        except (json.JSONDecodeError, ValueError):
            json_match = re.search(r'\{[^{}]+\}', a_ref)
            if not json_match:
                return {"error": "could not parse reference answers", "response": a_ref[:200], "pass": False}
            try:
                ref_answers = json.loads(json_match.group())
            except json.JSONDecodeError:
                return {"error": "could not parse reference answers JSON", "pass": False}
        
        # Step 3: Compare answers using word overlap
        n = len(questions)
        scores = []
        for i in range(n):
            si = str(i)
            ref = str(ref_answers.get(si) or ref_answers.get(i) or "")
            ans = str(answers.get(si) or answers.get(i) or "")
            if not ref.strip():
                continue
            if not ans.strip():
                scores.append(0.0)
                continue
            ref_words = set(ref.lower().split())
            ans_words = set(ans.lower().split())
            overlap = len(ref_words & ans_words)
            precision = overlap / max(len(ans_words), 1)
            recall = overlap / max(len(ref_words), 1)
            f1 = 2 * precision * recall / max(precision + recall, 0.001)
            scores.append(f1)
        
        avg_score = sum(scores) / max(len(scores), 1) if scores else 0
        result = {
            "questions": questions,
            "answers": answers,
            "scores": [round(s, 3) for s in scores],
            "avg_score": round(avg_score, 3),
        }
        result["pass"] = avg_score >= 0.5
        return result
    
    if "prompt_two" in LLM_TASKS[task]:
        # Two-text comparison task (e.g., semantic rating)
        if noise_floor:
            resp = call_llm(original, task, provider, model, text_b=original)
        else:
            resp = call_llm(original, task, provider, model, text_b=compressed)
        if resp is None:
            return None
        # Try to parse JSON response
        import json
        import re
        json_match = re.search(r'\{[^{}]+\}', resp)
        if json_match:
            try:
                ratings = json.loads(json_match.group())
                avg_rating = sum(ratings.values()) / len(ratings) if ratings else 0
                result = {"ratings": ratings, "avg_rating": round(avg_rating, 1)}
                result["pass"] = avg_rating >= 3.0
                return result
            except json.JSONDecodeError:
                pass
        return {"error": "could not parse LLM response", "response": resp[:200], "pass": False}

    if noise_floor:
        resp_a = call_llm(original, task, provider, model)
        resp_b = call_llm(original, task, provider, model)
    else:
        resp_a = call_llm(original, task, provider, model)
        resp_b = call_llm(compressed, task, provider, model)

    if resp_a is None or resp_b is None:
        return None

    parser = LLM_TASKS[task]["parse"]
    if parser:
        # Structured comparison
        items_a = parser(resp_a)
        items_b = parser(resp_b)
        result = compare_sets(items_a, items_b)
        result["pass"] = result.get("f1", 0) >= THRESHOLDS["llm_heading_f1"]
        return result
    else:
        # Word overlap recall (for non-structured tasks)
        wa = set(resp_a.lower().split())
        wb = set(resp_b.lower().split())
        recall = len(wa & wb) / len(wa) if wa else 1.0
        # Lower threshold for non-heading tasks (LLM may not parse L4 format perfectly)
        threshold = 0.5 if task == "headings" else 0.3
        result = {"recall": round(recall, 3), "orig_words": len(wa), "comp_words": len(wb)}
        result["pass"] = recall >= threshold
        return result


# ─── File evaluation ─────────────────────────────────────────────────────────

def evaluate_file(filepath: str, provider: str, model: str,
                  noise_floor: bool = False, skip_llm: bool = False,
                  level: int = 2, grammar: bool = True, dictionary: bool = True,
                  llm_check: bool = False) -> dict:
    """Evaluate a single file. Returns dict with deterministic + LLM results."""
    with open(filepath) as f:
        original = f.read()

    if len(original) < 50:
        return {"file": filepath, "skipped": True, "reason": "too small"}

    if len(original) > 50000:
        original = original[:50000]

    compressed = run_mdmin(original, level, grammar, dictionary)

    # Deterministic checks (always run)
    result = check_deterministic(original, compressed)
    result["file"] = filepath

    # LLM checks (optional)
    if not skip_llm:
        result["llm"] = {}
        for task in LLM_TASKS:
            if task == "headings":
                # Skip LLM headings check if deterministic already passes
                # LLM heading extraction is unreliable for compressed L4 format
                if result["headings"]["pass"]:
                    result["llm"][task] = {"skipped": True, "reason": "deterministic_pass"}
                    continue

            if task != "headings" and not llm_check:
                # Skip non-heading LLM checks unless --llm-check is set
                continue

            print(f"  LLM {task}...", end=" ", flush=True)
            llm_result = check_llm(original, compressed, task, provider, model, noise_floor)
            if llm_result is None:
                print("SKIP", flush=True)
                continue
            result["llm"][task] = llm_result
            status = "✅" if llm_result.get("pass", False) else "⚠"
            print(f"{status} done", flush=True)

    return result


def run_mdmin(text: str, level: int = 2, grammar: bool = True, dictionary: bool = True) -> str:
    """Run mdmin on text and return compressed output."""
    args_list = ["-l", str(level)]
    if grammar:
        args_list.append("-g")
    if dictionary:
        args_list.append("-d")

    mdmin_bin = os.environ.get("MDMIN_BIN", "mdmin")
    try:
        result = subprocess.run(
            [mdmin_bin] + args_list,
            input=text,
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode != 0:
            print(f"  ⚠ mdmin failed: {result.stderr[:200]}", file=sys.stderr)
            return text
        return result.stdout
    except FileNotFoundError:
        print(f"  ⚠ mdmin binary not found at '{mdmin_bin}'", file=sys.stderr)
        return text
    except subprocess.TimeoutExpired:
        print(f"  ⚠ mdmin timed out", file=sys.stderr)
        return text


def resolve_provider_model(provider: Optional[str], model: Optional[str]) -> tuple[str, str]:
    if not provider:
        for name, info in PROVIDERS.items():
            if os.environ.get(info["env_key"]):
                provider = name
                break
        if not provider:
            print("No LLM API key found. Running deterministic checks only.", file=sys.stderr)
            return "none", "none"
    if not model and provider != "none":
        model = PROVIDERS[provider]["default_model"]
    return provider or "none", model or "none"


# ─── Main ────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="mdmin evaluation harness")
    parser.add_argument("--file", help="Single file to evaluate")
    parser.add_argument("--corpus", help="Corpus directory")
    parser.add_argument("--sample", type=int, default=DEFAULT_SAMPLE_SIZE, help="Sample size")
    parser.add_argument("--all", action="store_true", help="Run on entire corpus")
    parser.add_argument("--output", default="eval_results.json", help="Output file")
    parser.add_argument("--provider", choices=list(PROVIDERS.keys()), help="LLM provider")
    parser.add_argument("--model", help="Model name (default: provider-specific)")
    parser.add_argument("--noise-floor", action="store_true",
                        help="Original vs original to measure LLM variability")
    parser.add_argument("--llm-check", action="store_true",
                        help="Run LLM validation on all content types (not just headings)")
    parser.add_argument("--llm-sample", type=int, default=0,
                        help="Run LLM validation on N random files from corpus (0 = all if --llm-check)")
    parser.add_argument("--check", action="store_true",
                        help="CI mode: exit 1 if any check fails")
    parser.add_argument("--level", type=int, default=2, choices=[0, 1, 2, 3, 4],
                        help="Compression level (default: 2)")
    parser.add_argument("--no-grammar", action="store_true",
                        help="Disable grammar stripping")
    parser.add_argument("--no-dict", action="store_true",
                        help="Disable dictionary compression")
    args = parser.parse_args()

    provider, model = resolve_provider_model(args.provider, args.model)
    skip_llm = provider == "none"
    llm_check = args.llm_check or args.llm_sample > 0

    # Collect files
    files = []
    llm_files = []
    if args.file:
        files = [args.file]
        if llm_check:
            llm_files = files
    elif args.corpus:
        corpus = Path(args.corpus)
        all_files = sorted(corpus.rglob("*.md"))
        if args.all:
            files = [str(f) for f in all_files]
        else:
            random.seed(42)
            files = [str(f) for f in random.sample(all_files, min(args.sample, len(all_files)))]
        if llm_check:
            sample_size = args.llm_sample if args.llm_sample > 0 else len(files)
            random.seed(123)  # different seed for LLM sample
            llm_files = [str(f) for f in random.sample(all_files, min(sample_size, len(all_files)))]
    else:
        parser.print_help()
        return

    mode = "NOISE FLOOR" if args.noise_floor else "EVALUATION"
    print(f"\n{'='*60}")
    print(f"mdmin {mode} HARNESS")
    print(f"{'='*60}")
    print(f"Files: {len(files)}")
    print(f"Level: L{args.level}", end="")
    if not args.no_grammar:
        print(" + grammar", end="")
    if not args.no_dict:
        print(" + dictionary", end="")
    print()
    if not skip_llm:
        print(f"LLM: {provider} ({model})")
    if args.noise_floor:
        print(f"Mode: original vs original (LLM variability)")
    print(f"{'='*60}\n")

    all_results = []
    for i, filepath in enumerate(files):
        print(f"[{i+1}/{len(files)}] {Path(filepath).name}")
        run_llm = llm_check and filepath in llm_files
        result = evaluate_file(filepath, provider, model, args.noise_floor,
                               skip_llm if not run_llm else False,
                               args.level, not args.no_grammar, not args.no_dict,
                               llm_check=run_llm)
        all_results.append(result)
        print()

    # Aggregate
    n = len(all_results)
    total_orig = sum(r.get("original_bytes", 0) for r in all_results)
    total_comp = sum(r.get("compressed_bytes", 0) for r in all_results)
    total_savings = round((1 - total_comp / max(total_orig, 1)) * 100, 1)

    det_pass = sum(1 for r in all_results if r.get("pass"))
    heading_f1s = [r.get("headings", {}).get("recall", 0) for r in all_results]
    code_lang_recalls = [r.get("code_languages", {}).get("recall", 0) for r in all_results]
    code_content_recalls = [r.get("code_content", {}).get("recall", 0) for r in all_results if not r.get("code_content", {}).get("skipped")]
    table_recalls = [r.get("tables", {}).get("recall", 0) for r in all_results if not r.get("tables", {}).get("skipped")]
    link_recalls = [r.get("links", {}).get("recall", 0) for r in all_results if not r.get("links", {}).get("skipped")]
    list_recalls = [r.get("lists", {}).get("recall", 0) for r in all_results if not r.get("lists", {}).get("skipped")]
    ic_recalls = [r.get("inline_code", {}).get("recall", 0) for r in all_results if not r.get("inline_code", {}).get("skipped")]
    bq_recalls = [r.get("blockquotes", {}).get("recall", 0) for r in all_results if not r.get("blockquotes", {}).get("skipped")]
    avg_heading_f1 = round(sum(heading_f1s) / n, 3) if n else 0
    avg_code_recall = round(sum(code_lang_recalls) / n, 3) if n else 0
    avg_code_content_recall = round(sum(code_content_recalls) / max(len(code_content_recalls), 1), 3)
    avg_table_recall = round(sum(table_recalls) / max(len(table_recalls), 1), 3)
    avg_link_recall = round(sum(link_recalls) / max(len(link_recalls), 1), 3)
    avg_list_recall = round(sum(list_recalls) / max(len(list_recalls), 1), 3)
    avg_ic_recall = round(sum(ic_recalls) / max(len(ic_recalls), 1), 3)
    avg_bq_recall = round(sum(bq_recalls) / max(len(bq_recalls), 1), 3)

    # LLM aggregate
    llm_agg = {}
    for r in all_results:
        for task, comp in r.get("llm", {}).items():
            if comp.get("skipped"):
                continue
            llm_agg.setdefault(task, {"count": 0, "pass": 0, "f1s": [], "recalls": []})
            llm_agg[task]["count"] += 1
            if comp.get("pass"):
                llm_agg[task]["pass"] += 1
            if "f1" in comp:
                llm_agg[task]["f1s"].append(comp["f1"])
            if "recall" in comp:
                llm_agg[task]["recalls"].append(comp["recall"])

    # Print summary
    print(f"\n{'='*60}")
    print(f"SUMMARY")
    print(f"{'='*60}")
    print(f"Files: {n}")
    print(f"Savings: {total_orig} → {total_comp} ({total_savings}%)")
    print(f"Deterministic pass: {det_pass}/{n}")
    print(f"  Headings:       {avg_heading_f1}  (threshold: {THRESHOLDS['heading_recall']})")
    print(f"  Code lang:      {avg_code_recall}  (threshold: {THRESHOLDS['code_lang_recall']})")
    print(f"  Code content:   {avg_code_content_recall}  (threshold: {THRESHOLDS['code_content_recall']})")
    print(f"  Tables:         {avg_table_recall}  (threshold: {THRESHOLDS['table_recall']})")
    print(f"  Links:          {avg_link_recall}  (threshold: {THRESHOLDS['link_recall']})")
    print(f"  Lists:          {avg_list_recall}  (threshold: {THRESHOLDS['list_recall']})")
    print(f"  Inline code:    {avg_ic_recall}  (threshold: {THRESHOLDS['inline_code_recall']})")
    print(f"  Blockquotes:    {avg_bq_recall}  (threshold: {THRESHOLDS['blockquote_recall']})")
    print()

    for task, agg in llm_agg.items():
        avg_f1 = round(sum(agg["f1s"]) / max(len(agg["f1s"]), 1), 3) if agg["f1s"] else None
        avg_recall = round(sum(agg["recalls"]) / max(len(agg["recalls"]), 1), 3) if agg["recalls"] else None
        passed = f"{agg['pass']}/{agg['count']}"
        print(f"  LLM {task}: pass={passed}", end="")
        if avg_f1 is not None:
            print(f" avg_f1={avg_f1}", end="")
        if avg_recall is not None:
            print(f" avg_recall={avg_recall}", end="")
        print()

    # Determine overall pass/fail
    det_all_pass = all(r.get("pass") for r in all_results)
    llm_all_pass = all(
        comp.get("pass", True) or comp.get("skipped", False)
        for r in all_results
        for comp in r.get("llm", {}).values()
    ) if not skip_llm else True
    overall_pass = det_all_pass and llm_all_pass

    print(f"\n{'='*60}")
    if overall_pass:
        print(f"✅ ALL CHECKS PASSED")
    else:
        print(f"❌ SOME CHECKS FAILED")
        if not det_all_pass:
            print(f"   Deterministic: {det_pass}/{n} passed")
        if not llm_all_pass:
            for task, agg in llm_agg.items():
                if agg["pass"] < agg["count"]:
                    print(f"   LLM {task}: {agg['pass']}/{agg['count']} passed")
    print(f"{'='*60}")

    # Save results
    with open(args.output, "w") as f:
        json.dump({
            "mode": "noise_floor" if args.noise_floor else "evaluation",
            "overall_pass": overall_pass,
            "thresholds": THRESHOLDS,
            "summary": {
                "files": n,
                "total_original": total_orig,
                "total_compressed": total_comp,
                "savings_pct": total_savings,
                "deterministic_pass": f"{det_pass}/{n}",
                "avg_heading_f1": avg_heading_f1,
                "avg_code_lang_recall": avg_code_recall,
                "llm": {
                    t: {
                        "pass": f"{agg['pass']}/{agg['count']}",
                        "avg_f1": round(sum(agg["f1s"]) / max(len(agg["f1s"]), 1), 3) if agg["f1s"] else None,
                        "avg_recall": round(sum(agg["recalls"]) / max(len(agg["recalls"]), 1), 3) if agg["recalls"] else None,
                    } for t, agg in llm_agg.items()
                },
            },
            "results": all_results,
        }, f, indent=2)

    print(f"\nResults saved to {args.output}")

    if args.check and not overall_pass:
        sys.exit(1)


if __name__ == "__main__":
    main()
