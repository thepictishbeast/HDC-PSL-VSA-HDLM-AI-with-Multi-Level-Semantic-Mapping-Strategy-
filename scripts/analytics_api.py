#!/usr/bin/env python3
"""
Fast Analytics API — DuckDB-powered endpoint for heavy aggregation queries.
Runs on port 3002, proxied by the main server for /api/classroom/* and /api/admin/dashboard.
"""

import duckdb
import json
import os
from http.server import HTTPServer, BaseHTTPRequestHandler

ANALYTICS_DB = os.path.expanduser("~/.local/share/plausiden/analytics.duckdb")
PORT = 3002

class AnalyticsHandler(BaseHTTPRequestHandler):
    duck = None

    def do_GET(self):
        if self.duck is None:
            AnalyticsHandler.duck = duckdb.connect(ANALYTICS_DB, read_only=True)

        if self.path == "/analytics/overview":
            self.send_overview()
        elif self.path == "/analytics/domains":
            self.send_domains()
        elif self.path == "/analytics/quality":
            self.send_quality()
        else:
            self.send_error(404)

    def send_overview(self):
        d = self.duck
        total = d.execute("SELECT COUNT(*) FROM fact_analytics").fetchone()[0]
        sources = d.execute("SELECT COUNT(DISTINCT source) FROM fact_analytics").fetchone()[0]
        domains = d.execute("SELECT COUNT(DISTINCT domain) FROM fact_analytics WHERE domain IS NOT NULL").fetchone()[0]
        avg_q = d.execute("SELECT AVG(quality_score) FROM fact_analytics").fetchone()[0]
        high = d.execute("SELECT COUNT(*) FROM fact_analytics WHERE quality_tier='high'").fetchone()[0]

        result = {
            "total_facts": total,
            "total_sources": sources,
            "total_domains": domains,
            "avg_quality": round(avg_q, 3) if avg_q else 0,
            "high_quality_count": high,
            "high_quality_pct": round(high / max(total, 1) * 100, 1),
        }
        self.send_json(result)

    def send_domains(self):
        rows = self.duck.execute("SELECT * FROM domain_stats").fetchall()
        domains = [{"domain": r[0], "count": r[1], "avg_quality": r[2], "avg_length": r[3], "high_count": r[4], "low_count": r[5]} for r in rows]
        self.send_json({"domains": domains})

    def send_quality(self):
        rows = self.duck.execute("SELECT * FROM quality_dist").fetchall()
        dist = {str(r[0]): r[1] for r in rows}
        self.send_json({"distribution": dist})

    def send_json(self, data):
        body = json.dumps(data).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Content-Length", len(body))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):
        pass  # Silent

if __name__ == "__main__":
    print(f"Analytics API on port {PORT} (DuckDB: {ANALYTICS_DB})")
    HTTPServer(("0.0.0.0", PORT), AnalyticsHandler).serve_forever()
