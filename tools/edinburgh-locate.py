#! /usr/bin/env python3
import argparse
import subprocess

ENSEMBLE_DIRECTORY_BIN = "edinburgh-ensemble-directory"

ENSEMBLE_SCAN_TARGETS = [
    ["edi-ch.digris.net", 8851, 8866],
    ["edi-fr.digris.net", 8851, 8866],
    ["edi-uk.digris.net", 8851, 8866],
]


def main(query: str):
    targets_args = [
        arg
        for host, start, end in ENSEMBLE_SCAN_TARGETS
        for arg in ("--scan", f"{host}:{start}-{end}")
    ]

    cmd = [
        ENSEMBLE_DIRECTORY_BIN,
        *targets_args,
        "--once",
        "--scan-parallel",
        "12",
        "--scan-timeout",
        "1",
    ]

    result = subprocess.run(cmd, capture_output=True, text=True, check=True)

    for line in result.stdout.splitlines():
        if line.lower().startswith("svc") and query.lower() in line.lower():
            print(line)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("query", type=str)
    args = parser.parse_args()
    main(query=args.query)
