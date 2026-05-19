#!/usr/bin/env python3
"""Sync the Python server SQLite database into the Rust server MariaDB schema.

The script intentionally uses only the Python standard library plus the
`mariadb`/`mysql` command line client so it can run on the server without
installing Python database packages.
"""

from __future__ import annotations

import argparse
import os
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path
from urllib.parse import quote, unquote, urlparse


SKIP_TABLES = {
    "download_token",
    "bundle_download_token",
    "_sqlx_migrations",
}

TRUNCATE_ORDER = [
    "download_token",
    "bundle_download_token",
    "notification",
    "login",
    "api_login",
    "songplay_token",
    "friend",
    "best_score",
    "recent30",
    "user_world",
    "user_save",
    "user_present",
    "present_item",
    "present",
    "user_redeem",
    "redeem_item",
    "redeem",
    "user_role",
    "role_power",
    "role",
    "power",
    "user_mission",
    "user_kvdata",
    "user_custom_course",
    "user_course",
    "course_item",
    "course_requirement",
    "course_chart",
    "course",
    "user_item",
    "purchase_item",
    "purchase",
    "item",
    "char_item",
    "user_char_full",
    "user_char",
    "character",
    "chart",
    "config",
    "user",
]


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


def shell_env_database_url() -> str | None:
    return os.environ.get("DATABASE_URL")


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
        raise RuntimeError(
            f"MySQL command failed ({proc.returncode}): {proc.stderr.strip()}"
        )
    return proc.stdout


def mysql_columns(args: argparse.Namespace) -> dict[str, list[str]]:
    sql = (
        "SELECT TABLE_NAME, COLUMN_NAME "
        "FROM INFORMATION_SCHEMA.COLUMNS "
        f"WHERE TABLE_SCHEMA = {sql_literal(args.mysql_database)} "
        "ORDER BY TABLE_NAME, ORDINAL_POSITION"
    )
    rows = run_mysql(args, sql).splitlines()
    result: dict[str, list[str]] = {}
    for row in rows:
        if not row:
            continue
        table, column = row.split("\t", 1)
        result.setdefault(table, []).append(column)
    return result


def sqlite_columns(con: sqlite3.Connection) -> dict[str, list[str]]:
    rows = con.execute(
        "SELECT name FROM sqlite_master "
        "WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
    ).fetchall()
    result: dict[str, list[str]] = {}
    for (table,) in rows:
        result[table] = [r[1] for r in con.execute(f'PRAGMA table_info("{table}")')]
    return result


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


def source_expr(table: str, column: str) -> str:
    if table == "user" and column == "highest_rating_ptt":
        return "rating_ptt"
    return sql_identifier(column)


def select_rows(
    con: sqlite3.Connection,
    table: str,
    source_cols: list[str],
    target_cols: list[str],
    batch_size: int,
):
    exprs = [source_expr(table, col) for col in target_cols]
    query = f"SELECT {', '.join(exprs)} FROM {sql_identifier(table)}"
    cursor = con.execute(query)
    while True:
        rows = cursor.fetchmany(batch_size)
        if not rows:
            break
        yield rows


def write_insert(
    out,
    table: str,
    target_cols: list[str],
    rows: list[sqlite3.Row],
) -> None:
    if not rows:
        return
    columns = ", ".join(sql_identifier(col) for col in target_cols)
    values = []
    for row in rows:
        values.append("(" + ", ".join(sql_literal(value) for value in row) + ")")
    out.write(f"INSERT INTO {sql_identifier(table)} ({columns}) VALUES\n")
    out.write(",\n".join(values))
    out.write(";\n")


