import platform
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Iterator, cast
from hugr.qsystem.result import TaggedResult
from selene_core import Simulator
from .state import SeleneQuestState


@dataclass
class QuestPlugin(Simulator):
    """
    A plugin for using QuEST, the statevector simulation engine,
    as the backend simulator for selene.
    """

    def __post_init__(self):
        pass

    @property
    def library_file(self):
        libdir = Path(__file__).parent / "_dist/lib/"
        match platform.system():
            case "Linux":
                return libdir / "libselene_quest_plugin.so"
            case "Darwin":
                return libdir / "libselene_quest_plugin.dylib"
            case "Windows":
                return libdir / "selene_quest_plugin.dll"
            case _:
                raise RuntimeError(f"Unsupported platform: {platform.system()}")

    def get_init_args(self):
        return []

    @staticmethod
    def extract_states_dict(
        results: Iterable[TaggedResult],
        cleanup: bool = True,
    ) -> dict[str, SeleneQuestState]:
        """Extract state results from a shot result stream and return them as a
        dictionary keyed by the state tag. Assumes tags are unique within the shot.

        By default, state files are removed after extraction, as they may take up
        considerable storage space. Pass `cleanup=False` to keep the files.
        """
        return dict(QuestPlugin.extract_states(results, cleanup=cleanup))

    @staticmethod
    def extract_states(
        results: Iterable[TaggedResult],
        cleanup: bool = True,
    ) -> Iterator[tuple[str, SeleneQuestState]]:
        """Extract state results from a shot result stream and return them as a
        pair of (tag, state).

        By default, state files are removed after extraction, as they may take up
        considerable storage space. Pass `cleanup=False` to keep the state files.
        """
        return (
            (
                cast(str, state_tag),
                SeleneQuestState.parse_from_file(pth, cleanup=cleanup),
            )
            for tag, result in results
            if (state_tag := _state_tag(tag)) is not None
            and isinstance(result, str)
            and (pth := Path(result)).is_file()
        )


def _state_tag(tag: str) -> str | None:
    """Strip prefix for state results if it is present and return the remainder."""
    prefix = "STATE:"
    if tag.startswith(prefix):
        return tag[len(prefix) :]
    return None
