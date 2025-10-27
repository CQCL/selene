from dataclasses import dataclass
from typing import Iterator, Iterable
from pathlib import Path

from ..event_hooks import EventHook
from ..exceptions import (
    SelenePanicError,
    SeleneRuntimeError,
    SeleneStartupError,
    SeleneTimeoutError,
)

from . import ResultStream, TaggedResult
from .exception_encoding import encode_exception, decode_exception


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


def parse_record_minimal(
    record: tuple,
) -> TaggedResult | ExitMessage | ShotBoundary:
    """
    When the user requests that results from the data stream are 'unparsed', it is
    an instruction to selene to avoid manipulation of the entries of the data itself.
    This allows them to perform their own parsing after the selene run.

    Nonetheless, we do require some minimal parsing of structural entries within
    the data stream, such as shot boundaries and exits, to maintain the normal operation
    of the selene frontend.
    """
    tag, *data = record
    assert isinstance(tag, str), f"tag must be a string, got {tag} of type {type(tag)}"
    if tag.endswith(":__SHOT_BOUNDARY__"):
        return ShotBoundary()

    if tag.startswith("EXIT:"):
        code = data[0]
        return ExitMessage(message=tag, code=code)

    return (tag, data[0])


def parse_record(
    record: tuple,
    event_hook: EventHook,
) -> TaggedResult | ExitMessage | ShotBoundary | None:
    """
    When the user requests that the results stream is parsed, we wish to intercept
    entries from the stream in order to understand their context, then provide them
    in a simple and friendly manner to the user.

    - Structural entries such as exits and shot boundaries are processed as normal,
      with the handling managed by the selene frontend for smooth operation.

    - Normal "USER:[TYPE]:[NAME]" tags, e.g. the I/O result of 'result(...)' calls
      in Guppy, are stripped of their namespace prefixes and their data elements are
      returned. Thus a user that makes a result call with a tag will receive that tag
      back without the intermediate namespacing added during compilation to guide
      preprocessing.

    - "STATE" tags are preserved, but stripped of their namespace prefix.

    - Other tags are passed to the event hook, which can handle them as needed. This
      currently includes metrics and circuit instructions.
    """
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


def parse_shot_minimal(
    parser: ResultStream,
    stdout_file: Path,
    stderr_file: Path,
) -> Iterator[TaggedResult]:
    """
    Filters the results stream for tagged results within one shot, yielding them
    them one by one.

    Each record from the stream is parsed through `parse_record_minimal`, which
    only processes structural entries such as exits and shot boundaries.

    If any exception occurs, it is caught and yielded as the final four entries
    of the resulting stream. No exception is raised to the caller. This flow allows
    the caller to process the results up to the point of failure, and then handle
    the error as they see fit.
    """
    try:
        for record in parser:
            parsed = parse_record_minimal(record)
            if parsed is None:
                pass
            if isinstance(parsed, tuple):  # TaggedResult is a tuple
                yield parsed
            elif isinstance(parsed, ExitMessage):
                if parsed.code >= 1000:
                    yield from encode_exception(
                        SelenePanicError(
                            message=parsed.message,
                            code=parsed.code,
                            stdout="",
                            stderr="",
                        ),
                        stdout_file,
                        stderr_file,
                    )
                else:
                    yield (parsed.message, parsed.code)
            elif isinstance(parsed, ShotBoundary):
                parser.next_shot()
                break

    # pass any errors through the results stream
    except Exception as e:
        yield from encode_exception(e, stdout_file, stderr_file)


def postprocess_unparsed_shot(
    shot_results: list[TaggedResult],
) -> tuple[list[TaggedResult], Exception | None]:
    """
    Decode any exception information from a shot's results, filtering out
    error-related tags if present. Returns the filtered results and, optionally,
    any exception that was decoded.
    """
    decoded_exception = decode_exception(shot_results)
    if decoded_exception is None:
        return shot_results, None
    else:
        # filter out error metadata except for the exit entry
        return shot_results[:-3], decoded_exception


def postprocess_unparsed_stream(
    shot_results: Iterable[Iterable[TaggedResult]],
) -> tuple[list[list[TaggedResult]], Exception | None]:
    """
    Post-processes a stream of unparsed shots, extracting errors and filtering
    out error-related tags. Returns a list of results for each shot, along with
    any exception that occurred during processing.
    """
    results = []
    for shot in shot_results:
        filtered_shot, exception = postprocess_unparsed_shot(list(shot))
        if exception is None:
            results.append(filtered_shot)
        else:
            if not isinstance(exception, SeleneStartupError):
                results.append(filtered_shot)
            return results, exception
    return results, None


def parse_shot_full(
    parser: ResultStream,
    event_hook: EventHook,
    stdout_file: Path,
    stderr_file: Path,
) -> Iterator[TaggedResult]:
    """
    Filters the results stream for tagged results within one shot, yielding them
    them one by one.

    Exits are handled directly and, if the error code is >= 1000, are raised as
    a SelenePanicError. Any other exceptions that emerge are also raised, with
    contextual information via the stdout and stderr corresponding to the process
    that is feeding the results stream.
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
    except SeleneTimeoutError as error:
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


def parse_shot(
    parser: ResultStream,
    event_hook: EventHook,
    full: bool,
    stdout_file: Path,
    stderr_file: Path,
) -> Iterator[TaggedResult]:
    """
    Parses a shot from the results stream, yielding tagged results one by one.
    If `full` is True, the results are parsed through `parse_record`.
    Otherwise, they are parsed through `parse_record_minimal`, which only processes
    structural entries such as exits and shot boundaries.
    """
    if full:
        yield from parse_shot_full(parser, event_hook, stdout_file, stderr_file)
    else:
        yield from parse_shot_minimal(parser, stdout_file, stderr_file)
