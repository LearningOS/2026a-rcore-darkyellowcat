"""Regression checks for actual checker output and cumulative course scores."""

import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
from io import BytesIO


def load(name):
    path = Path(__file__).resolve().parents[1] / "scripts" / (name + ".py")
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


grade = load("rcore_grade")
publish = load("rcore_publish")


class GradingTests(unittest.TestCase):
    def test_organization_student_mapping(self):
        self.assertEqual(publish.student_login(
            "LearningOS/2026a-rcore-Alayfolk64", "LearningOS",
            "alayfolk64", "Alayfolk64"), "Alayfolk64")

    def test_reject_unassigned_template_wrong_student_and_personal_fork(self):
        cases = [
            ("LearningOS/2026a-rcore", "LearningOS", "Alayfolk64", ""),
            ("LearningOS/2026a-rcore", "LearningOS", "Alayfolk64", "Alayfolk64"),
            ("LearningOS/2026a-rcore-Alayfolk64", "LearningOS", "teacher", "Alayfolk64"),
            ("LearningOS/2026a-rcore-Alayfolk64", "LearningOS", "other", "other"),
            ("Alayfolk64/2026a-rcore-Alayfolk64", "Alayfolk64", "Alayfolk64", "Alayfolk64"),
        ]
        for case in cases:
            with self.subTest(case=case), self.assertRaises(ValueError):
                publish.student_login(*case)

    def test_real_randomized_checker_summary(self):
        # Captured from the official checker; its makefile rewrites 'passed'.
        output = "Test trace OK10564!\nTest passed10564: 7/7\nReport for lab1 found.\n"
        self.assertEqual(grade.parse_points(output), (7, 7))
        self.assertEqual(grade.parse_points("Test passed: 6/7\n"), (6, 7))

    def test_missing_ambiguous_or_invalid_summary(self):
        for output in ("", "Test passed4: 0/0", "Test passed4: 8/7",
                       "Test passed4: 7/7\nTest passed5: 7/7"):
            with self.subTest(output=output), self.assertRaises(ValueError):
                grade.parse_points(output)

    def test_chapters_accumulate_once_in_any_order(self):
        state = {}
        for index, chapter in enumerate(("ch8", "ch3", "ch6", "ch4", "ch5"), 1):
            state = publish.update_progress(state, "student/rcore", "student",
                                            chapter, "7/7", "commit")
            self.assertEqual(state["score"], index * 100)
        state = publish.update_progress(state, "student/rcore", "student", "ch3", "7/7", "retry")
        self.assertEqual(state["score"], 500)

    def test_partial_or_wrong_owner_is_rejected(self):
        with self.assertRaises(ValueError):
            publish.update_progress({}, "student/rcore", "student", "ch3", "6/7", "commit")
        state = publish.update_progress({}, "student/rcore", "student", "ch3", "7/7", "commit")
        with self.assertRaises(ValueError):
            publish.update_progress(state, "another/rcore", "another", "ch4", "7/7", "commit")

    def test_malformed_or_inconsistent_history_is_rejected(self):
        valid = publish.update_progress({}, "student/rcore", "student", "ch3", "7/7", "commit")
        for state in [[], dict(valid, totalScore=999), dict(valid, score=500), dict(valid, chapters={"ch3": None})]:
            with self.assertRaises(ValueError):
                publish.update_progress(state, "student/rcore", "student", "ch4", "16/16", "commit")

    def test_boolean_response_and_timeout_are_not_success(self):
        with patch.object(publish, "urlopen", return_value=BytesIO(b'{"result":true}')):
            with self.assertRaisesRegex(RuntimeError, "rejected"):
                publish.upload_score({}, "placeholder-only")
        with patch.object(publish, "urlopen", side_effect=TimeoutError("placeholder-only timeout")):
            with self.assertRaisesRegex(RuntimeError, "response interrupted") as caught:
                publish.upload_score({}, "placeholder-only")
        self.assertNotIn("placeholder-only", str(caught.exception))

    def test_http_success_is_not_business_success(self):
        for body in (b'{"result":400,"message":"user is not join"}', b'not JSON'):
            with self.subTest(body=body), patch.object(publish, "urlopen", return_value=BytesIO(body)):
                with self.assertRaises(RuntimeError):
                    publish.upload_score({}, "test-placeholder")
        with patch.object(publish, "urlopen", return_value=BytesIO(b'{"result":1}')):
            publish.upload_score({}, "test-placeholder")


if __name__ == "__main__":
    unittest.main()
