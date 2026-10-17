#!/usr/bin/env python3

# Formatted with ruff.

import argparse
import logging
import os
import pathlib
import platform
import shutil
from abc import abstractmethod
from typing import Optional, Protocol

MSRV = "1.90.0"
CARGO_MSRV = "cargo +1.90.0"

WINDOWS = platform.system().startswith("Windows") or platform.system().startswith(
    "CYGWIN_NT"
)

SCCACHE = shutil.which("sccache")
NO_CACHE = False


def run(cmd):
    assert isinstance(cmd, str)
    logging.info(cmd)
    os.system(cmd)


def env(name, value):
    os.environ[name] = value
    logging.info(f"Set {name}={value}")


def sccache():
    if SCCACHE is None:
        logging.warning("'sccache' not found!")
        return
    if NO_CACHE:
        logging.warning("'sccache' is disabled!")
        return
    env("RUSTC_WRAPPER", SCCACHE)


def rustflags():
    flags = ["-Dwarnings"]

    if WINDOWS:
        # `v8`'s prebuilt static libraries are built with the static CRT
        # (`/MT`, `libcmt.lib`), so Rust must link the static CRT too. This
        # mirrors `.cargo/config.toml`, which RUSTFLAGS would otherwise override.
        flags.append("-Ctarget-feature=+crt-static")

    flags = " ".join(flags)
    env("RUSTFLAGS", flags)


def rustdocflags():
    flags = ["-Dwarnings"]

    flags = " ".join(flags)
    env("RUSTDOCFLAGS", flags)


def rust_backtrace():
    env("RUST_BACKTRACE", "full")


def miriflags():
    env(
        "MIRIFLAGS",
        "-Zmiri-backtrace=full -Zmiri-disable-isolation -Zmiri-permissive-provenance",
    )


def rsvim_log():
    var = os.getenv("RSVIM_LOG")
    if isinstance(var, str):
        env("RSVIM_LOG", var)
    else:
        env("RSVIM_LOG", "trace")


class Cmd(Protocol):
    @abstractmethod
    def run(self, args) -> None:
        raise Exception("Not implemented!")

    @abstractmethod
    def name(self) -> str:
        raise Exception("Not implemented!")

    @abstractmethod
    def alias(self) -> Optional[str]:
        raise Exception("Not implemented!")


# clippy/c
class Clippy(Cmd):
    def __init__(self, subparsers) -> None:
        self._name = "clippy"
        self._alias = "c"

        self.clippy_parser = subparsers.add_parser(
            self._name,
            aliases=[self._alias],
            help="cargo clippy",
        )
        self.clippy_parser.add_argument(
            "-m",
            "--minimal-version",
            action="store_true",
            dest="minimal_version",
            help=f"With minimal rustc version {MSRV}",
        )
        self.clippy_parser.add_argument("-v", "--verbose", action="store_true")

    def name(self) -> str:
        return self._name

    def alias(self) -> Optional[str]:
        return self._alias

    def run(self, args) -> None:
        sccache()
        rustflags()

        if args.minimal_version:
            cmd = f"{CARGO_MSRV} clippy --workspace --all-targets"
        else:
            cmd = "cargo clippy --workspace --all-targets"
        if args.verbose:
            cmd = f"{cmd} --verbose"
        run(cmd)


# test/t
class Test(Cmd):
    def __init__(self, subparsers) -> None:
        self._name = "test"
        self._alias = "t"

        self.test_parser = subparsers.add_parser(
            self._name,
            aliases=[self._alias],
            help="cargo test",
        )
        self.test_parser.add_argument(
            "-NF", "--no-default-features", action="store_true"
        )
        self.test_parser.add_argument(
            "-l", "--list", action="store_true", dest="list_test"
        )
        self.test_parser.add_argument(
            "-m",
            "--minimal-version",
            action="store_true",
            dest="minimal_version",
            help=f"With minimal rustc version {MSRV}",
        )
        self.test_parser.add_argument("name", nargs="*", default=[])

    def name(self) -> str:
        return self._name

    def alias(self) -> Optional[str]:
        return self._alias

    def run(self, args) -> None:
        if args.list_test:
            self.list()
        else:
            self.test(args.name, args.no_default_features, args.minimal_version)

    def test(self, name, no_default_features, minimal_version) -> None:
        sccache()
        rustflags()
        rust_backtrace()
        rsvim_log()

        if minimal_version:
            cmd = f"{CARGO_MSRV} nextest run --no-capture"
        else:
            cmd = "cargo nextest run --no-capture"

        if no_default_features:
            cmd = f"{cmd} --no-default-features"

        if len(name) == 0:
            cmd = f"{cmd} --all"
        else:
            name = " ".join(list(dict.fromkeys(name)))
            cmd = f"{cmd} {name}"
        run(cmd)

    def list(self) -> None:
        sccache()
        rustflags()
        cmd = "cargo nextest list --no-pager"
        run(cmd)


