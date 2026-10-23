"""Run the official rCore checker, preserving its exit status and report checks."""

import json
import os
from pathlib import Path
import re
import subprocess
import sys

CHECKER_REV = "7d61ec55b58eed6ca7052917846c1b87af34563a"
TEST_REV = "a0593662ad55d670ba8c27ce1763347cd0dd552f"
CHAPTERS = {"ch3": 1, "ch4": 2, "ch5": 3, "ch6": 4, "ch8": 5}


def run(command):
    print("+ " + " ".join(command), flush=True)
    subprocess.run(command, check=True)


def parse_points(log):
    # The official checker randomizes every 'passed' marker, including its summary.
    matches = re.findall(r"^Test passed\d*: (\d+)/(\d+)\s*$", log, re.MULTILINE)
    if len(matches) != 1:
        raise ValueError("Expected exactly one official 'Test passed: N/M' summary.")
    got, total = map(int, matches[0])
    if total <= 0 or not 0 <= got <= total:
        raise ValueError("Invalid test counts in checker output.")
    return got, total


def main():
    root = Path(__file__).resolve().parents[2]
    os.chdir(root)
    tmp = root / "tmp"
    tmp.mkdir(exist_ok=True)
    os.environ["TMPDIR"] = str(tmp)
    branch = os.environ.get("GITHUB_REF_NAME")
    if not branch:
        branch = subprocess.check_output(["git", "branch", "--show-current"], text=True).strip()
    if branch not in CHAPTERS:
        sys.exit("Use one of ch3, ch4, ch5, ch6, ch8.")
    if (root / "ci-user").exists():
        sys.exit("ci-user already exists. Run this checker in a fresh disposable checkout.")
    run(["git", "config", "--global", "--add", "safe.directory", str(root)])
    run(["qemu-system-riscv64", "--version"])
    run(["rustup", "target", "add", "riscv64gc-unknown-none-elf"])
    run(["git", "clone", "https://github.com/LearningOS/rCore-Tutorial-Checker.git", "ci-user"])
    run(["git", "-C", "ci-user", "checkout", "--detach", CHECKER_REV])
    run(["git", "clone", "https://github.com/LearningOS/rCore-Tutorial-Test.git", "ci-user/user"])
    run(["git", "-C", "ci-user/user", "checkout", "--detach", TEST_REV])
    command = ["make", "test", "CHAPTER=" + branch[2:]]
    print("+ " + " ".join(command), flush=True)
    with (tmp / "rcore-grade.log").open("w") as log_file:
        process = subprocess.Popen(command, cwd="ci-user", stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT, text=True, errors="replace")
        for line in process.stdout:
            print(line, end="", flush=True)
            log_file.write(line)
        returncode = process.wait()
    log = (tmp / "rcore-grade.log").read_text()
    result = {"chapter": branch, "checker_commit": CHECKER_REV, "tests_commit": TEST_REV,
              "returncode": returncode, "passed": False}
    try:
        got, total = parse_points(log)
        result.update(points=f"{got}/{total}", passed=(returncode == 0 and got == total))
    except ValueError as error:
        result["error"] = str(error)
    (tmp / "rcore-result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
            summary.write(f"### {branch}\n\nTests: {result.get('points', 'no result')}. "
                          f"Chapter accepted: {result['passed']}. Checker exit: {returncode}.\n")
    if not result["passed"]:
        sys.exit(returncode if returncode > 0 else 1)
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            output.write(f"points={result['points']}\n")


if __name__ == "__main__":
    main()
