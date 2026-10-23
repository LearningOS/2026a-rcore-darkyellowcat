"""Persist fully passed chapters on gh-pages, then submit their cumulative score."""

import json
import os
from pathlib import Path
import re
import subprocess
import sys
from http.client import HTTPException
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

CHAPTERS = ("ch3", "ch4", "ch5", "ch6", "ch8")
COURSE_ID = 2073
TOTAL_SCORE = 500
API_URL = "https://api.opencamp.cn/web/api/courseRank/createByThirdToken"
ORGANIZATION = "LearningOS"


def student_login(repository, owner, actor, student):
    """Bind a course repository to its assigned student, including on retries."""
    if not re.fullmatch(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,37}[A-Za-z0-9])?", student):
        raise ValueError("STUDENT_GITHUB must be the student's GitHub login.")
    expected = f"{ORGANIZATION}/2026a-rcore-{student}"
    if owner.lower() != ORGANIZATION.lower() or repository.lower() != expected.lower():
        raise ValueError("Repository does not match the assigned course student.")
    if actor.lower() != student.lower():
        raise ValueError("Only the assigned student's runs can upload their score.")
    return student


def git(*args, cwd=None):
    return subprocess.check_output(["git", *args], cwd=cwd, text=True, timeout=30).strip()


def update_progress(state, repository, user, chapter, points, commit):
    if chapter not in CHAPTERS:
        raise ValueError("Unexpected chapter.")
    match = re.fullmatch(r"(\d+)/(\d+)", points)
    if not match or int(match[1]) <= 0 or match[1] != match[2]:
        raise ValueError("Only a fully passed chapter can be recorded.")
    if not isinstance(state, dict):
        raise ValueError("Invalid score history.")
    if state:
        if (state.get("schema") != 1 or state.get("courseId") != COURSE_ID
                or state.get("repository") != repository or state.get("user") != user
                or state.get("totalScore") != TOTAL_SCORE):
            raise ValueError("Existing score history belongs to a different course or user.")
        chapters = state.get("chapters")
        if not isinstance(chapters, dict) or any(ch not in CHAPTERS for ch in chapters):
            raise ValueError("Invalid chapter history.")
        if type(state.get("score")) is not int or state["score"] != 100 * len(chapters):
            raise ValueError("Invalid cumulative score history.")
        for record in chapters.values():
            if not isinstance(record, dict):
                raise ValueError("Invalid passed-chapter record.")
            old = re.fullmatch(r"(\d+)/(\d+)", record.get("points", ""))
            if record.get("score") != 100 or not old or old[1] != old[2] or int(old[1]) <= 0:
                raise ValueError("Invalid passed-chapter record.")
    else:
        state = {"schema": 1, "courseId": COURSE_ID, "repository": repository,
                 "user": user, "totalScore": TOTAL_SCORE, "chapters": {}}
    state["chapters"][chapter] = {"points": points, "score": 100, "commit": commit}
    state["score"] = 100 * len(state["chapters"])
    return state


def upload_score(payload, token):
    request = Request(API_URL, data=json.dumps(payload).encode(), method="POST",
                      headers={"Content-Type": "application/json", "Accept": "application/json",
                               "token": token})
    try:
        with urlopen(request, timeout=30) as response:
            body = response.read().decode()
    except HTTPError as error:
        details = error.read().decode(errors="replace").replace(token, "[REDACTED]")
        raise RuntimeError(f"OpenCamp HTTP {error.code}: {details}") from None
    except (TimeoutError, HTTPException) as error:
        raise RuntimeError(f"OpenCamp response interrupted: {str(error).replace(token, '[REDACTED]')}. Re-run the upload job.") from None
    except URLError as error:
        raise RuntimeError(f"OpenCamp connection failed: {str(error.reason).replace(token, '[REDACTED]')}") from None
    try:
        result = json.loads(body)
    except json.JSONDecodeError:
        raise RuntimeError(f"OpenCamp returned invalid JSON: {body.replace(token, '[REDACTED]')}") from None
    if not isinstance(result, dict) or type(result.get("result")) is not int or result["result"] != 1:
        raise RuntimeError(f"OpenCamp rejected the score: {body.replace(token, '[REDACTED]')}")
    print("OpenCamp accepted the score (result=1).")


