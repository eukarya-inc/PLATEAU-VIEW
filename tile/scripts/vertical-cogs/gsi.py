"""Small HTTP client for GSI's public pages and the 基盤地図情報 download service.

Credentials (only needed for the JPGEO2024 ISG package and DEM downloads) are
read from a ``KEY=VALUE`` file (``GSI_USER`` / ``GSI_PASS``) whose path is
given by ``$GSI_LOGIN_CONF``. They are only ever held in memory and sent in the
login form POST; nothing here prints, logs or persists them.
"""

from __future__ import annotations

import http.cookiejar
import json
import os
import re
import ssl
import time
import urllib.error
import urllib.parse
import urllib.request

UA = (
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36"
)
KIBAN_API = "https://service.gsi.go.jp/kiban/app/api/"


class _HttpsRedirect(urllib.request.HTTPRedirectHandler):
    """GSI's OIDC flow hands back an http:// redirect_uri; follow it over https."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        if newurl.startswith("http://"):
            newurl = "https://" + newurl[len("http://") :]
        return super().redirect_request(req, fp, code, msg, headers, newurl)


class Client:
    def __init__(self) -> None:
        self._cj = http.cookiejar.CookieJar()
        # www.gsi.go.jp still needs TLS legacy renegotiation, which OpenSSL 3
        # refuses by default. Certificate verification stays on.
        ctx = ssl.create_default_context()
        ctx.options |= getattr(ssl, "OP_LEGACY_SERVER_CONNECT", 0x4)
        self._op = urllib.request.build_opener(
            urllib.request.HTTPCookieProcessor(self._cj),
            urllib.request.HTTPSHandler(context=ctx),
            _HttpsRedirect(),
        )
        self._op.addheaders = [("User-Agent", UA)]
        self.logged_in = False

    def get(self, url: str, timeout: int = 300) -> bytes:
        for attempt in range(3):
            try:
                with self._op.open(url, timeout=timeout) as r:
                    return r.read()
            except urllib.error.HTTPError as e:
                if e.code < 500 or attempt == 2:
                    raise
            except urllib.error.URLError:
                if attempt == 2:
                    raise
            time.sleep(3 * (attempt + 1))
        raise RuntimeError("unreachable")

    def get_json(self, url: str):
        return json.loads(self.get(url, timeout=60))

    # -- catalogue (no login) ------------------------------------------------
    def dem_editions(self, mesh6: str, types: str = "DEM1A,DEM5A,DEM5B,DEM5C") -> list[dict]:
        q = urllib.parse.urlencode(
            {"date_from": "2008-01", "date_to": "2030-12", "type_codes": types, "mesh_codes": mesh6}
        )
        return self.get_json(KIBAN_API + "dem/update?" + q).get("results") or []

    def geoid_files(self, version: str, type_code: str = "ISG") -> list[dict]:
        q = urllib.parse.urlencode({"type_codes": type_code})
        return self.get_json(KIBAN_API + f"geoid/update/{version}?" + q).get("results") or []

    # -- authenticated download ---------------------------------------------
    def login(self) -> None:
        if self.logged_in:
            return
        path = os.environ.get("GSI_LOGIN_CONF")
        if not path:
            raise SystemExit("GSI_LOGIN_CONF is not set (path to a GSI_USER=/GSI_PASS= file)")
        creds: dict[str, str] = {}
        with open(path, encoding="utf-8") as f:
            for line in f:
                line = line.rstrip("\n")
                if "=" in line:
                    k, v = line.split("=", 1)
                    creds[k.strip()] = v
        if "GSI_USER" not in creds or "GSI_PASS" not in creds:
            raise SystemExit(f"{path}: needs GSI_USER and GSI_PASS")
        html = self.get(
            KIBAN_API + "Login?next=https://service.gsi.go.jp/kiban/app/blank/", timeout=60
        ).decode("utf-8", "replace")
        m = re.search(r'<form id="kc-form-login"[^>]*action="([^"]+)"', html)
        if m:
            action = m.group(1).replace("&amp;", "&")
            data = urllib.parse.urlencode(
                {"username": creds["GSI_USER"], "password": creds["GSI_PASS"], "credentialId": ""}
            ).encode()
            del creds
            with self._op.open(urllib.request.Request(action, data=data), timeout=60) as r:
                if "kc-form-login" in r.read().decode("utf-8", "replace"):
                    raise SystemExit("GSI login failed (login form returned again)")
        st = self.get_json(KIBAN_API + "isLogin")
        if not (st.get("results") or {}).get("login"):
            raise SystemExit("GSI login failed (isLogin=false)")
        self.logged_in = True

    def download_file(self, file_id: int) -> bytes:
        self.login()
        data = self.get(KIBAN_API + f"download/file/{file_id}")
        if data[:2] != b"PK":
            raise RuntimeError(f"download {file_id}: not a zip ({len(data)} bytes)")
        return data
