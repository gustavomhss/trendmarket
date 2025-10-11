"""Minimal jsonschema implementation for offline environments."""

from __future__ import annotations

import datetime as _dt
import re
from typing import Any, Dict, Iterable, Iterator, List, Mapping, Sequence, Tuple, Union

__all__ = ["Draft7Validator", "ValidationError", "SchemaError"]


class SchemaError(Exception):
    """Raised when the provided schema is structurally invalid."""

    def __init__(self, message: str):
        super().__init__(message)
        self.message = message


class ValidationError(Exception):
    """Represents a validation failure."""

    def __init__(self, message: str, path: Sequence[Union[str, int]] | None = None,
                 schema_path: Sequence[Union[str, int]] | None = None):
        super().__init__(message)
        self.message = message
        self.path: Tuple[Union[str, int], ...] = tuple(path or ())
        self.schema_path: Tuple[Union[str, int], ...] = tuple(schema_path or ())


_JSON_TYPES = {
    "object": dict,
    "array": list,
    "string": str,
    "number": (int, float),
    "integer": int,
    "boolean": bool,
    "null": type(None),
}


class Draft7Validator:
    """Lightweight Draft-07 JSON schema validator.

    This implementation is intentionally limited but supports the constructs
    required by the observability gatecheck workflows: ``type`` checks,
    ``properties`` and ``required`` constraints, ``additionalProperties``
    (``true``/``false`` or a nested schema), arrays with ``items`` schemas,
    ``minItems``, numeric and string bounds, ``const`` and ``enum`` checks,
    object property counts, regex ``pattern`` validation, and the ``date-time``
    format.
    """

    def __init__(self, schema: Mapping[str, Any]):
        if not isinstance(schema, Mapping):
            raise SchemaError("Schema must be an object")
        self.schema = schema
        self._validate_schema(schema)

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------
    def iter_errors(self, instance: Any) -> Iterator[ValidationError]:
        yield from self._iter_errors(instance, self.schema, (), ())

    def validate(self, instance: Any) -> None:
        for error in self.iter_errors(instance):
            raise error

    # ------------------------------------------------------------------
    # Schema validation helpers
    # ------------------------------------------------------------------
    def _validate_schema(self, schema: Mapping[str, Any], path: Tuple[Union[str, int], ...] = ()) -> None:
        for key, value in schema.items():
            if key == "type":
                self._validate_type_keyword(value, path + ("type",))
            elif key == "properties":
                if not isinstance(value, Mapping):
                    raise SchemaError(f"properties must be an object at {'/'.join(map(str, path)) or '(root)'}")
                for prop, prop_schema in value.items():
                    if not isinstance(prop_schema, Mapping):
                        raise SchemaError(f"schema for property '{prop}' must be an object")
                    self._validate_schema(prop_schema, path + ("properties", prop))
            elif key == "required":
                if not isinstance(value, Sequence) or isinstance(value, (str, bytes)):
                    raise SchemaError("required must be an array of strings")
            elif key == "additionalProperties":
                if not isinstance(value, (bool, Mapping)):
                    raise SchemaError("additionalProperties must be boolean or schema")
                if isinstance(value, Mapping):
                    self._validate_schema(value, path + ("additionalProperties",))
            elif key == "items":
                if isinstance(value, Mapping):
                    self._validate_schema(value, path + ("items",))
                elif isinstance(value, Sequence):
                    for idx, item_schema in enumerate(value):
                        if not isinstance(item_schema, Mapping):
                            raise SchemaError("items array must contain schema objects")
                        self._validate_schema(item_schema, path + ("items", idx))
                else:
                    raise SchemaError("items must be a schema or array of schemas")
            elif key in {"minItems", "maxItems", "minLength", "maxLength", "minProperties", "maxProperties"}:
                if not isinstance(value, int) or value < 0:
                    raise SchemaError(f"{key} must be a non-negative integer")
            elif key in {"minimum", "maximum"}:
                if not isinstance(value, (int, float)):
                    raise SchemaError(f"{key} must be numeric")
            elif key == "pattern":
                if not isinstance(value, str):
                    raise SchemaError("pattern must be a string")
                try:
                    re.compile(value)
                except re.error as exc:  # pragma: no cover - defensive
                    raise SchemaError(f"invalid regex pattern at {'/'.join(map(str, path))}: {exc}") from exc
            elif key == "enum":
                if not isinstance(value, Sequence) or isinstance(value, (str, bytes)):
                    raise SchemaError("enum must be an array")
            elif key == "const":
                # any value allowed
                pass
            elif key == "format":
                if not isinstance(value, str):
                    raise SchemaError("format must be a string")
            elif isinstance(value, Mapping):
                self._validate_schema(value, path + (key,))

    def _validate_type_keyword(self, type_value: Any, path: Tuple[Union[str, int], ...]) -> None:
        if isinstance(type_value, str):
            if type_value not in _JSON_TYPES:
                raise SchemaError(f"unsupported type '{type_value}' at {'/'.join(map(str, path))}")
        elif isinstance(type_value, Sequence):
            if not type_value or any(not isinstance(item, str) for item in type_value):
                raise SchemaError("type array must contain type names")
            for item in type_value:
                if item not in _JSON_TYPES:
                    raise SchemaError(f"unsupported type '{item}' at {'/'.join(map(str, path))}")
        else:
            raise SchemaError("type must be string or array of strings")

    # ------------------------------------------------------------------
    # Validation helpers
    # ------------------------------------------------------------------
    def _iter_errors(
        self,
        instance: Any,
        schema: Mapping[str, Any],
        instance_path: Tuple[Union[str, int], ...],
        schema_path: Tuple[Union[str, int], ...],
    ) -> Iterator[ValidationError]:
        type_keyword = schema.get("type")
        if type_keyword is not None and not self._matches_type(instance, type_keyword):
            expected = (
                ", ".join(type_keyword)
                if isinstance(type_keyword, Sequence) and not isinstance(type_keyword, (str, bytes))
                else type_keyword
            )
            yield ValidationError(
                f"{self._instance_repr(instance)} is not of type {expected}",
                instance_path,
                schema_path + ("type",),
            )
            return

        if "const" in schema and instance != schema["const"]:
            yield ValidationError(
                f"value does not match const {schema['const']!r}",
                instance_path,
                schema_path + ("const",),
            )
            return

        if "enum" in schema:
            enum_values = schema["enum"]
            if instance not in enum_values:
                yield ValidationError(
                    "value is not one of the allowed options",
                    instance_path,
                    schema_path + ("enum",),
                )
                return

        if isinstance(instance, Mapping):
            yield from self._validate_object(instance, schema, instance_path, schema_path)
        elif isinstance(instance, list):
            yield from self._validate_array(instance, schema, instance_path, schema_path)
        elif isinstance(instance, str):
            yield from self._validate_string(instance, schema, instance_path, schema_path)
        elif isinstance(instance, (int, float)) and not isinstance(instance, bool):
            yield from self._validate_number(instance, schema, instance_path, schema_path)

    def _validate_object(
        self,
        instance: Mapping[str, Any],
        schema: Mapping[str, Any],
        instance_path: Tuple[Union[str, int], ...],
        schema_path: Tuple[Union[str, int], ...],
    ) -> Iterator[ValidationError]:
        required = schema.get("required", [])
        for name in required:
            if name not in instance:
                yield ValidationError(
                    f"'{name}' is a required property",
                    instance_path,
                    schema_path + ("required",),
                )
        if "minProperties" in schema and len(instance) < schema["minProperties"]:
            yield ValidationError(
                f"object has fewer than {schema['minProperties']} properties",
                instance_path,
                schema_path + ("minProperties",),
            )
        properties = schema.get("properties", {})
        for name, subschema in properties.items():
            if name in instance:
                yield from self._iter_errors(instance[name], subschema, instance_path + (name,), schema_path + ("properties", name))
        additional = schema.get("additionalProperties", True)
        if additional is False:
            for name in instance:
                if name not in properties:
                    yield ValidationError(
                        f"additional property '{name}' is not allowed",
                        instance_path + (name,),
                        schema_path + ("additionalProperties",),
                    )
        elif isinstance(additional, Mapping):
            for name, value in instance.items():
                if name not in properties:
                    yield from self._iter_errors(value, additional, instance_path + (name,), schema_path + ("additionalProperties",))

    def _validate_array(
        self,
        instance: Sequence[Any],
        schema: Mapping[str, Any],
        instance_path: Tuple[Union[str, int], ...],
        schema_path: Tuple[Union[str, int], ...],
    ) -> Iterator[ValidationError]:
        if "minItems" in schema and len(instance) < schema["minItems"]:
            yield ValidationError(
                f"array has fewer than {schema['minItems']} items",
                instance_path,
                schema_path + ("minItems",),
            )
        if "maxItems" in schema and len(instance) > schema["maxItems"]:
            yield ValidationError(
                f"array has more than {schema['maxItems']} items",
                instance_path,
                schema_path + ("maxItems",),
            )
        items_schema = schema.get("items")
        if isinstance(items_schema, Mapping):
            for idx, value in enumerate(instance):
                yield from self._iter_errors(value, items_schema, instance_path + (idx,), schema_path + ("items",))
        elif isinstance(items_schema, Sequence) and not isinstance(items_schema, (str, bytes)):
            for idx, subschema in enumerate(items_schema):
                if idx < len(instance):
                    yield from self._iter_errors(instance[idx], subschema, instance_path + (idx,), schema_path + ("items", idx))

    def _validate_string(
        self,
        instance: str,
        schema: Mapping[str, Any],
        instance_path: Tuple[Union[str, int], ...],
        schema_path: Tuple[Union[str, int], ...],
    ) -> Iterator[ValidationError]:
        if "minLength" in schema and len(instance) < schema["minLength"]:
            yield ValidationError(
                f"string is shorter than {schema['minLength']} characters",
                instance_path,
                schema_path + ("minLength",),
            )
        if "maxLength" in schema and len(instance) > schema["maxLength"]:
            yield ValidationError(
                f"string is longer than {schema['maxLength']} characters",
                instance_path,
                schema_path + ("maxLength",),
            )
        if "pattern" in schema:
            pattern = schema["pattern"]
            if not re.search(pattern, instance):
                yield ValidationError(
                    f"string does not match pattern {pattern!r}",
                    instance_path,
                    schema_path + ("pattern",),
                )
        if schema.get("format") == "date-time":
            if not self._is_valid_datetime(instance):
                yield ValidationError(
                    "value is not a valid date-time",
                    instance_path,
                    schema_path + ("format",),
                )

    def _validate_number(
        self,
        instance: Union[int, float],
        schema: Mapping[str, Any],
        instance_path: Tuple[Union[str, int], ...],
        schema_path: Tuple[Union[str, int], ...],
    ) -> Iterator[ValidationError]:
        if "minimum" in schema and instance < schema["minimum"]:
            yield ValidationError(
                f"number is less than minimum of {schema['minimum']}",
                instance_path,
                schema_path + ("minimum",),
            )
        if "maximum" in schema and instance > schema["maximum"]:
            yield ValidationError(
                f"number exceeds maximum of {schema['maximum']}",
                instance_path,
                schema_path + ("maximum",),
            )

    # ------------------------------------------------------------------
    # Utility helpers
    # ------------------------------------------------------------------
    @staticmethod
    def _matches_type(instance: Any, expected: Union[str, Sequence[str]]) -> bool:
        if isinstance(expected, str):
            return Draft7Validator._is_type(instance, expected)
        return any(Draft7Validator._is_type(instance, option) for option in expected)

    @staticmethod
    def _is_type(instance: Any, type_name: str) -> bool:
        python_type = _JSON_TYPES[type_name]
        if type_name == "integer":
            return isinstance(instance, int) and not isinstance(instance, bool)
        if type_name == "number":
            return (isinstance(instance, (int, float)) and not isinstance(instance, bool))
        return isinstance(instance, python_type)

    @staticmethod
    def _instance_repr(instance: Any) -> str:
        if isinstance(instance, str):
            return f"'{instance}'"
        return repr(instance)

    @staticmethod
    def _is_valid_datetime(value: str) -> bool:
        try:
            if value.endswith("Z"):
                value = value[:-1] + "+00:00"
            _dt.datetime.fromisoformat(value)
            return True
        except ValueError:
            return False


ValidationError.__name__ = "ValidationError"
SchemaError.__name__ = "SchemaError"
