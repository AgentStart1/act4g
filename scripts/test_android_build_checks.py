"""Regression checks for rejecting invalid Android native packages."""

import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch
import zipfile


SPEC = importlib.util.spec_from_file_location(
    "android_build_checks", Path(__file__).with_name("android-build-checks.py")
)
checks = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checks)


class ApkChecksTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.library = self.directory / "libgact.so"
        self.elf = b"\x7fELF\x02\x01" + bytes(10) + struct.pack("<HH", 3, 183) + bytes(44)
        self.library.write_bytes(self.elf)
        self.native = patch.object(checks, "NATIVE_LIBRARY", self.library)
        self.native.start()
        self.addCleanup(self.native.stop)

    def package(self, libraries):
        apk = self.directory / "app.apk"
        with zipfile.ZipFile(apk, "w") as archive:
            for name, data in libraries.items():
                archive.writestr(name, data)
        return apk

    def test_accepts_matching_arm64_library(self):
        checks.verify_apk(self.package({"lib/arm64-v8a/libgact.so": self.elf}))

    def test_rejects_missing_application_library(self):
        with self.assertRaisesRegex(ValueError, "missing"):
            checks.verify_apk(self.package({}))

    def test_rejects_unexpected_abi(self):
        apk = self.package({
            "lib/arm64-v8a/libgact.so": self.elf,
            "lib/x86_64/libhelper.so": self.elf,
        })
        with self.assertRaisesRegex(ValueError, "Unexpected ABI"):
            checks.verify_apk(apk)

    def test_rejects_wrong_elf_architecture(self):
        x86 = self.elf[:18] + struct.pack("<H", 62) + self.elf[20:]
        with self.assertRaisesRegex(ValueError, "AArch64 ELF64"):
            checks.verify_apk(self.package({"lib/arm64-v8a/libgact.so": x86}))

    def test_rejects_truncated_library(self):
        with self.assertRaisesRegex(ValueError, "AArch64 ELF64"):
            checks.verify_apk(self.package({"lib/arm64-v8a/libgact.so": b"\x7fELF"}))

    def test_rejects_stale_application_library(self):
        with self.assertRaisesRegex(ValueError, "differs from the cargo-ndk output"):
            checks.verify_apk(self.package({"lib/arm64-v8a/libgact.so": self.elf + b"stale"}))


if __name__ == "__main__":
    unittest.main()
