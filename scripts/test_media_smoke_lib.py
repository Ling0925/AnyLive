#!/usr/bin/env python3
"""Unit tests for scripts/media_smoke_lib pure helpers."""
from __future__ import annotations

import sys
import unittest
from pathlib import Path

# Allow `python3 -m unittest scripts/test_media_smoke_lib.py` from repo root.
sys.path.insert(0, str(Path(__file__).resolve().parent))

from media_smoke_lib import (  # noqa: E402
    MediaSmokeError,
    assert_stream_key_matches_room,
    obs_server_from_push_url,
    parse_play_response,
    parse_publish_response,
    srs_api_base_from_rtmp,
    srs_http_ok_url,
)

ROOM = "11111111-1111-1111-1111-111111111111"


class TestObsServerFromPushUrl(unittest.TestCase):
    def test_strips_stream_key_suffix(self) -> None:
        push = f"rtmp://localhost:1935/live/{ROOM}"
        self.assertEqual(
            obs_server_from_push_url(push, ROOM),
            "rtmp://localhost:1935/live",
        )

    def test_fallback_without_matching_key(self) -> None:
        self.assertEqual(
            obs_server_from_push_url("rtmp://host/live/abc", "other"),
            "rtmp://host/live",
        )

    def test_empty_push(self) -> None:
        self.assertEqual(obs_server_from_push_url("", ROOM), "")


class TestParsePublishResponse(unittest.TestCase):
    def test_happy_path(self) -> None:
        data = {
            "push_url": f"rtmp://localhost:1935/live/{ROOM}",
            "stream_key": ROOM,
            "expires_at": "2099-01-01T00:00:00Z",
        }
        out = parse_publish_response(data)
        self.assertEqual(out["stream_key"], ROOM)
        self.assertEqual(out["push_url"], data["push_url"])
        self.assertEqual(out["server"], "rtmp://localhost:1935/live")

    def test_missing_push_url(self) -> None:
        with self.assertRaises(MediaSmokeError):
            parse_publish_response({"stream_key": ROOM})

    def test_missing_stream_key(self) -> None:
        with self.assertRaises(MediaSmokeError):
            parse_publish_response({"push_url": "rtmp://x/live/k"})

    def test_empty_fields(self) -> None:
        with self.assertRaises(MediaSmokeError):
            parse_publish_response({"push_url": "  ", "stream_key": ROOM})
        with self.assertRaises(MediaSmokeError):
            parse_publish_response({"push_url": "rtmp://x/live/k", "stream_key": ""})


class TestParsePlayResponse(unittest.TestCase):
    def test_happy_path(self) -> None:
        hls = f"http://localhost:8080/live/{ROOM}.m3u8"
        self.assertEqual(parse_play_response({"hls": hls, "flv": "x"}, ROOM), hls)

    def test_hls_must_reference_room(self) -> None:
        with self.assertRaises(MediaSmokeError):
            parse_play_response(
                {"hls": "http://localhost:8080/live/other.m3u8"},
                ROOM,
            )

    def test_hls_must_be_m3u8(self) -> None:
        with self.assertRaises(MediaSmokeError):
            parse_play_response(
                {"hls": f"http://localhost:8080/live/{ROOM}"},
                ROOM,
            )

    def test_missing_hls(self) -> None:
        with self.assertRaises(MediaSmokeError):
            parse_play_response({}, ROOM)

    def test_empty_room_id(self) -> None:
        with self.assertRaises(MediaSmokeError):
            parse_play_response({"hls": f"http://x/{ROOM}.m3u8"}, "")


class TestAssertStreamKeyMatchesRoom(unittest.TestCase):
    def test_match(self) -> None:
        assert_stream_key_matches_room(ROOM, ROOM)

    def test_mismatch(self) -> None:
        with self.assertRaises(MediaSmokeError):
            assert_stream_key_matches_room("abc", ROOM)

    def test_strips_whitespace(self) -> None:
        assert_stream_key_matches_room(f"  {ROOM}  ", f" {ROOM}")


class TestSrsUrls(unittest.TestCase):
    def test_srs_http_ok_url_full(self) -> None:
        self.assertEqual(
            srs_http_ok_url("http://127.0.0.1:1985"),
            "http://127.0.0.1:1985/api/v1/versions",
        )

    def test_srs_http_ok_url_bare_host(self) -> None:
        self.assertEqual(
            srs_http_ok_url("127.0.0.1:1985"),
            "http://127.0.0.1:1985/api/v1/versions",
        )

    def test_srs_http_ok_url_trailing_slash(self) -> None:
        self.assertEqual(
            srs_http_ok_url("http://localhost:1985/"),
            "http://localhost:1985/api/v1/versions",
        )

    def test_srs_http_ok_url_empty(self) -> None:
        with self.assertRaises(MediaSmokeError):
            srs_http_ok_url("")

    def test_srs_api_base_from_rtmp(self) -> None:
        self.assertEqual(
            srs_api_base_from_rtmp("rtmp://localhost:1935/live"),
            "http://localhost:1985",
        )

    def test_srs_api_base_custom_port(self) -> None:
        self.assertEqual(
            srs_api_base_from_rtmp("rtmp://media.example:1935/live", api_port=1985),
            "http://media.example:1985",
        )


if __name__ == "__main__":
    unittest.main()