# miri
class Miri(Cmd):
    def __init__(self, subparsers) -> None:
        self._name = "miri"

        self.test_parser = subparsers.add_parser(
            self._name,
            help="cargo +nightly miri test",
        )
        self.test_parser.add_argument("name", nargs="*", default=[])
        self.test_parser.add_argument("-j", "--job", nargs=1, metavar="N")

    def name(self) -> str:
        return self._name

    def alias(self) -> Optional[str]:
        return None

    def run(self, args) -> None:
        rustflags()
        rust_backtrace()
        miriflags()
        if args.job is None:
            job = ""
        else:
            job = f" -j {args.job[0]}"
        cmd = f"cargo +nightly miri nextest run{job} --no-default-features -F unicode -p rsvim_core"
        run(cmd)


# build/b
class Build(Cmd):
    def __init__(self, subparsers) -> None:
        self._name = "build"
        self._alias = "b"

        self.build_parser = subparsers.add_parser(
            self._name,
            aliases=[self._alias],
            help="cargo build",
        )
        self.build_parser.add_argument("-v", "--verbose", action="store_true")
        self.build_parser.add_argument("-F", "--features", action="append")
        self.build_parser.add_argument(
            "-NF", "--no-default-features", action="store_true"
        )
        self.build_parser.add_argument("-r", "--release", action="store_true")
        self.build_parser.add_argument("-n", "--nightly", action="store_true")
        self.build_parser.add_argument(
            "-m",
            "--minimal-version",
            action="store_true",
            dest="minimal_version",
            help="With minimal rust toolchain version 1.90.0",
        )

    def name(self) -> str:
        return self._name

    def alias(self) -> Optional[str]:
        return self._alias

    def run(self, args) -> None:
        sccache()
        rustflags()
        if args.minimal_version:
            cmd = "cargo +1.90 build"
        else:
            cmd = "cargo build"
        if args.release:
            cmd = f"{cmd} --release"
        elif args.nightly:
            cmd = f"{cmd} --profile nightly"
        if args.verbose:
            cmd = f"{cmd} -vv"
        if args.no_default_features:
            cmd = f"{cmd} --no-default-features"
        if isinstance(args.features, list) and len(args.features) > 0:
            feat = ",".join(args.features)
            cmd = f"{cmd} -F {feat}"
        run(cmd)


# bench
class Bench(Cmd):
    def __init__(self, subparsers) -> None:
        self._name = "bench"

        self.bench_parser = subparsers.add_parser(
            self._name,
            help="cargo bench",
        )
        self.bench_parser.add_argument("name", nargs=1)

    def name(self) -> str:
        return self._name

    def alias(self) -> Optional[str]:
        return None

    def run(self, args) -> None:
        name = args.name[0]
        cmd = f"cargo bench -p rsvim_core --bench {name}"
        run(cmd)


# fmt/f
class Format(Cmd):
    def __init__(self, subparsers) -> None:
        self._name = "fmt"
        self._alias = "f"

        self.fmt_parser = subparsers.add_parser(
            self._name,
            aliases=[self._alias],
            help="code formattors",
        )
        self.fmt_parser.add_argument("-r", "--rust", action="store_true")
        self.fmt_parser.add_argument("-t", "--tsc", action="store_true")

    def name(self) -> str:
        return self._name

    def alias(self) -> Optional[str]:
        return self._alias

    def run(self, args) -> None:
        if args.tsc:
            self.tsc()
        elif args.rust:
            self.rustfmt()
        else:
            for cmd in [
                self.others,
                self.rustfmt,
                self.tsc,
            ]:
                cmd()

    def others(self):
        typos = "typos"
        taplo = "taplo fmt"
        prettier = "prettier --write *.md ./runtime/*.ts"
        for cmd in [typos, taplo, prettier]:
            run(cmd)

    def rustfmt(self):
        cmd = "cargo +nightly fmt"
        run(cmd)

    def tsc(self):
        cmd = "tsc"

        # Compile .ts files into .js files
        run(cmd)

        # Minify .js files into .min.js files
        for file in ["00__web", "01__rsvim"]:
            src_file = f"./rsvim_core/src/js/runtime/{file}.js"
            dest_file = f"./rsvim_core/src/js/runtime/{file}.min.js"
            cmd = f"terser {src_file} --keep-classnames --keep-fnames -o {dest_file}"
            run(cmd)

        # Post-process .d.ts files to avoid boring lint warnings
        for file in ["00__web.d.ts", "01__rsvim.d.ts"]:
            src_file = f"types/{file}"
            dest_file = f".{file}"
            with open(src_file, "r") as sfp:
                with open(dest_file, "w") as dfp:
                    dfp.write("// @ts-nocheck\n")
                    for line in sfp.readlines():
                        dfp.write(line)
            cmd = f"mv {dest_file} {src_file}"
            run(cmd)


