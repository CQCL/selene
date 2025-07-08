import subprocess
import sys
from hatchling.builders.hooks.plugin.interface import BuildHookInterface
from pathlib import Path
import tempfile


class ExampleHeaders:
    def __init__(self, hook: "SeleneCoreBuildHook") -> None:
        self.hook = hook

    def run(self):
        self.generate_core_header()
        self.generate_header_from_example("error_model")
        self.generate_header_from_example("simulator")
        self.generate_header_from_example("runtime")

    def generate_core_header(self):
        p = subprocess.Popen(
            [
                "cbindgen",
                "--config",
                "cbindgen.toml",
                "--output",
                f"{self.hook.root}/python/selene_core/_dist/include/selene/core_types.h",
            ],
            cwd=self.hook.root,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        for line in p.stdout:
            self.hook.app.display_info(line)
        p.wait()
        if p.returncode != 0:
            self.hook.app.display_error(
                f"cbindgen failed with return code {p.returncode}"
            )
            sys.exit(1)
        self.hook.app.display_success("core_types cbindgen completed successfully")

    def generate_header_from_example(self, what):
        self.hook.app.display_mini_header(f"Expanding {what} code")
        p = subprocess.Popen(
            [
                "cargo",
                "expand",
            ],
            cwd=Path(self.hook.root) / "examples" / what,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        stdout, stderr = p.communicate()
        if p.returncode != 0:
            self.hook.app.display_error(
                f"cargo expand failed with return code {p.returncode}"
            )
            self.hook.app.display_error(stderr)
            sys.exit(1)
        temp_rs_file = Path(tempfile.gettempdir()) / "expanded.rs"
        with temp_rs_file.open("w") as f:
            f.write(stdout)
        p = subprocess.Popen(
            [
                "cbindgen",
                "--config",
                "../cbindgen.toml",
                "--lang",
                "C",
                "-o",
                f"{self.hook.root}/python/selene_core/_dist/include/selene/{what}.h",
                f"{temp_rs_file}",
            ],
            cwd=Path(self.hook.root) / "examples" / what,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        for line in p.stdout:
            self.hook.app.display_info(line)
        p.wait()
        if p.returncode != 0:
            self.hook.app.display_error(
                f"cbindgen failed with return code {p.returncode}"
            )
            sys.exit(1)
        self.hook.app.display_success(f"{what} cbindgen completed successfully")


class SeleneCoreBuildHook(BuildHookInterface):
    def initialize(self, version: str, build_data: dict) -> None:
        header_extractor = ExampleHeaders(self)
        header_extractor.run()

        artifacts = []
        dist_dir = Path("python/selene_core/_dist")
        for artifact in dist_dir.rglob("*"):
            if artifact.is_file():
                artifacts.append(str(artifact.as_posix()))

        self.app.display_info("Found artifacts:")
        for a in artifacts:
            self.app.display_info(f"    {a}")

        build_data["artifacts"] += artifacts