def build_import_sql(
    args: argparse.Namespace,
    con: sqlite3.Connection,
    source: dict[str, list[str]],
    target: dict[str, list[str]],
    output_path: Path,
) -> dict[str, int]:
    counts: dict[str, int] = {}
    import_tables = [
        table
        for table in source
        if table in target and table not in SKIP_TABLES
    ]

    with output_path.open("w", encoding="utf-8") as out:
        out.write("SET NAMES utf8mb4;\n")
        out.write("SET FOREIGN_KEY_CHECKS=0;\n")
        out.write("SET UNIQUE_CHECKS=0;\n")
        if "user" in target and "name" in target["user"]:
            out.write(
                "ALTER TABLE `user` MODIFY `name` "
                "VARCHAR(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin;\n"
            )
        for table in TRUNCATE_ORDER:
            if table in target and table not in {"_sqlx_migrations"}:
                out.write(f"TRUNCATE TABLE {sql_identifier(table)};\n")

        for table in import_tables:
            source_cols = source[table]
            target_cols = [col for col in target[table] if col in source_cols]
            if table == "user" and "highest_rating_ptt" in target[table]:
                if "highest_rating_ptt" not in target_cols:
                    insert_at = target[table].index("highest_rating_ptt")
                    preceding = [
                        col for col in target[table][:insert_at] if col in target_cols
                    ]
                    target_cols.insert(len(preceding), "highest_rating_ptt")
            if not target_cols:
                continue

            row_count = 0
            for rows in select_rows(
                con, table, source_cols, target_cols, args.batch_size
            ):
                write_insert(out, table, target_cols, rows)
                row_count += len(rows)
            counts[table] = row_count

        out.write("SET UNIQUE_CHECKS=1;\n")
        out.write("SET FOREIGN_KEY_CHECKS=1;\n")

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


def compare_counts(
    args: argparse.Namespace,
    source_counts: dict[str, int],
) -> list[tuple[str, int, int]]:
    tables = sorted(source_counts)
    sql = " UNION ALL ".join(
        f"SELECT {sql_literal(table)} AS table_name, COUNT(*) AS count FROM {sql_identifier(table)}"
        for table in tables
    )
    output = run_mysql(args, sql)
    target_counts = {}
    for line in output.splitlines():
        table, count = line.split("\t", 1)
        target_counts[table] = int(count)
    mismatches = []
    for table, source_count in source_counts.items():
        target_count = target_counts.get(table, -1)
        if source_count != target_count:
            mismatches.append((table, source_count, target_count))
    return mismatches


def apply_database_url(args: argparse.Namespace) -> None:
    url = args.database_url or shell_env_database_url()
    if not url:
        return
    info = parse_database_url(url)
    args.mysql_host = args.mysql_host or str(info["host"])
    args.mysql_port = args.mysql_port or int(info["port"])
    args.mysql_user = args.mysql_user or str(info["user"])
    args.mysql_password = args.mysql_password or str(info["password"])
    args.mysql_database = args.mysql_database or str(info["database"])


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Sync Python server arcaea_database.db into Rust MariaDB."
    )
    parser.add_argument("--sqlite", required=True, help="Path to arcaea_database.db")
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
    args = parser.parse_args()
    apply_database_url(args)
    missing = [
        name
        for name in [
            "mysql_host",
            "mysql_port",
            "mysql_user",
            "mysql_password",
            "mysql_database",
        ]
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

    con = sqlite3.connect(f"file:{quote(str(sqlite_path))}?mode=ro", uri=True)
    con.row_factory = sqlite3.Row
    source = sqlite_columns(con)
    target = mysql_columns(args)

    with tempfile.TemporaryDirectory() as tmp:
        sql_path = Path(args.keep_sql) if args.keep_sql else Path(tmp) / "import.sql"
        counts = build_import_sql(args, con, source, target, sql_path)
        print("planned rows:")
        for table in sorted(counts):
            print(f"  {table}: {counts[table]}")
        if args.dry_run:
            print(f"dry run SQL: {sql_path}")
            return 0
        execute_sql_file(args, sql_path)

    mismatches = compare_counts(args, counts)
    if mismatches:
        print("count mismatches:", file=sys.stderr)
        for table, source_count, target_count in mismatches:
            print(f"  {table}: source={source_count}, target={target_count}", file=sys.stderr)
        return 1
    print("sync completed; row counts match")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
