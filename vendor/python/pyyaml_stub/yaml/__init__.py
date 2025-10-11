"""Very small subset of the PyYAML API used for gatecheck automation."""

from __future__ import annotations

import ast
import json
import re
from dataclasses import dataclass
from typing import Any, List, Sequence, Tuple

__all__ = ["safe_load", "safe_dump", "YAMLError"]


class YAMLError(Exception):
    """Base YAML error."""


@dataclass
class _Line:
    text: str
    indent: int


_BOOL_MAP = {
    "true": True,
    "false": False,
    "on": True,
    "off": False,
    "yes": True,
    "no": False,
}


_NULL_VALUES = {"null", "~", "none", ""}


def safe_load(stream: Any) -> Any:
    """Parse a YAML string or file-like object.

    The implementation supports the subset of YAML used within the project:
    nested mappings, lists, inline collection literals, and basic scalars.
    """

    if hasattr(stream, "read"):
        text = stream.read()
    else:
        text = str(stream)
    lines = _preprocess_lines(text)
    parser = _Parser(lines)
    return parser.parse()


def safe_dump(data: Any, *, indent: int = 2) -> str:
    return _Emitter(indent).dump(data)


def _preprocess_lines(text: str) -> List[_Line]:
    result: List[_Line] = []
    for raw_line in text.splitlines():
        stripped = raw_line.split("#", 1)[0].rstrip()
        if not stripped:
            continue
        indent = len(raw_line) - len(raw_line.lstrip(" "))
        result.append(_Line(stripped, indent))
    return result


class _Parser:
    def __init__(self, lines: Sequence[_Line]):
        self.lines = list(lines)
        self.index = 0

    def parse(self) -> Any:
        if not self.lines:
            return None
        value = self._parse_block(0)
        # skip trailing whitespace lines
        return value

    def _parse_block(self, indent: int) -> Any:
        mapping: dict[str, Any] = {}
        sequence: List[Any] = []
        mode: str | None = None

        while self.index < len(self.lines):
            current = self.lines[self.index]
            if current.indent < indent:
                break
            if current.text.startswith("- ") and current.indent == indent:
                if mode == "mapping":
                    break
                mode = "sequence"
                sequence.append(self._parse_sequence_item(indent))
                continue
            if current.indent > indent:
                break
            if mode == "sequence":
                break
            mode = "mapping"
            key, value = self._parse_mapping_entry(indent)
            mapping[key] = value

        if mode == "sequence":
            return sequence
        return mapping

    def _parse_sequence_item(self, indent: int) -> Any:
        current = self.lines[self.index]
        value_str = current.text[2:].strip()
        self.index += 1
        if not value_str:
            return self._parse_block(indent + 2)
        if ":" in value_str and not value_str.startswith("["):
            # treat as an inline mapping; inject synthetic line
            synthetic = _Line(" " * (indent + 2) + value_str, indent + 2)
            self.lines.insert(self.index, synthetic)
            return self._parse_block(indent + 2)
        return _parse_scalar(value_str)

    def _parse_mapping_entry(self, indent: int) -> Tuple[str, Any]:
        current = self.lines[self.index]
        text = current.text
        self.index += 1
        if ":" not in text:
            raise YAMLError(f"invalid mapping entry: {text}")
        key_part, value_part = text.split(":", 1)
        key = key_part.strip()
        value_part = value_part.strip()
        if not value_part:
            value = self._parse_block(indent + 2)
        else:
            value = _parse_scalar(value_part)
        return key, value


class _Emitter:
    def __init__(self, indent: int):
        self.indent = indent

    def dump(self, data: Any, level: int = 0) -> str:
        lines: List[str] = []
        self._emit(data, level, lines)
        return "\n".join(lines) + ("\n" if lines else "")

    def _emit(self, data: Any, level: int, lines: List[str]) -> None:
        if isinstance(data, dict):
            for key, value in data.items():
                prefix = " " * (level * self.indent) + f"{key}:"
                if isinstance(value, (dict, list)):
                    lines.append(prefix)
                    self._emit(value, level + 1, lines)
                else:
                    rendered = _render_scalar(value)
                    lines.append(prefix + f" {rendered}")
        elif isinstance(data, list):
            for item in data:
                prefix = " " * (level * self.indent) + "-"
                if isinstance(item, (dict, list)):
                    lines.append(prefix)
                    self._emit(item, level + 1, lines)
                else:
                    rendered = _render_scalar(item)
                    lines.append(prefix + f" {rendered}")
        else:
            lines.append(" " * (level * self.indent) + _render_scalar(data))


def _parse_scalar(value: str) -> Any:
    lower = value.lower()
    if lower in _BOOL_MAP:
        return _BOOL_MAP[lower]
    if lower in _NULL_VALUES:
        return None
    if value.startswith("'") and value.endswith("'"):
        return value[1:-1]
    if value.startswith('"') and value.endswith('"'):
        return value[1:-1]
    if value.startswith("[") or value.startswith("{"):
        try:
            json_ready = _prepare_inline_json(value)
            return ast.literal_eval(json_ready)
        except (SyntaxError, ValueError) as exc:
            raise YAMLError(f"invalid inline collection: {value}") from exc
    if re.fullmatch(r"[-+]?[0-9]+", value):
        try:
            return int(value)
        except ValueError:
            pass
    if re.fullmatch(r"[-+]?[0-9]*\.[0-9]+", value):
        try:
            return float(value)
        except ValueError:
            pass
    return value


def _prepare_inline_json(value: str) -> str:
    replaced = re.sub(r"\btrue\b", "True", value)
    replaced = re.sub(r"\bfalse\b", "False", replaced)
    replaced = re.sub(r"\bnull\b", "None", replaced)
    replaced = replaced.replace(": ", ": ")
    return replaced


def _render_scalar(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return json.dumps(value)
    if isinstance(value, str):
        if re.search(r"[:#\n]", value):
            return json.dumps(value)
        return value
    return json.dumps(value)