# doc
class Document(Cmd):
    def __init__(self, subparsers) -> None:
        self._name = "doc"

        self.doc_parser = subparsers.add_parser(
            self._name,
            help="cargo doc",
        )

    def name(self) -> str:
        return self._name

    def alias(self) -> Optional[str]:
        return None

    def run(self, args) -> None:
        rustdocflags()
        cmd = "cargo doc && browser-sync start --ss target/doc -s target/doc --directory --startPath rsvim_core --no-open"
        run(cmd)


# release
class Release(Cmd):
    def __init__(self, subparsers) -> None:
        self._name = "release"

        self.release_parser = subparsers.add_parser(
            self._name,
            help="cargo release",
        )
        self.release_parser.add_argument(
            "level",
            choices=["alpha", "beta", "rc", "major", "minor", "patch"],
            help="Release [LEVEL]",
        )
        self.release_parser.add_argument(
            "-e",
            "--execute",
            action="store_true",
            help="Execute `cargo release` with `--no-verify`",
        )

    def name(self) -> str:
        return self._name

    def alias(self) -> Optional[str]:
        return None

    def run(self, args) -> None:
        cwd = pathlib.Path.cwd()
        git_root = cwd / ".git"
        assert git_root.is_dir(), "The $CWD/$PWD must be git repo root!"

        # First publish "rsvim_macro" as it is used by other crates.
        cmd = f"cargo release --package rsvim_macro {args.level}"
        if args.execute:
            cmd = f"{cmd} --execute --no-verify"
        run(cmd)

        # Then publish other crates.
        cmd = f"cargo release --package rsvim_core --package rsvim {args.level}"
        if args.execute:
            cmd = f"{cmd} --execute --no-verify"
        run(cmd)


# npm package
class NpmPackage(Cmd):
    def __init__(self, subparsers) -> None:
        self._name = "npmpack"

        self.npm_parser = subparsers.add_parser(
            self._name,
            help="npm bump and publish package",
        )
        self.npm_parser.add_argument(
            "-b", "--bump", choices=["major", "minor", "patch"]
        )

    def name(self) -> str:
        return self._name

    def alias(self) -> Optional[str]:
        return None

    def run(self, args) -> None:
        if args.bump is not None:
            self.bump(args.bump)

    def bump(self, level) -> None:
        cmd = f"npm version {level} --no-git-tag-version"
        run(cmd)


if __name__ == "__main__":
    logging.basicConfig(format="%(levelname)s: %(message)s", level=logging.INFO)

    parser = argparse.ArgumentParser(description="development toolkit")
    parser.add_argument(
        "-n",
        "--no-cache",
        action="store_true",
        help="disable sccache",
    )

    subparsers = parser.add_subparsers(dest="subcommand")

    commands = [
        Bench(subparsers),
        Build(subparsers),
        Clippy(subparsers),
        Document(subparsers),
        Format(subparsers),
        Miri(subparsers),
        NpmPackage(subparsers),
        Release(subparsers),
        Test(subparsers),
    ]

    parsed_args = parser.parse_args()
    logging.info(f"Args {parsed_args}")

    if parsed_args.no_cache:
        NO_CACHE = True

    for command in commands:
        sub = parsed_args.subcommand
        if sub is not None and (command.name() == sub or command.alias() == sub):
            command.run(parsed_args)
            exit(0)

    logging.error("missing arguments, use -h/--help for more details.")
