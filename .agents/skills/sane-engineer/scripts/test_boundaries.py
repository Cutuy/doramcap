#!/usr/bin/env python3
"""Behavioral fixtures for dependency reachability and rule validation."""
import unittest
from check_boundaries import violations


def graph(kind=None):
    return {"workspace_members": ["core", "adapter"], "packages": [
        {"id": "core", "name": "core", "manifest_path": "/fixture/core/Cargo.toml", "dependencies": [
            {"name": "adapter", "rename": "renamed", "kind": kind, "path": "/fixture/adapter"}]},
        {"id": "adapter", "name": "adapter", "manifest_path": "/fixture/adapter/Cargo.toml", "dependencies": [
            {"name": "bollard", "kind": None}]}]}


class Boundaries(unittest.TestCase):
    def test_transitive_and_renamed_dependency(self):
        # Arrange
        metadata = graph()
        # Act
        found = violations(metadata, {"rules": [{"from": "core", "forbid": ["bollard"]}]})
        # Assert
        self.assertEqual(found, ["core -> adapter -> bollard"])

    def test_dev_scope_and_direct_match(self):
        # Arrange
        metadata = graph("dev")
        rule = {"from": "core", "forbid": ["adapt*"]}
        # Act / Assert
        self.assertEqual(violations(metadata, {"rules": [rule]}), [])
        rule["kinds"] = ["dev"]
        self.assertEqual(violations(metadata, {"rules": [rule]}), ["core -> adapter"])

    def test_external_name_collision_is_not_traversed(self):
        # Arrange
        metadata = graph()
        metadata["packages"][0]["dependencies"][0].pop("path")
        # Act / Assert
        self.assertEqual(violations(metadata, {"rules": [{"from": "core", "forbid": ["bollard"]}]}), [])

    def test_invalid_policy_fails(self):
        # Arrange
        for rule in ({"from": "missing", "forbid": ["x"]}, {"from": "core", "forbid": ["x"], "kind": ["dev"]}):
            # Act / Assert
            with self.assertRaises(ValueError):
                violations(graph(), {"rules": [rule]})


if __name__ == "__main__":
    unittest.main()
