#!/usr/bin/env python3
"""Sync Python server arcaea_log.db tables into the Rust MariaDB schema."""

from __future__ import annotations

import argparse
import os
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path
from urllib.parse import unquote, urlparse


LOG_TABLES = ("user_score", "user_rating")


def parse_database_url(url: str) -> dict[str, str | int]:
    parsed = urlparse(url)
    if parsed.scheme not in {"mysql", "mariadb"}:
        raise ValueError(f"Unsupported database URL scheme: {parsed.scheme}")
    if not parsed.hostname or not parsed.path.strip("/"):
        raise ValueError("DATABASE_URL must include host and database name")
    return {
        "host": parsed.hostname,
        "port": parsed.port or 3306,
        "user": unquote(parsed.username or ""),
        "password": unquote(parsed.password or ""),
        "database": parsed.path.strip("/"),
    }


def sql_identifier(name: str) -> str:
    return "`" + name.replace("`", "``") + "`"


def sql_literal(value: object) -> str:
    if value is None:
        return "NULL"
    if isinstance(value, bool):
        return "1" if value else "0"
    if isinstance(value, (int, float)):
        return str(value)
    if isinstance(value, bytes):
        return "0x" + value.hex()
    text = str(value)
    return "'" + text.replace("\\", "\\\\").replace("'", "''").replace("\0", "\\0") + "'"


def apply_database_url(args: argparse.Namespace) -> None:
    url = args.database_url or os.environ.get("DATABASE_URL")
    if not url:
        return
    info = parse_database_url(url)
    args.mysql_host = args.mysql_host or str(info["host"])
    args.mysql_port = args.mysql_port or int(info["port"])
    args.mysql_user = args.mysql_user or str(info["user"])
    args.mysql_password = args.mysql_password or str(info["password"])
    args.mysql_database = args.mysql_database or str(info["database"])


def mysql_cmd(args: argparse.Namespace, extra: list[str] | None = None) -> list[str]:
    cmd = [
        args.mysql_client,
        "--protocol=tcp",
        f"--host={args.mysql_host}",
        f"--port={args.mysql_port}",
        f"--user={args.mysql_user}",
        f"--database={args.mysql_database}",
        "--default-character-set=utf8mb4",
        "--binary-mode",
        "--show-warnings",
    ]
    if extra:
        cmd.extend(extra)
    return cmd


def run_mysql(args: argparse.Namespace, sql: str) -> str:
    env = os.environ.copy()
    env["MYSQL_PWD"] = args.mysql_password
    proc = subprocess.run(
        mysql_cmd(args, ["--batch", "--raw", "--skip-column-names", "--execute", sql]),
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(f"MySQL command failed ({proc.returncode}): {proc.stderr.strip()}")
    return proc.stdout


def table_columns(con: sqlite3.Connection, table: str) -> list[str]:
    return [row[1] for row in con.execute(f"PRAGMA table_info({sql_literal(table)})")]


def write_insert(out, table: str, columns: list[str], rows: list[sqlite3.Row]) -> None:
    if not rows:
        return
    column_sql = ", ".join(sql_identifier(column) for column in columns)
    values = []
    for row in rows:
        values.append("(" + ", ".join(sql_literal(value) for value in row) + ")")
    out.write(f"INSERT INTO {sql_identifier(table)} ({column_sql}) VALUES\n")
    out.write(",\n".join(values))
    out.write(";\n")


def build_import_sql(
    con: sqlite3.Connection,
    output_path: Path,
    batch_size: int,
    replace_existing: bool,
) -> dict[str, int]:
    counts: dict[str, int] = {}
    with output_path.open("w", encoding="utf-8") as out:
        out.write("SET NAMES utf8mb4;\n")
        if replace_existing:
            for table in reversed(LOG_TABLES):
                out.write(f"TRUNCATE TABLE {sql_identifier(table)};\n")
        for table in LOG_TABLES:
            columns = table_columns(con, table)
            if not columns:
                raise RuntimeError(f"SQLite table not found or has no columns: {table}")
            cursor = con.execute(f"SELECT {', '.join(sql_identifier(c) for c in columns)} FROM {sql_identifier(table)}")
            count = 0
            while True:
                rows = cursor.fetchmany(batch_size)
                if not rows:
                    break
                write_insert(out, table, columns, rows)
                count += len(rows)
            counts[table] = count
    return counts


def execute_sql_file(args: argparse.Namespace, path: Path) -> None:
    env = os.environ.copy()
    env["MYSQL_PWD"] = args.mysql_password
    with path.open("rb") as stdin:
        proc = subprocess.run(
            mysql_cmd(args),
            stdin=stdin,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
            check=False,
        )
    if proc.returncode != 0:
        raise RuntimeError(
            "MySQL import failed:\n"
            + proc.stderr.decode("utf-8", errors="replace")[-4000:]
        )


def compare_counts(args: argparse.Namespace, source_counts: dict[str, int]) -> list[tuple[str, int, int]]:
    sql = " UNION ALL ".join(
        f"SELECT {sql_literal(table)} AS table_name, COUNT(*) AS count FROM {sql_identifier(table)}"
        for table in LOG_TABLES
    )
    output = run_mysql(args, sql)
    target_counts = {}
    for line in output.splitlines():
        table, count = line.split("\t", 1)
        target_counts[table] = int(count)
    return [
        (table, source_count, target_counts.get(table, -1))
        for table, source_count in source_counts.items()
        if source_count != target_counts.get(table, -1)
    ]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Sync Python arcaea_log.db into Rust MariaDB log tables.")
    parser.add_argument("--sqlite", required=True, help="Path to arcaea_log.db")
    parser.add_argument("--database-url", help="MySQL/MariaDB connection URL")
    parser.add_argument("--mysql-host")
    parser.add_argument("--mysql-port", type=int)
    parser.add_argument("--mysql-user")
    parser.add_argument("--mysql-password")
    parser.add_argument("--mysql-database")
    parser.add_argument("--mysql-client", default="mariadb")
    parser.add_argument("--batch-size", type=int, default=500)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--keep-sql", help="Keep generated SQL at this path")
    parser.add_argument(
        "--append",
        action="store_true",
        help="Append rows instead of replacing existing user_score/user_rating data.",
    )
    args = parser.parse_args()
    apply_database_url(args)
    missing = [
        name
        for name in ("mysql_host", "mysql_port", "mysql_user", "mysql_password", "mysql_database")
        if getattr(args, name) in (None, "")
    ]
    if missing:
        parser.error(f"missing MySQL connection settings: {', '.join(missing)}")
    return args


def main() -> int:
    args = parse_args()
    sqlite_path = Path(args.sqlite)
    if not sqlite_path.is_file():
        raise FileNotFoundError(sqlite_path)
    con = sqlite3.connect(f"file:{sqlite_path}?mode=ro", uri=True)
    con.row_factory = sqlite3.Row

    with tempfile.TemporaryDirectory() as tmp:
        sql_path = Path(args.keep_sql) if args.keep_sql else Path(tmp) / "import_log.sql"
        counts = build_import_sql(con, sql_path, args.batch_size, not args.append)
        print("planned rows:")
        for table in LOG_TABLES:
            print(f"  {table}: {counts[table]}")
        if args.dry_run:
            print(f"dry run SQL: {sql_path}")
            return 0
        execute_sql_file(args, sql_path)

    mismatches = compare_counts(args, counts)
    if not args.append and mismatches:
        print("count mismatches:", file=sys.stderr)
        for table, source_count, target_count in mismatches:
            print(f"  {table}: source={source_count}, target={target_count}", file=sys.stderr)
        return 1
    print("log sync completed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
