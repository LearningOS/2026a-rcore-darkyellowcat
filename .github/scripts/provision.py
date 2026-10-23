"""Prepare course repositories and publish their formal names after verified CI."""

import time
from github_api import GitHubError, REQUEST_BUDGET, REQUEST_TIMEOUT, api

ORGANIZATION = "LearningOS"
CONFIGURATION_TIMEOUT = 600
POLL_INTERVAL = 10


class ConfigurationError(RuntimeError):
    def __init__(self, message, url):
        super().__init__(f"{message}: {url}")
        self.url = url


def check_configuration(repository):
    endpoint = "repos/" + repository
    # Dispatch metadata identifies this exact run, even if other runs exist.
    dispatched = api("POST", endpoint + "/actions/workflows/check-config.yml/dispatches",
                     {"ref": "main", "return_run_details": True}, retry=True)
    run_id = (dispatched or {}).get("workflow_run_id")
    if type(run_id) is not int or run_id <= 0:
        raise RuntimeError("GitHub did not return a configuration run ID; enrollment was not confirmed.")
    url = f"https://github.com/{repository}/actions/runs/{run_id}"
    deadline = time.monotonic() + CONFIGURATION_TIMEOUT
    while time.monotonic() < deadline:
        run = api("GET", endpoint + f"/actions/runs/{run_id}", missing_ok=True)
        if run is not None and run["status"] == "completed":
            if run["conclusion"] != "success":
                raise ConfigurationError(f"Configuration check ended with {run['conclusion']}", url)
            jobs = api("GET", endpoint + f"/actions/runs/{run_id}/jobs")
            if not any(job["name"] == "configuration" and job["conclusion"] == "success"
                       for job in jobs["jobs"]):
                raise ConfigurationError("Configuration job did not actually pass (missing or skipped)", url)
            print("Configuration check passed: " + url, flush=True)
            return url
        status = run["status"] if run is not None else "not yet visible"
        print(f"Waiting for configuration check ({status}): {url}", flush=True)
        time.sleep(POLL_INTERVAL)
    raise ConfigurationError("Configuration check was not completed within 10 minutes", url)


def validate_source(repo, template):
    if repo.get("archived") or repo.get("disabled"):
        raise ValueError("Repository is archived or disabled; left untouched.")
    source_name = (repo.get("template_repository") or {}).get("full_name", "")
    if source_name.lower() != template.lower() or repo.get("private"):
        raise ValueError(repo.get("full_name", "Repository") +
                         " already exists with a different source; left untouched.")


def prepare_repository(repository, template, course, login, course_id):
    endpoint = "repos/" + repository
    repo = api("GET", endpoint, missing_ok=True)
    if repo is not None:
        validate_source(repo, template)
    else:
        try:
            api("POST", "repos/" + template + "/generate", {
                "owner": ORGANIZATION, "name": repository.split("/", 1)[1],
                "private": False, "include_all_branches": True,
                "description": f"{course['title']} - {login}",
            }, retry=True)
        except GitHubError as error:
            if not error.temporary and error.status != 422:
                raise
            print(str(error), flush=True)
            repo = api("GET", endpoint, missing_ok=True)
            if repo is None:
                raise
            validate_source(repo, template)
            print("Preparation repository exists after create error; continuing configuration.")

    for attempt in range(30):
        repo = api("GET", endpoint, missing_ok=True)
        branches = api("GET", endpoint + "/branches?per_page=100", missing_ok=True) if repo else None
        if repo and branches is not None and set(course["branches"]).issubset(
                {item["name"] for item in branches}):
            validate_source(repo, template)
            return repo
        if attempt == 29:
            raise ValueError("Repository generation is incomplete; preparation was not published.")
        time.sleep(2)


