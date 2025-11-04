from hatchling.builders.hooks.plugin.interface import BuildHookInterface
from pathlib import Path


class SeleneCoreBuildHook(BuildHookInterface):
    def initialize(self, version: str, build_data: dict) -> None:
        artifacts = []
        dist_dir = Path("python/selene_core/_dist")
        for artifact in dist_dir.rglob("*"):
            if artifact.is_file():
                artifacts.append(str(artifact.as_posix()))

        self.app.display_info("Found artifacts:")
        for a in artifacts:
            self.app.display_info(f"    {a}")

        build_data["artifacts"] += artifacts
