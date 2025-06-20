from dataclasses import dataclass
from typing import Iterator
from pathlib import Path

from ..event_hooks import EventHook
from ..exceptions import SelenePanicError, SeleneRuntimeError, SeleneStartupError

from . import ResultStream, TaggedResult


@dataclass
class ExitMessage:
    message: str
    code: int

    def __repr__(self):
        return f"ExitMessage: {self.message} with code {self.code}"


class ShotBoundary:
    """
    Represents a shot boundary in the results stream. If this is received,
    handling of the current shot should end, and further results belong to
    the next shot.
    """

    pass


def parse_record(
    record: tuple,
    event_hook: EventHook,
) -> TaggedResult | ExitMessage | ShotBoundary | None:
    tag, *data = record
    assert isinstance(tag, str), f"tag must be a string, got {tag} of type {type(tag)}"
    if tag.endswith(":__SHOT_BOUNDARY__"):
        return ShotBoundary()

    if tag.startswith("L3:") or tag.startswith("USER:"):
        split = tag.split(":")
        # preserve state tag prefix
        tag_start = 1 if split[1] == "STATE" else 2

        stripped_tag = ":".join(split[tag_start:])
        assert len(data) == 1, (
            f"tag data must be a single element, got {len(data)} elementsfor {tag}"
        )
        return (stripped_tag, data[0])

    if tag.startswith("EXIT:"):
        stripped_message = ":".join(tag.split(":")[2:])
        # if the tag namespace gets duplicated, strip it out
        if any(
            stripped_message.startswith(x)
            for x in ["EXIT:INT:", "L3:INT:", "USER:INT:"]
        ):
            stripped_message = ":".join(stripped_message.split(":")[2:])
        code = data[0]
        return ExitMessage(message=stripped_message, code=code)

    if event_hook.try_invoke(tag, data):
        return None

    return None


def parse_shot(
    parser: ResultStream, event_hook: EventHook, stdout_file: Path, stderr_file: Path
) -> Iterator[TaggedResult]:
    """
    Filters the results stream for tagged results within one shot, yielding them
    them one by one.
    """
    try:
        for record in parser:
            parsed = parse_record(record, event_hook)
            if parsed is None:
                pass
            if isinstance(parsed, tuple):  # TaggedResult is a tuple
                yield parsed
            elif isinstance(parsed, ExitMessage):
                if parsed.code >= 1000:
                    raise SelenePanicError(
                        message=parsed.message,
                        code=parsed.code,
                        stdout=stdout_file.read_text(),
                        stderr=stderr_file.read_text(),
                    )
                else:
                    # temporary behaviour: yield it as a result
                    yield (f"exit: {parsed.message}", parsed.code)
            elif isinstance(parsed, ShotBoundary):
                parser.next_shot()
                break
    # pass panic errors to the caller
    except SelenePanicError as panic:
        raise panic
    except SeleneRuntimeError as error:
        error.stdout = stdout_file.read_text()
        error.stderr = stderr_file.read_text()
        raise error
    except SeleneStartupError as error:
        error.stdout = stdout_file.read_text()
        error.stderr = stderr_file.read_text()
        raise error
    # pass other errors as generic SeleneRuntimeErrors
    except Exception as e:
        raise SeleneRuntimeError(
            message=str(e),
            stdout=stdout_file.read_text(),
            stderr=stderr_file.read_text(),
        ) from e
