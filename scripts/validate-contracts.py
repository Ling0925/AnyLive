#!/usr/bin/env python3
"""Validate OpenAPI YAML structure and error codes file for P0 contracts."""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    try:
        import yaml  # type: ignore
    except ImportError:
        # Fallback: minimal checks without PyYAML
        return fallback_checks()

    codes_path = ROOT / "contracts/errors/codes.yaml"
    openapi_path = ROOT / "contracts/openapi/openapi.yaml"

    codes = yaml.safe_load(codes_path.read_text())
    assert "codes" in codes and isinstance(codes["codes"], dict), "codes.yaml missing codes map"
    required = {
        "INTERNAL",
        "AUTH_INVALID_OTP",
        "GIFT_INSUFFICIENT_BALANCE",
        "ROOM_NOT_LIVE",
    }
    missing = required - set(codes["codes"])
    assert not missing, f"missing error codes: {missing}"

    doc = yaml.safe_load(openapi_path.read_text())
    assert doc["openapi"].startswith("3."), "openapi version"
    paths = doc["paths"]
    for p in ("/health", "/ready", "/api/v1/meta", "/api/v1/rooms", "/api/v1/me"):
        assert p in paths, f"missing path {p}"
    schemas = doc["components"]["schemas"]
    for s in ("Room", "User", "TokenPair", "ApiError"):
        assert s in schemas, f"missing schema {s}"

    print("OK: contracts validated with PyYAML")
    return 0


def fallback_checks() -> int:
    codes = (ROOT / "contracts/errors/codes.yaml").read_text()
    openapi = (ROOT / "contracts/openapi/openapi.yaml").read_text()
    for token in (
        "GIFT_INSUFFICIENT_BALANCE",
        "AUTH_INVALID_OTP",
        "/health",
        "/api/v1/rooms",
        "TokenPair",
    ):
        blob = codes if token.startswith("GIFT") or token.startswith("AUTH") else openapi
        # check both
        if token not in codes and token not in openapi:
            print(f"FAIL: missing {token}", file=sys.stderr)
            return 1
    print("OK: contracts validated (fallback text checks)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
