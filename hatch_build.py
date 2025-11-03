import subprocess
import platform
import shutil
import sys
from hatchling.builders.hooks.plugin.interface import BuildHookInterface
from pathlib import Path
import json


class CargoWorkspaceBuild:
    def __init__(self, hook: "BundleBuildHook") -> None:
        self.hook = hook
        self.metadata = self._get_metadata()

    def run(self):
        self.build_all()
        self.extract_libs()

    def _get_metadata(self):
        call = subprocess.run(
            [
                "cargo",
                "metadata",
                "--no-deps",
            ],
            cwd=self.hook.root,
            check=True,
            capture_output=True,
        )
        return json.loads(call.stdout)

    def build_all(self):
        self.hook.app.display_mini_header("Building cargo workspace")
        p = subprocess.Popen(
            [
                "cargo",
                "build",
                "--workspace",
                "--release",
                "--locked",
            ],
            cwd=self.hook.root,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        for line in p.stdout:
            line = line.decode("utf-8").strip()
            if "Finished" in line:
                self.hook.app.display_info(line)
            elif "error" in line:
                self.hook.app.display_error(line)
            else:
                self.hook.app.display_info(line)
        p.wait()
        if p.returncode != 0:
            self.hook.app.display_error(
                f"Cargo build failed with return code {p.returncode}"
            )
            sys.exit(1)
        self.hook.app.display_success("Cargo build completed successfully")

    def extract_libs(self):
        release_dir = Path(self.hook.root) / "target/release"
        assert release_dir.exists()
        for package in self.metadata["packages"]:
            for target in package["targets"]:
                if "cdylib" not in target["kind"]:
                    continue
                self.hook.app.display_info(f"Found cdylib target: {target['name']}")
                lib_name = target["name"]
                lib_filenames = {
                    "darwin": [f"lib{lib_name}.dylib"],
                    "linux": [f"lib{lib_name}.so"],
                    "win32": [f"{lib_name}.dll", f"{lib_name}.dll.lib"],
                }[sys.platform]
                assert all((release_dir / file).exists() for file in lib_filenames), (
                    f"Compiled library for {lib_name} not found in {release_dir}. "
                    f"Expected to find {','.join(lib_filenames)}."
                )
                self.hook.app.display_info(f"Found {lib_name} in {release_dir}")

                cargo_toml_path = Path(package["manifest_path"])
                python_path = cargo_toml_path.parent / "python"
                subdirs = filter(lambda x: x.is_dir(), python_path.iterdir())
                subdirs = filter(
                    lambda x: x.name != "test" and x.name != "tests", subdirs
                )
                subdirs = filter(lambda x: "pycache" not in x.name, subdirs)
                subdirs = filter(lambda x: not x.name.endswith("egg-info"), subdirs)
                subdirs = filter(lambda x: not x.name.startswith("."), subdirs)
                subdirs_list = list(subdirs)
                assert len(subdirs_list) == 1, (
                    f"Multiple python directories found in {python_path} - "
                    " hatch_build.py can't tell where the build artifacts should go."
                )
                subdir = subdirs_list[0]
                destination = subdir / "_dist/lib"
                destination.mkdir(parents=True, exist_ok=True)
                for filename in lib_filenames:
                    lib_path = release_dir / filename
                    if lib_path.exists():
                        self.hook.app.display_info(
                            f"Copying {lib_path} to {destination}"
                        )
                        shutil.copy(lib_path, destination)


class BundleBuildHook(BuildHookInterface):
    def initialize(self, version: str, build_data: dict) -> None:
        cargo_runner = CargoWorkspaceBuild(self)
        cargo_runner.run()
        self.build_selene_c_interface()
        self.build_helios_qis()

        packages = [Path("selene-sim/python/selene_sim")]
        for topic_dir in Path("selene-ext").iterdir():
            if topic_dir.name.startswith("."):
                continue
            if not topic_dir.is_dir():
                continue
            for package in topic_dir.iterdir():
                if package.name.startswith("."):
                    continue
                if not package.is_dir():
                    continue
                python_dir = package / "python"
                if not python_dir.exists():
                    continue
                subdirs = filter(lambda x: x.is_dir(), python_dir.iterdir())
                subdirs = filter(
                    lambda x: x.name != "test" and x.name != "tests", subdirs
                )
                subdirs = filter(lambda x: "pycache" not in x.name, subdirs)
                subdirs = filter(lambda x: not x.name.endswith("egg-info"), subdirs)
                subdirs = filter(lambda x: not x.name.startswith("."), subdirs)
                subdirs_list = list(subdirs)
                if len(subdirs_list) == 0:
                    continue
                if len(subdirs_list) > 1:
                    self.app.display_error(
                        f"Multiple python directories found in {python_dir} - "
                        "hatch_build.py can't tell which one to use."
                    )
                    sys.exit(1)
                package = subdirs_list[0]
                packages.append(package)
                self.app.display_info(f"Found package: {package}")

        artifacts = []
        for package in packages:
            package_root = Path(package)
            # add py.typed
            py_typed = package_root / "py.typed"
            if py_typed.exists():
                artifacts.append(str(py_typed.as_posix()))
            # add distribution files (e.g. dynamic libraries, headers)
            dist_dir = Path(package) / "_dist"
            if not dist_dir.exists():
                pass
            for artifact in dist_dir.rglob("*"):
                if artifact.is_file():
                    artifacts.append(str(artifact.as_posix()))

        self.app.display_info("Found artifacts:")
        for a in artifacts:
            self.app.display_info(f"    {a}")

        build_data["packages"] = packages
        build_data["artifacts"] += artifacts
        # Set platform-specific wheel tags
        if sys.platform == "darwin":
            arch = platform.machine()
            if arch == "arm64":
                build_data["tag"] = "py3-none-macosx_13_0_arm64"
            else:
                build_data["tag"] = "py3-none-macosx_13_0_x86_64"
        elif sys.platform == "linux":
            build_data["tag"] = "py3-none-manylinux_2_28_x86_64"
        elif sys.platform == "win32":
            build_data["tag"] = "py3-none-win_amd64"
        else:
            # Fallback to generic tag
            build_data["tag"] = "py3-none-any"

    def find_release_files(self, cdylib_name):
        release_dir = Path(self.root) / "target/release"
        if sys.platform == "darwin":
            return [release_dir / f"lib{cdylib_name}.dylib"]
        elif sys.platform == "linux":
            return [release_dir / f"lib{cdylib_name}.so"]
        elif sys.platform == "win32":
            return [
                release_dir / f"{cdylib_name}.dll",
                release_dir / f"{cdylib_name}.dll.lib",
            ]

    def build_cargo_workspace(self):
        self.app.display_mini_header("Building cargo workspace")
        try:
            subprocess.run(
                [
                    "cargo",
                    "build",
                    "--workspace",
                    "--release",
                    "--locked",
                ],
                cwd=self.root,
                check=True,
                capture_output=True,
            )
        except subprocess.CalledProcessError as e:
            self.app.display_error(f"Cargo build failed: {e.stderr.decode()}")
            sys.exit(1)
        self.app.display_success("Cargo build completed successfully")

    def distribute_cargo_artifacts(self):
        self.app.display_waiting("Copying artifacts")
        lib_paths = []
        for lib in self.find_release_files("selene"):
            self.app.display_info(f"Copying {lib} to {self.install_lib_dir}")
            shutil.copy(lib, self.install_lib_dir)
            lib_paths.append(lib)
        for lib in self.find_release_files("helios_selene_interface"):
            self.app.display_info(f"Copying {lib} to {self.install_lib_dir}")
            shutil.copy(lib, self.install_lib_dir)
            lib_paths.append(lib)

        return lib_paths

    def build_selene_c_interface(self):
        self.app.display_mini_header("Building Selene C interface")
        selene_sim_dir = Path(self.root) / "selene-sim"
        dist_dir = selene_sim_dir / "python/selene_sim/_dist"
        dist_dir.mkdir(parents=True, exist_ok=True)

        try:
            subprocess.run(
                [
                    "cbindgen",
                    "-o",
                    "c/include/selene/selene.h",
                ],
                check=True,
                cwd=selene_sim_dir,
                capture_output=True,
            )
        except subprocess.CalledProcessError as e:
            self.app.display_error(f"cbindgen failed: {e.stderr.decode()}")
            sys.exit(1)

        cmake_build_dir = selene_sim_dir / "c/build"
        cmake_build_dir.mkdir(parents=True, exist_ok=True)

        try:
            subprocess.run(
                ["cmake", f"-DCMAKE_INSTALL_PREFIX={dist_dir}", ".."],
                cwd=cmake_build_dir,
                check=True,
                capture_output=True,
            )
        except subprocess.CalledProcessError as e:
            if b"is different than the directory" not in e.stderr:
                self.app.display_error(f"cmake failed: {e.stderr.decode()}")
                sys.exit(1)
            try:
                # existing build dir is incompatible, delete and retry
                shutil.rmtree(cmake_build_dir)
                cmake_build_dir.mkdir()
                subprocess.run(
                    [
                        "cmake",
                        f"-DCMAKE_INSTALL_PREFIX={dist_dir}",
                        "-DCMAKE_BUILD_TYPE=Release",
                        "..",
                    ],
                    cwd=cmake_build_dir,
                    check=True,
                    capture_output=True,
                )
            except subprocess.CalledProcessError as e:
                self.app.display_error(f"cmake failed: {e.stderr.decode()}")
                sys.exit(1)

        try:
            subprocess.run(
                [
                    "cmake",
                    "--build",
                    ".",
                    "--target",
                    "install",
                ],
                cwd=cmake_build_dir,
                check=True,
                capture_output=True,
            )
        except subprocess.CalledProcessError as e:
            self.app.display_error(f"cmake build failed: {e.stderr.decode()}")
            sys.exit(1)

        self.app.display_success("C interface build completed successfully")

    def build_helios_qis(self):
        self.app.display_mini_header("Building Helios QIS")
        helios_qis_dir = Path(self.root) / "selene-ext/interfaces/helios_qis"
        cmake_build_dir = helios_qis_dir / "c/build"
        cmake_build_dir.mkdir(parents=True, exist_ok=True)
        dist_dir = helios_qis_dir / "python/selene_helios_qis_plugin/_dist"
        dist_dir.mkdir(parents=True, exist_ok=True)
        selene_sim_dist_dir = Path(self.root) / "selene-sim/python/selene_sim/_dist"

        try:
            subprocess.run(
                [
                    "cmake",
                    f"-DCMAKE_INSTALL_PREFIX={dist_dir}",
                    "-DCMAKE_BUILD_TYPE=Release",
                    f"-DCMAKE_PREFIX_PATH={selene_sim_dist_dir}",
                    "..",
                ],
                cwd=cmake_build_dir,
                check=True,
                capture_output=True,
            )
        except subprocess.CalledProcessError as e:
            if b"is different than the directory" not in e.stderr:
                self.app.display_error(f"cmake failed: {e.stderr.decode()}")
                sys.exit(1)
            try:
                # existing build dir is incompatible, delete and retry
                shutil.rmtree(cmake_build_dir)
                cmake_build_dir.mkdir()
                subprocess.run(
                    [
                        "cmake",
                        f"-DCMAKE_INSTALL_PREFIX={dist_dir}",
                        f"-DCMAKE_PREFIX_PATH={selene_sim_dist_dir}",
                        "..",
                    ],
                    cwd=cmake_build_dir,
                    check=True,
                    capture_output=True,
                )
            except subprocess.CalledProcessError as e:
                self.app.display_error(f"cmake failed: {e.stderr.decode()}")
                sys.exit(1)

        try:
            subprocess.run(
                [
                    "cmake",
                    "--build",
                    ".",
                    "--target",
                    "install",
                ],
                check=True,
                cwd=cmake_build_dir,
                capture_output=True,
            )
        except subprocess.CalledProcessError as e:
            self.app.display_error(f"cmake build failed: {e.stderr.decode()}")
            sys.exit(1)

        self.app.display_success("Helios QIS build completed successfully")
