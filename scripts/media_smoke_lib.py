#!/usr/bin/env python3
"""Pure helpers for media dogfood smoke (publish/play URL checks).

No network I/O — suitable for unit tests and shell-driven smoke scripts.
"""
from __future__ import annotations

from typing import Any, Mapping
from urllib.parse import urljoin, urlparse


class MediaSmokeError(ValueError):
    """Raised when a media response fails consistency checks."""


def obs_server_from_push_url(push_url: str, stream_key: str) -> str:
    """Strip trailing /{stream_key} from a full RTMP push URL for OBS Server.

    Mirrors apps/mobile OBS dialog logic:
    push_url is rtmp://host/app/stream — OBS wants Server=rtmp://host/app.
    """
    push = (push_url or "").strip()
    if not push:
        return ""
    key = (stream_key or "").strip()
    if key:
        suffix = f"/{key}"
        if push.endswith(suffix):
            return push[: -len(suffix)]
    # Fallback: drop last path segment after scheme://
    scheme_sep = "://"
    scheme_i = push.find(scheme_sep)
    min_i = scheme_i + len(scheme_sep) if scheme_i >= 0 else 0
    i = push.rfind("/")
    if i > min_i:
        return push[:i]
    return push


def parse_publish_response(data: Mapping[str, Any]) -> dict[str, str]:
    """Parse POST .../media/publish JSON into OBS-ready fields.

    Returns:
        dict with keys: server, stream_key, push_url
    """
    if not isinstance(data, Mapping):
        raise MediaSmokeError(f"publish response must be an object, got {type(data).__name__}")

    push_url = data.get("push_url")
    stream_key = data.get("stream_key")
    if not isinstance(push_url, str) or not push_url.strip():
        raise MediaSmokeError("publish response missing non-empty push_url")
    if not isinstance(stream_key, str) or not stream_key.strip():
        raise MediaSmokeError("publish response missing non-empty stream_key")

    push_url = push_url.strip()
    stream_key = stream_key.strip()
    server = obs_server_from_push_url(push_url, stream_key)
    if not server:
        raise MediaSmokeError(f"could not derive OBS server from push_url={push_url!r}")

    return {
        "server": server,
        "stream_key": stream_key,
        "push_url": push_url,
    }


def parse_play_response(data: Mapping[str, Any], room_id: str) -> str:
    """Parse GET .../media/play JSON and return the HLS URL.

    Validates that the HLS path references the given room_id.
    """
    if not isinstance(data, Mapping):
        raise MediaSmokeError(f"play response must be an object, got {type(data).__name__}")
    if not isinstance(room_id, str) or not room_id.strip():
        raise MediaSmokeError("room_id must be a non-empty string")

    room_id = room_id.strip()
    hls = data.get("hls")
    if not isinstance(hls, str) or not hls.strip():
        raise MediaSmokeError("play response missing non-empty hls")
    hls = hls.strip()

    # Expected pattern: {base}/{room_id}.m3u8
    path = urlparse(hls).path or hls
    if room_id not in path and room_id not in hls:
        raise MediaSmokeError(
            f"hls URL does not reference room_id={room_id!r}: {hls!r}"
        )
    if not (hls.endswith(".m3u8") or path.endswith(".m3u8")):
        raise MediaSmokeError(f"hls URL should end with .m3u8: {hls!r}")

    return hls


def assert_stream_key_matches_room(stream_key: str, room_id: str) -> None:
    """Stream key must authorize the room (not a bare UUID alone).

    Format: `{room_id}?exp={unix}&sig={hex}` so RTMP stream name is the room
    UUID (stable HLS path) while HMAC lives in the query string.
    """
    if not isinstance(stream_key, str) or not stream_key.strip():
        raise MediaSmokeError("stream_key must be a non-empty string")
    if not isinstance(room_id, str) or not room_id.strip():
        raise MediaSmokeError("room_id must be a non-empty string")
    sk = stream_key.strip()
    rid = room_id.strip()
    if sk == rid:
        raise MediaSmokeError(
            f"stream_key must include exp+sig query, not bare room_id {rid!r}"
        )
    # Preferred: uuid?exp=&sig=
    if sk.startswith(f"{rid}?") and "exp=" in sk and "sig=" in sk:
        return
    # Legacy underscore form room_exp_sig
    if sk.startswith(f"{rid}_") and sk.count("_") >= 2:
        rest = sk[len(rid) + 1 :]
        if rest.split("_", 1)[0].isdigit():
            return
    raise MediaSmokeError(
        f"stream_key {sk!r} is not a signed token for room_id {rid!r}"
    )



def format_obs_paste_block(
    server: str,
    stream_key: str,
    *,
    push_url: str = "",
    hls: str = "",
    flv: str = "",
    expires_at: str = "",
) -> str:
    """Human-facing paste-ready OBS + HLS block (no network I/O).

    Used by dogfood scripts; Server must already be derived via
    ``obs_server_from_push_url`` (never hardcode localhost in callers when
    push_url is available).
    """
    lines = [
        "---- OBS (custom RTMP) paste-ready ----",
        f"Server:      {server}",
        f"Stream Key:  {stream_key}",
    ]
    if push_url:
        lines.append(f"push_url:    {push_url}")
    if expires_at:
        lines.append(f"expires_at:  {expires_at}")
    if hls:
        lines.append(f"HLS:         {hls}")
    if flv:
        lines.append(f"flv:         {flv}")
    lines.append("---------------------------------------")
    sk = (stream_key or "").strip()
    if "?" not in sk or "exp=" not in sk or "sig=" not in sk:
        lines.append(
            "WARN: stream_key should include ?exp=&sig= (signed token; bare UUID rejected)."
        )
    else:
        lines.append("NOTE: paste full stream_key including ?exp=&sig= (not bare room UUID).")
    return "\n".join(lines)


def srs_http_ok_url(base: str) -> str:
    """Build SRS HTTP API versions endpoint used as a liveness probe.

    SRS listens on :1985 by default; GET /api/v1/versions returns JSON.
    """
    base = (base or "").strip().rstrip("/")
    if not base:
        raise MediaSmokeError("SRS base URL must be non-empty")
    # Accept bare host:port or full URL.
    if "://" not in base:
        base = f"http://{base}"
    return urljoin(base + "/", "api/v1/versions")


def srs_api_base_from_rtmp(rtmp_server: str, api_port: int = 1985) -> str:
    """Derive default SRS HTTP API base from an RTMP server URL host.

    Example: rtmp://localhost:1935/live → http://localhost:1985
    """
    server = (rtmp_server or "").strip()
    if not server:
        raise MediaSmokeError("rtmp_server must be non-empty")
    if "://" not in server:
        server = f"rtmp://{server}"
    parsed = urlparse(server)
    host = parsed.hostname
    if not host:
        raise MediaSmokeError(f"could not parse host from rtmp_server={rtmp_server!r}")
    return f"http://{host}:{api_port}"
