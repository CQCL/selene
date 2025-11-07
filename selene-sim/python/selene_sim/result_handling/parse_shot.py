from typing import Iterator, Iterable

from ..process import SeleneProcess
from ..event_hooks import EventHook
from ..exceptions import (
    SelenePanicError,
    SeleneRuntimeError,
    SeleneStartupError,
    SeleneTimeoutError,
)
from .result_stream import ResultStream, TaggedResult
from .exception_encoding import (
    encode_exception,
    extract_exception_from_results,
)
from .extract_shot import (
    extract_shot,
    ShotEntry,
    UserResult,
    UserStateResult,
    ShotExitMessage,
    MetricValue,
    InstructionLogEntry,
)


def parse_shot(
    stream: ResultStream,
    event_hook: EventHook,
    full: bool,
    process: SeleneProcess,
) -> Iterator[TaggedResult]:
    """
    Parses a shot from the results stream, yielding tagged results one by one.
    If `full` is True, the results are parsed and provide a pythonic interface
    (e.g. with exceptions upon different kinds of errors).

    If `full` is False, the results are simply interpreted, and exceptions are
    encoded as special entries in the results stream. This is handy if processing
    of the results is desired even in the presence of errors.
    """
    shot_entries = extract_shot(stream)
    if full:
        return parsed_interface(shot_entries, event_hook, stream, process)
    else:
        return unparsed_interface(shot_entries, stream, process)


# There are two modes of outputting shots. One is "parsed", another is "unparsed",
# the former targeting users who are interacting with a local selene instance, and
# the latter targetting services which aim to provide selene functionality remotely.
def parsed_interface(
    shot_entries: Iterable[ShotEntry],
    event_hook: EventHook,
    stream: ResultStream,
    process: SeleneProcess,
) -> Iterator[TaggedResult]:
    """
    Filters the shot entries for tagged results within one shot, stripping them
    of system prefixes and yielding them one by one.

    Standard shot exits are passed through as results for visibility, while
    panics are provided as SelenePanicErrors. Other exceptions that emerge are
    also raised, with contextual information via the stdout and stderr
    corresponding to the process that is feeding the results stream.
    """
    try:
        for entry in shot_entries:
            match entry:
                case UserResult(tag=tag, value=value):
                    tag_split = tag.split(":", maxsplit=2)
                    if len(tag_split) != 3:
                        raise SeleneRuntimeError(
                            f"Expected user result tag to have three parts, got {tag}"
                        )
                    yield ((tag_split[2], value))
                case UserStateResult(tag=tag, path=path):
                    tag_split = tag.split(":", maxsplit=1)
                    if len(tag_split) != 2:
                        raise SeleneRuntimeError(
                            f"Expected user state result tag to have two parts, got {tag}"
                        )
                    # TODO: this is a string, but TaggedResult currently expects an int,
                    # float, bool, or a list of those.
                    yield ((tag_split[1], path))  # type: ignore
                case ShotExitMessage(message=message, code=code):
                    message_split = message.split(":", maxsplit=2)
                    if len(message_split) != 3:
                        raise SeleneRuntimeError(
                            f"Expected exit message tag to have three parts, got {message}"
                        )
                    yield ((f"exit: {message_split[2]}", code))
                case MetricValue(name=name, value=value):
                    event_hook.try_invoke(name, [value])
                case InstructionLogEntry(tag=tag, values=values) as entry:
                    event_hook.try_invoke(tag, values)
    except Exception as error:
        # taint the stream to prevent further reading
        stream.taint()

        # if the error is not a selene exception, wrap it in a selene-specific runtime error
        if not isinstance(
            error,
            (
                SelenePanicError,
                SeleneRuntimeError,
                SeleneStartupError,
                SeleneTimeoutError,
            ),
        ):
            error = SeleneRuntimeError(message=str(error))
        if error.message.startswith("EXIT:INT:"):
            error.message = error.message[len("EXIT:INT:") :]

        # attach stdout and stderr to the exception
        process.terminate(
            expected_natural_exit=isinstance(
                error, (SeleneStartupError, SelenePanicError)
            )
        )
        error.stdout = process.stdout.read_text()
        error.stderr = process.stderr.read_text()
        raise error from None


# This isn't always suitable for remote services, which may want to inspect tags,
# rely on exception-free operation, and may wish to receive metadata (that would
# otherwise be handled by event hooks) directly as tagged information. This is
# provided through `generate_unparsed_shots`. Users of this may then choose to
# use `postprocess_unparsed_stream` to extract a (shots, error) tuple from the
# unparsed stream.


# Unparsed interface
def unparsed_interface(
    shot_entries: Iterator[ShotEntry],
    stream: ResultStream,
    process: SeleneProcess,
) -> Iterator[TaggedResult]:
    """
    Filters the shot entries for tagged results within one shot, yielding them
    one by one, without stripping tags or raising exceptions. When exceptions
    are encountered, they are caught and encoded with special tags such that
    they can be decoded further down the line (e.g. with postprocess_unparsed_stream).
    """
    try:
        for entry in shot_entries:
            match entry:
                case UserResult(tag=tag, value=value):
                    yield ((tag, value))
                case UserStateResult():
                    raise SeleneRuntimeError(
                        "User state results are not compatible with selene's unparsed interface"
                    )
                case ShotExitMessage(message=message, code=code):
                    yield ((message, code))
                case MetricValue(name=name, value=value):
                    yield ((name, value))
                case InstructionLogEntry():
                    raise SeleneRuntimeError(
                        "Instruction log entries are not compatible with selene's unparsed interface"
                    )
    except Exception as e:
        # taint the stream to prevent further reading
        stream.taint()
        process.terminate(
            expected_natural_exit=isinstance(e, (SelenePanicError, SeleneStartupError))
        )
        process.wait(check_return_code=False)
        # encode the exception as tagged results
        yield from encode_exception(e, process.stdout, process.stderr)


# Post-processing for unparsed streams
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
        filtered_shot, exception = extract_exception_from_results(shot)
        if exception is None:
            results.append(filtered_shot)
        else:
            if isinstance(exception, SelenePanicError):
                filtered_shot.append((f"EXIT:INT:{exception.message}", exception.code))
            if filtered_shot:
                results.append(filtered_shot)
            return results, exception
    return results, None
