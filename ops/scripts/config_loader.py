"""Utility helpers to load watcher/hook configuration files.

The dry-run tooling must support both JSON and YAML inputs without relying on
external dependencies. We first attempt to load `yaml.safe_load` if available;
otherwise we fall back to a tiny YAML parser that understands the subset of
YAML used in the repository (nested mappings, sequences and scalars).
"""

from __future__ import annotations

import importlib.util
import json
from dataclasses import dataclass
from typing import Any, Iterable, List, Tuple

__all__ = ["load_config"]


def _load_yaml_module():
    spec = importlib.util.find_spec("yaml")
    if spec is None:
        return None
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)  # type: ignore[assignment]
    return getattr(module, "safe_load", None)


_YAML_SAFE_LOAD = _load_yaml_module()


def load_config(text: str) -> Any:
    """Parse a configuration file expressed in JSON or YAML."""

    if not text.strip():
        return None

    if _YAML_SAFE_LOAD is not None:
        return _YAML_SAFE_LOAD(text)

    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return _SimpleYAMLParser(text).parse()


@dataclass
class _Token:
    indent: int
    value: str


class _SimpleYAMLParser:
    """Very small YAML parser for mappings and sequences.

    It intentionally supports only a constrained subset of YAML sufficient for
    the dry-run configuration files: nested dictionaries, lists and scalar
    values (strings, numbers, booleans and nulls). It ignores blank lines and
    comments.
    """

    def __init__(self, text: str) -> None:
        self.tokens: List[_Token] = list(self._tokenize(text))
        self.index = 0

    @staticmethod
    def _strip_comment(line: str) -> str:
        result: List[str] = []
        in_single = False
        in_double = False
        for char in line:
            if char == "'" and not in_double:
                in_single = not in_single
            elif char == "\"" and not in_single:
                in_double = not in_double
            elif char == "#" and not in_single and not in_double:
                break
            result.append(char)
        return "".join(result)

    @staticmethod
    def _tokenize(text: str) -> Iterable[_Token]:
        for raw_line in text.splitlines():
            stripped_line = raw_line.rstrip("\n")
            cleaned = _SimpleYAMLParser._strip_comment(stripped_line)
            if not cleaned.strip():
                continue
            if "\t" in cleaned:
                raise ValueError("YAML parser does not support tabs for indentation")
            indent = len(cleaned) - len(cleaned.lstrip(" "))
            yield _Token(indent=indent, value=cleaned.strip())

    def parse(self) -> Any:
        if not self.tokens:
            return None
        value, index = self._parse_block(0, self.tokens[0].indent)
        if index != len(self.tokens):
            raise ValueError("Unexpected trailing content in YAML document")
        return value

    def _parse_block(self, index: int, indent: int) -> Tuple[Any, int]:
        token = self.tokens[index]
        if token.indent > indent:
            raise ValueError("Invalid indentation in YAML document")
        if token.value.startswith("- "):
            return self._parse_sequence(index, indent)
        return self._parse_mapping(index, indent)

    def _parse_sequence(self, index: int, indent: int) -> Tuple[List[Any], int]:
        items: List[Any] = []
        while index < len(self.tokens):
            token = self.tokens[index]
            if token.indent < indent:
                break
            if token.indent > indent or not token.value.startswith("- "):
                break
            content = token.value[2:].strip()
            index += 1
            if not content:
                if index < len(self.tokens) and self.tokens[index].indent > indent:
                    value, index = self._parse_block(index, self.tokens[index].indent)
                else:
                    value = None
                items.append(value)
                continue
            if content.endswith(":") or ": " in content:
                key, _, remainder = content.partition(":")
                key = key.strip()
                remainder = remainder.strip()
                item: dict[str, Any]
                if remainder:
                    item = {key: self._parse_scalar(remainder)}
                    if index < len(self.tokens) and self.tokens[index].indent > indent:
                        nested, index = self._parse_block(
                            index, self.tokens[index].indent
                        )
                        if not isinstance(nested, dict):
                            raise ValueError(
                                f"Expected mapping after list item '{key}'"
                            )
                        item.update(nested)
                else:
                    if index < len(self.tokens) and self.tokens[index].indent > indent:
                        nested, index = self._parse_block(
                            index, self.tokens[index].indent
                        )
                    else:
                        nested = None
                    item = {key: nested}
                items.append(item)
                continue
            value = self._parse_scalar(content)
            items.append(value)
        return items, index

    def _parse_mapping(self, index: int, indent: int) -> Tuple[dict[str, Any], int]:
        mapping: dict[str, Any] = {}
        while index < len(self.tokens):
            token = self.tokens[index]
            if token.indent < indent:
                break
            if token.indent > indent:
                raise ValueError("Invalid indentation in mapping")
            if token.value.startswith("- "):
                raise ValueError("Mixed sequence and mapping without parent key")
            if ":" not in token.value:
                raise ValueError(f"Invalid mapping entry: {token.value}")
            key, _, remainder = token.value.partition(":")
            key = key.strip()
            remainder = remainder.strip()
            index += 1
            if remainder:
                value = self._parse_scalar(remainder)
                if index < len(self.tokens) and self.tokens[index].indent > indent:
                    nested, index = self._parse_block(index, self.tokens[index].indent)
                    if not isinstance(nested, dict):
                        raise ValueError(
                            f"Expected nested mapping after key '{key}'"
                        )
                    merged = dict(value if isinstance(value, dict) else {})
                    if merged:
                        raise ValueError(
                            f"Conflicting inline and nested values for key '{key}'"
                        )
                    mapping[key] = nested
                else:
                    mapping[key] = value
            else:
                if index < len(self.tokens) and self.tokens[index].indent > indent:
                    value, index = self._parse_block(index, self.tokens[index].indent)
                else:
                    value = None
                mapping[key] = value
        return mapping, index

    @staticmethod
    def _parse_scalar(token: str) -> Any:
        if not token:
            return ""
        if token[0] == token[-1] and token[0] in {'"', "'"} and len(token) >= 2:
            return _SimpleYAMLParser._unescape_string(token[1:-1], quote=token[0])
        lowered = token.lower()
        if lowered in {"true", "yes", "on"}:
            return True
        if lowered in {"false", "no", "off"}:
            return False
        if lowered in {"null", "none", "~"}:
            return None
        try:
            if "_" in token:
                raise ValueError
            return int(token)
        except ValueError:
            try:
                return float(token)
            except ValueError:
                pass
        if token.startswith("[") or token.startswith("{"):
            try:
                return json.loads(token)
            except json.JSONDecodeError:
                pass
        return token

    @staticmethod
    def _unescape_string(value: str, *, quote: str) -> str:
        escape = "\\" + quote
        return value.replace(escape, quote).replace("\\\\", "\\")
