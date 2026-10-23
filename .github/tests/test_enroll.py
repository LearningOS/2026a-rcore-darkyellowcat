"""Maintainer fallback uses the same verified enrollment implementation."""

import importlib.util
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("maintainer_enroll", ROOT / "enroll.py")
enroll = importlib.util.module_from_spec(spec)
spec.loader.exec_module(enroll)


class EnrollmentTests(unittest.TestCase):
    def test_invalid_login_is_rejected_before_provisioning(self):
        self.assertEqual(enroll.read_students(["Alayfolk64"], None), ["Alayfolk64"])
        for login in ("../bad", "student/name", "student@example.org", "student user", "-student"):
            with self.subTest(login=login), self.assertRaises(ValueError):
                enroll.read_students([login], None)

    def test_owner_fallback_uses_canonical_identity_and_verified_provisioner(self):
        answers = [{"state": "active", "role": "admin"}, {"type": "User", "login": "Student"}]
        with patch.object(sys, "argv", ["enroll.py", "student"]), \
                patch.object(enroll.subprocess, "run"), \
                patch.object(enroll.shutil, "which", return_value="gh"), \
                patch.object(enroll, "api", side_effect=answers), \
                patch.object(enroll, "provision", return_value=("https://github.com/ready", "https://github.com/check")) as provision:
            enroll.main()
        provision.assert_called_once_with("Student", enroll.COURSE_ID, enroll.COURSE)

    def test_non_owner_cannot_provision(self):
        with patch.object(sys, "argv", ["enroll.py", "student"]), \
                patch.object(enroll.subprocess, "run"), \
                patch.object(enroll.shutil, "which", return_value="gh"), \
                patch.object(enroll, "api", return_value={"state": "active", "role": "member"}), \
                patch.object(enroll, "provision") as provision:
            with self.assertRaisesRegex(ValueError, "owner"):
                enroll.main()
        provision.assert_not_called()

    def test_invalid_roster_user_stops_before_provisioning(self):
        with patch.object(sys, "argv", ["enroll.py", "student"]), \
                patch.object(enroll.subprocess, "run"), \
                patch.object(enroll.shutil, "which", return_value="gh"), \
                patch.object(enroll, "api", side_effect=[{"state": "active", "role": "admin"}, {"type": "Organization"}]), \
                patch.object(enroll, "provision") as provision:
            with self.assertRaisesRegex(ValueError, "personal"):
                enroll.main()
        provision.assert_not_called()


if __name__ == "__main__":
    unittest.main()
