"""Check the Android toolchain selection and the native library shipped in an APK."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import zipfile


ROOT = Path(__file__).resolve().parents[1]
NATIVE_LIBRARY = ROOT / "android/gradle/app/src/main/jniLibs/arm64-v8a/libgact.so"


def verify_ndk():
    properties = dict(
        line.split("=", 1)
        for line in (ROOT / "android/gradle/gradle.properties").read_text().splitlines()
        if "=" in line
    )
    version = properties["gact.ndkVersion"]
    ndk = Path(os.environ["ANDROID_NDK_HOME"]).resolve(strict=True)
    for name in ("ANDROID_NDK_ROOT", "NDK_HOME"):
        if Path(os.environ[name]).resolve(strict=True) != ndk:
            raise ValueError(f"{name} does not match ANDROID_NDK_HOME: {ndk}")
    source_properties = dict(
        (key.strip(), value.strip())
        for key, value in (
            line.split("=", 1)
            for line in (ndk / "source.properties").read_text().splitlines()
            if "=" in line
        )
    )
    if source_properties["Pkg.Revision"] != version:
        raise ValueError(f"Expected NDK {version}, got {source_properties['Pkg.Revision']}")
    environment = json.loads(subprocess.check_output(
        ["cargo", "ndk-env", "-t", "arm64-v8a", "-P", "26", "--json"], text=True
    ))
    clang = Path(environment["CLANG_PATH"])
    sysroot = Path(environment["CARGO_NDK_SYSROOT_PATH"]).resolve(strict=True)
    if not clang.resolve().is_relative_to(ndk) or not sysroot.is_relative_to(ndk):
        raise ValueError(f"cargo-ndk selected another toolchain: {clang}, {sysroot}")
    print(f"cargo-ndk NDK {version}: {ndk}", flush=True)
    subprocess.run([str(clang), "--version"], check=True)


def verify_apk(apk):
    with zipfile.ZipFile(apk) as archive:
        libraries = [name for name in archive.namelist() if name.startswith("lib/") and name.endswith(".so")]
        if "lib/arm64-v8a/libgact.so" not in libraries:
            raise ValueError("APK is missing lib/arm64-v8a/libgact.so")
        for name in libraries:
            if name.split("/")[1] != "arm64-v8a":
                raise ValueError(f"Unexpected ABI in APK: {name}")
            with archive.open(name) as library:
                header = library.read(20)
            if len(header) < 20 or header[:6] != b"\x7fELF\x02\x01" or struct.unpack("<HH", header[16:20]) != (3, 183):
                raise ValueError(f"Expected an AArch64 ELF64 shared library: {name}")
        with archive.open("lib/arm64-v8a/libgact.so") as library:
            packaged_hash = hashlib.file_digest(library, "sha256").hexdigest()
    with NATIVE_LIBRARY.open("rb") as library:
        built_hash = hashlib.file_digest(library, "sha256").hexdigest()
    if packaged_hash != built_hash:
        raise ValueError("Packaged libgact.so differs from the cargo-ndk output")
    print(f"Verified {apk}: {', '.join(libraries)}")
    print(f"libgact.so SHA-256: {packaged_hash}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("ndk")
    apk = commands.add_parser("apk")
    apk.add_argument("path", type=Path)
    args = parser.parse_args()
    if args.command == "ndk":
        verify_ndk()
    else:
        verify_apk(args.path)


if __name__ == "__main__":
    main()
