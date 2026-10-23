"""Maintainer fallback: python3 enroll.py LOGIN, or read students.txt."""

import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / ".github/scripts"))
from github_api import api, redact
from provision import ORGANIZATION, provision

COURSE_ID = "2073"
COURSE = {"title": "专业阶段 - rCore-Tutorial", "template": "2026a-rcore", "secret": "OSCAMP_2026A_RCORE_TOKEN", "branches": ["main", "ch1", "ch2", "ch3", "ch4", "ch5", "ch6", "ch7", "ch8"]}


def read_students(arguments, path):
    if len(arguments) > 1:
        raise ValueError("Usage: python3 enroll.py [GITHUB_LOGIN]")
    lines = arguments if arguments else path.read_text().splitlines()
    students = []
    for line in lines:
        login = line.split("#", 1)[0].strip()
        if not login:
            continue
        if not re.fullmatch(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,37}[A-Za-z0-9])?", login):
            raise ValueError(f"Invalid GitHub login: {login}")
        if login.lower() not in {name.lower() for name in students}:
            students.append(login)
    if not students:
        raise ValueError("Add GitHub logins to students.txt first.")
    return students


def main():
    os.chdir(ROOT)
    (ROOT / "tmp").mkdir(exist_ok=True)
    os.environ["TMPDIR"] = str(ROOT / "tmp")
    students = read_students(sys.argv[1:], ROOT / "students.txt")
    if not shutil.which("gh"):
        raise ValueError("The maintainer needs GitHub CLI: https://cli.github.com/ . Students do not need it.")
    subprocess.run(["gh", "auth", "status", "--hostname", "github.com"], check=True, timeout=30)
    membership = api("GET", "user/memberships/orgs/" + ORGANIZATION)
    if membership.get("state") != "active" or membership.get("role") != "admin":
        raise ValueError("Run this maintainer script as an owner of " + ORGANIZATION)
    roster = []
    for login in students:
        student = api("GET", "users/" + login)
        if student.get("type") != "User":
            raise ValueError(login + " is not a personal GitHub account.")
        roster.append(student["login"])
    for login in roster:
        url, check_url = provision(login, COURSE_ID, COURSE)
        print("Repository ready: " + url, flush=True)
        print("Configuration passed: " + check_url, flush=True)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, RuntimeError, OSError, KeyError, subprocess.SubprocessError) as error:
        sys.exit(redact(str(error)))
