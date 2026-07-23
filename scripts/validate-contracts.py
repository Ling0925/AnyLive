#!/usr/bin/env python3
"""Validate OpenAPI YAML structure and error codes file for P0 contracts."""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# WBS E1.4 — media/pay webhook + NATS event JSON Schema artifacts
EVENT_CONTRACTS = (
    "contracts/events/gift.sent.v1.json",
)
WEBHOOK_CONTRACTS = (
    "contracts/webhooks/pay.mock.v1.json",
    "contracts/webhooks/pay.hmac.v1.json",
    "contracts/webhooks/srs.on_publish.v1.json",
)


def assert_json_schema_files() -> None:
    """Ensure event/webhook contract files exist and parse as JSON objects."""
    for rel in EVENT_CONTRACTS + WEBHOOK_CONTRACTS:
        path = ROOT / rel
        assert path.is_file(), f"missing contract {rel}"
        data = json.loads(path.read_text(encoding="utf-8"))
        assert isinstance(data, dict), f"{rel} must be a JSON object"
        assert "type" in data or "$schema" in data, f"{rel} missing schema shape"
        assert "properties" in data or "required" in data or "$id" in data, (
            f"{rel} looks empty"
        )
    readme = ROOT / "contracts/webhooks/README.md"
    assert readme.is_file(), "missing contracts/webhooks/README.md"


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
    required_paths = (
        "/health",
        "/ready",
        "/metrics",
        "/api/v1/meta",
        "/api/v1/rooms",
        "/api/v1/me",
        "/api/v1/me/export",
        "/api/v1/legal/privacy",
        "/api/v1/legal/terms",
        "/api/v1/wallet",
        "/api/v1/wallet/ledger",
        "/api/v1/gifts",
        "/api/v1/feed/hot",
        "/api/v1/reports",
        "/api/v1/admin/mute",
        "/api/v1/admin/unmute",
        "/api/v1/admin/reports/{reportId}",
        "/api/v1/admin/wallet/reconcile",
        "/api/v1/admin/pay/expire-orders",
        "/api/v1/webhooks/srs/on_publish",
        "/api/v1/pay/channels",
        "/api/v1/pay/products",
        "/api/v1/pay/orders",
        "/api/v1/pay/orders/{orderId}",
        "/api/v1/pay/orders/{orderId}/sandbox-complete",
        "/api/v1/webhooks/pay/mock",
        "/api/v1/webhooks/pay/stripe",
        "/api/v1/webhooks/pay/iap",
        "/api/v1/rooms/{roomId}/livekit/join",
        "/api/v1/rooms/{roomId}/interactive/invite",
        "/api/v1/rooms/{roomId}/interactive/respond",
        "/api/v1/rooms/{roomId}/interactive/leave",
        "/api/v1/rooms/{roomId}/interactive",
        "/api/v1/rooms/{roomId}/pk",
        "/api/v1/rooms/{roomId}/pk/start",
        "/api/v1/rooms/{roomId}/pk/end",
        "/api/v1/events",
        "/api/v1/me/creator",
        "/api/v1/me/sessions",
        "/api/v1/me/sessions/{jti}",
        "/api/v1/me/push-tokens",
        "/api/v1/search",
        "/api/v1/rooms/{roomId}/recording",
        "/api/v1/me/avatar/presign",
    )
    for p in required_paths:
        assert p in paths, f"missing path {p}"

    # Method coverage for multi-method / newly added routes
    me_ops = paths["/api/v1/me"]
    for method in ("get", "patch", "delete"):
        assert method in me_ops, f"/api/v1/me missing {method}"
    assert "get" in paths["/api/v1/me/export"]
    assert "get" in paths["/api/v1/legal/privacy"]
    assert "get" in paths["/api/v1/legal/terms"]
    assert "get" in paths["/api/v1/wallet/ledger"]
    assert "post" in paths["/api/v1/admin/mute"]
    assert "post" in paths["/api/v1/admin/unmute"]
    assert "patch" in paths["/api/v1/admin/reports/{reportId}"]
    assert "get" in paths["/api/v1/admin/wallet/reconcile"]
    assert "post" in paths["/api/v1/admin/pay/expire-orders"]
    assert "get" in paths["/api/v1/pay/channels"]
    assert "get" in paths["/api/v1/pay/products"]
    assert "post" in paths["/api/v1/pay/orders"]
    assert "get" in paths["/api/v1/pay/orders/{orderId}"]
    assert "post" in paths["/api/v1/pay/orders/{orderId}/sandbox-complete"]
    assert "post" in paths["/api/v1/webhooks/pay/mock"]
    assert "post" in paths["/api/v1/webhooks/pay/stripe"]
    assert "post" in paths["/api/v1/webhooks/pay/iap"]
    assert "post" in paths["/api/v1/rooms/{roomId}/livekit/join"]
    assert "post" in paths["/api/v1/rooms/{roomId}/interactive/invite"]
    assert "post" in paths["/api/v1/rooms/{roomId}/interactive/respond"]
    assert "post" in paths["/api/v1/rooms/{roomId}/interactive/leave"]
    assert "get" in paths["/api/v1/rooms/{roomId}/interactive"]
    assert "get" in paths["/api/v1/rooms/{roomId}/pk"]
    assert "post" in paths["/api/v1/rooms/{roomId}/pk/start"]
    assert "post" in paths["/api/v1/rooms/{roomId}/pk/end"]
    assert "post" in paths["/api/v1/events"]
    assert "get" in paths["/api/v1/me/creator"]
    sessions = paths["/api/v1/me/sessions"]
    assert "get" in sessions and "delete" in sessions
    assert "delete" in paths["/api/v1/me/sessions/{jti}"]
    push = paths["/api/v1/me/push-tokens"]
    for method in ("get", "post", "delete"):
        assert method in push, f"push-tokens missing {method}"
    assert "get" in paths["/api/v1/search"]
    assert "get" in paths["/api/v1/rooms/{roomId}/recording"]
    assert "put" in paths["/api/v1/rooms/{roomId}/recording"]
    assert "post" in paths["/api/v1/me/avatar/presign"]

    schemas = doc["components"]["schemas"]
    for s in (
        "Room",
        "User",
        "TokenPair",
        "ApiError",
        "AccountExport",
        "LegalDoc",
        "LedgerList",
        "LedgerEntry",
        "PatchMeRequest",
        "PushDevice",
        "PushRegisterRequest",
        "Session",
        "SessionListResponse",
        "MuteUserRequest",
        "PayProduct",
        "PayProductListResponse",
        "PayOrder",
        "CreatePayOrderRequest",
        "PayChannelListResponse",
        "UnmuteUserRequest",
        "ResolveReportRequest",
        "AdminReport",
        "WalletReconcileResponse",
        "BalanceMismatch",
        "ExpirePayOrdersResponse",
        "LiveKitJoinRequest",
        "LiveKitJoinResponse",
        "InteractiveInviteRequest",
        "InteractiveRespondRequest",
        "InteractiveSession",
        "InteractiveSessionList",
        "StartPkRequest",
        "PkSession",
        "PkSessionResponse",
        "ClientEventBatch",
        "ClientEvent",
        "ClientEventIngestResponse",
        "CreatorStatsResponse",
    ):
        assert s in schemas, f"missing schema {s}"

    assert_json_schema_files()

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
        "/api/v1/me/export",
        "/api/v1/legal/privacy",
        "/api/v1/legal/terms",
        "/api/v1/wallet/ledger",
        "/api/v1/admin/mute",
        "/api/v1/admin/unmute",
        "/api/v1/admin/reports/{reportId}",
        "/api/v1/admin/wallet/reconcile",
        "TokenPair",
        "AccountExport",
        "LegalDoc",
        "LedgerList",
        "WalletReconcileResponse",
    ):
        # check both
        if token not in codes and token not in openapi:
            print(f"FAIL: missing {token}", file=sys.stderr)
            return 1
    try:
        assert_json_schema_files()
    except Exception as e:  # noqa: BLE001 — surface as FAIL for CI
        print(f"FAIL: event/webhook contracts: {e}", file=sys.stderr)
        return 1
    print("OK: contracts validated (fallback text checks)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