def publish_repository(preparing, final_repository, repository_id):
    # Rename is the final configuration write. All setup and the exact CI
    # check have passed, and repository invitations follow the repository ID.
    deadline = time.monotonic() + REQUEST_BUDGET
    for attempt in range(1, 5):
        delay = 2 ** attempt
        existing = api("GET", "repos/" + final_repository, missing_ok=True)
        if existing is not None:
            if existing["id"] != repository_id:
                raise ValueError("The final repository name is already occupied; nothing was overwritten.")
            return
        current = api("GET", "repos/" + preparing)
        if current["id"] != repository_id:
            raise ValueError("Preparation repository identity changed; nothing was renamed.")
        try:
            api("PATCH", "repos/" + preparing, {
                "name": final_repository.split("/", 1)[1],
                "description": "OpenCamp coursework; enrollment configuration verified",
            }, retry=False)
        except GitHubError as error:
            # A lost rename response can still mean success. Check the exact
            # repository ID before retrying, never another student's repo.
            if not error.temporary and error.status not in {404, 422}:
                raise
            print(str(error), flush=True)
            delay = max(delay, error.retry_after or 0)
            if error.status in {403, 429} and error.temporary:
                # A rejected rate-limit request did not rename the repository.
                # Do not issue even a reconciliation GET before the allowed wait.
                if attempt == 4:
                    raise
                if time.monotonic() + delay + REQUEST_TIMEOUT > deadline:
                    raise RuntimeError(f"Rename requires waiting {delay}s; no early retry was sent. Retry the application later.")
                time.sleep(delay)
                continue
            existing = api("GET", "repos/" + final_repository, missing_ok=True)
            if existing is not None and existing["id"] == repository_id:
                return
            if attempt == 4:
                raise
        else:
            existing = api("GET", "repos/" + final_repository, missing_ok=True)
            if existing is not None and existing["id"] == repository_id:
                return
            if attempt == 4:
                raise RuntimeError("Cannot verify the published repository ID.")
        if time.monotonic() + delay + REQUEST_TIMEOUT > deadline:
            raise RuntimeError(f"Rename requires waiting {delay}s; no early retry was sent. Retry the application later.")
        time.sleep(delay)


def provision(login, course_id, course):
    # Finish all checks that do not need a repository before creating one.
    student = api("GET", "users/" + login)
    if student["type"] != "User" or student["login"].lower() != login.lower():
        raise ValueError("The applicant is not the expected personal GitHub account.")
    template = ORGANIZATION + "/" + course["template"]
    source = api("GET", "repos/" + template)
    if not source.get("is_template") or source.get("private") or source.get("archived") or source.get("disabled"):
        raise ValueError(template + " must be a public template repository.")
    branches = api("GET", "repos/" + template + "/branches?per_page=100")
    if not set(course["branches"]).issubset({item["name"] for item in branches}):
        raise ValueError("Course template is missing required branches; no repository was created.")
    secret = api("GET", f"orgs/{ORGANIZATION}/actions/secrets/{course['secret']}")
    if secret.get("visibility") not in {"all", "selected"}:
        raise ValueError("The course organization secret must allow public repositories.")

    final_repository = template + "-" + login
    repo = api("GET", "repos/" + final_repository, missing_ok=True)
    if repo is None:
        repository = ORGANIZATION + "/preparing-" + course["template"] + "-" + login
        repo = prepare_repository(repository, template, course, login, course_id)
    else:
        validate_source(repo, template)
        repository = final_repository
        print("Checking existing course repository; student code is preserved: " + repository)
    endpoint = "repos/" + repository

    variable_path = endpoint + "/actions/variables/STUDENT_GITHUB"
    variable = api("GET", variable_path, missing_ok=True)
    if variable is None:
        try:
            api("POST", endpoint + "/actions/variables", {"name": "STUDENT_GITHUB", "value": login},
                retry=True)
        except GitHubError as error:
            if not error.temporary and error.status != 422:
                raise
            print(str(error), flush=True)
            variable = api("GET", variable_path, missing_ok=True)
            if variable is None:
                raise
        if variable is None:
            variable = api("GET", variable_path)
    if variable["value"].lower() != login.lower():
        raise ValueError("Repository belongs to another student; identity was not overwritten.")

    # Grant only this course credential before checking the student repository.
    if secret["visibility"] == "selected":
        api("PUT", f"orgs/{ORGANIZATION}/actions/secrets/{course['secret']}/repositories/{repo['id']}")

    api("PUT", endpoint + "/actions/workflows/check-config.yml/enable")
    check_url = check_configuration(repository)
    api("PUT", endpoint + "/actions/workflows/build.yml/enable")
    api("PUT", endpoint + "/collaborators/" + login, {"permission": "push"})
    if repository != final_repository:
        publish_repository(repository, final_repository, repo["id"])
        check_url = check_url.replace(repository, final_repository, 1)
        print("Preparation passed; formal repository published: " + final_repository, flush=True)
    return "https://github.com/" + final_repository, check_url