def save_state(rank, state_path, state, message):
    state_path.write_text(json.dumps(state, indent=2) + "\n")
    git("add", state_path.name, cwd=rank)
    changed = subprocess.run(["git", "diff", "--cached", "--quiet"], cwd=rank, timeout=30)
    if changed.returncode == 1:
        git("-c", "user.name=github-actions[bot]", "-c",
            "user.email=41898282+github-actions[bot]@users.noreply.github.com", "commit",
            "-m", message, cwd=rank)
        git("push", "origin", "HEAD:gh-pages", cwd=rank)
    elif changed.returncode != 0:
        raise RuntimeError(f"Cannot inspect score changes (git exit {changed.returncode}).")


def main():
    token = os.environ.get("OSCAMP_2026A_RCORE_TOKEN", "")
    if not token:
        sys.exit("OSCAMP_2026A_RCORE_TOKEN is missing. Ask the maintainer to authorize this repository in the organization secret.")
    if os.environ.get("OSCAMP_COURSE_ID") != str(COURSE_ID):
        sys.exit("Unexpected course ID; nothing was uploaded.")
    repository = os.environ["GITHUB_REPOSITORY"]
    user = student_login(repository, os.environ["GITHUB_REPOSITORY_OWNER"],
                         os.environ["GITHUB_ACTOR"], os.environ.get("STUDENT_GITHUB", ""))
    branch = os.environ["GITHUB_REF_NAME"]
    points = os.environ["GRADE_POINTS"]
    commit = os.environ["GITHUB_SHA"]
    # Validate the current result before writing any Git state.
    update_progress({}, repository, user, branch, points, commit)
    root = Path(__file__).resolve().parents[2]
    os.chdir(root)
    rank = root / "rank"
    if rank.exists():
        sys.exit("rank already exists; use a fresh CI checkout.")
    refs = git("ls-remote", "--heads", "origin", "refs/heads/gh-pages")
    if refs:
        git("fetch", "--depth=1", "origin", "gh-pages")
        git("worktree", "add", "--detach", str(rank), "FETCH_HEAD")
    else:
        git("worktree", "add", "--detach", str(rank), "HEAD")
        git("switch", "--orphan", "gh-pages", cwd=rank)
    state_path = rank / f"course-{COURSE_ID}.json"
    state = json.loads(state_path.read_text()) if state_path.exists() else {}
    state = update_progress(state, repository, user, branch, points, commit)
    state["upload"] = {"status": "pending", "runId": os.environ["GITHUB_RUN_ID"],
                       "runAttempt": os.environ["GITHUB_RUN_ATTEMPT"]}
    save_state(rank, state_path, state, f"Record {branch} for OpenCamp course {COURSE_ID}")
    payload = {"channel": "github", "courseId": COURSE_ID, "name": user,
               "score": state["score"], "totalScore": TOTAL_SCORE, "ext": "{}"}
    print(f"Submitting measured score: course={COURSE_ID}, user={user}, "
          f"score={state['score']}/{TOTAL_SCORE}", flush=True)
    upload_score(payload, token)
    state["upload"]["status"] = "accepted"
    save_state(rank, state_path, state, f"Confirm OpenCamp {COURSE_ID} upload accepted")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
            summary.write(f"### OpenCamp upload accepted\n\nCourse: {COURSE_ID}. "
                          f"User: {user}. Score: **{state['score']}/{TOTAL_SCORE}**.\n")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, RuntimeError, OSError, subprocess.SubprocessError) as error:
        sys.exit(str(error))
