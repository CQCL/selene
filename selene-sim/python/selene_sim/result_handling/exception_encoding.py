from pathlib import Path
from typing import Iterator
from selene_sim.exceptions import (
    SelenePanicError,
    SeleneRuntimeError,
    SeleneStartupError,
    SeleneTimeoutError,
)
from . import TaggedResult


# when encoding exceptions through the result stream,
# these prefixes are used to identify metadata surrounding
# the error.
EXCEPTION_TYPE_PREFIX = "_EXCEPTION:INT:"
STDERR_PREFIX = "_STDERR:INT:"
STDOUT_PREFIX = "_STDOUT:INT:"


def encode_exception(
    exception: Exception, stdout_file: Path, stderr_file: Path
) -> Iterator[TaggedResult]:
    """
    Given an exception that occurs during a shot, encode it as a series
    of result stream entries with specific prefixes, so that it can be
    recovered on the other end of the stream.
    """
    match exception:
        case SelenePanicError(message=message, code=code):
            # The EXIT:INT: prefix is already present
            # in the message. The code is provided by
            # the user program or Selene.
            yield (message, code)
            yield (f"{EXCEPTION_TYPE_PREFIX}SelenePanicError", 0)
            yield (f"{STDERR_PREFIX}{stderr_file}", 0)
            yield (f"{STDOUT_PREFIX}{stdout_file}", 0)
        case SeleneRuntimeError(message=message):
            # We need to encode the EXIT:INT: prefix here,
            # and provide a generic code.
            yield (f"EXIT:INT:{message}", 110000)
            yield (f"{EXCEPTION_TYPE_PREFIX}SeleneRuntimeError", 0)
            yield (f"{STDERR_PREFIX}{stderr_file}", 0)
            yield (f"{STDOUT_PREFIX}{stdout_file}", 0)
        case SeleneStartupError(message=message):
            # We need to encode the EXIT:INT: prefix here,
            # and provide a generic code.
            yield (f"EXIT:INT:{message}", 110001)
            yield (f"{EXCEPTION_TYPE_PREFIX}SeleneStartupError", 0)
            yield (f"{STDERR_PREFIX}{stderr_file}", 0)
            yield (f"{STDOUT_PREFIX}{stdout_file}", 0)
        case SeleneTimeoutError(message=message):
            # We need to encode the EXIT:INT: prefix here,
            # and provide a generic code.
            yield (f"EXIT:INT:{message}", 110002)
            yield (f"{EXCEPTION_TYPE_PREFIX}SeleneTimeoutError", 0)
            yield (f"{STDERR_PREFIX}{stderr_file}", 0)
            yield (f"{STDOUT_PREFIX}{stdout_file}", 0)
        case other:
            # Encapsulate any other exception into a
            # SeleneRuntimeError for consistent parsing
            # on the other end.
            yield (f"EXIT:INT:{other}", 110000)
            yield ("{EXCEPTION_TYPE_PREFIX}SeleneRuntimeError", 0)
            yield (f"{STDERR_PREFIX}{stderr_file}", 0)
            yield (f"{STDOUT_PREFIX}{stdout_file}", 0)


def decode_exception(shot_results: list[TaggedResult]) -> Exception | None:
    """
    Given a list of shot results, check if the last four entries correspond
    to an encoded exception. If so, decode it and return the exception object.
    If not, return None.
    """

    # If the last 4 tags are EXIT:INT: EXCEPTION_TYPE_PREFIX, STDERR_PREFIX, and
    # STDOUT_PREFIX, then we have an error to process. Otherwise return None
    expected_prefixes = [
        "EXIT:INT:",
        EXCEPTION_TYPE_PREFIX,
        STDERR_PREFIX,
        STDOUT_PREFIX,
    ]
    if len(shot_results) < 4:
        return None

    last_four_tags = list(map(lambda x: x[0], shot_results[-4:]))
    if not all(last_four_tags[i].startswith(expected_prefixes[i]) for i in range(4)):
        return None

    error_message = last_four_tags[0].removeprefix("EXIT:INT:")
    error_code = shot_results[-4][1]
    exception_type = last_four_tags[1].removeprefix(EXCEPTION_TYPE_PREFIX)
    stderr_path = last_four_tags[2].removeprefix(STDERR_PREFIX)
    stdout_path = last_four_tags[3].removeprefix(STDOUT_PREFIX)

    # Satisfy Mypy that the variables are not None
    assert isinstance(error_message, str)
    assert isinstance(error_code, int)
    assert isinstance(exception_type, str)
    assert isinstance(stderr_path, str)
    assert isinstance(stdout_path, str)

    match exception_type:
        case "SelenePanicError":
            return SelenePanicError(
                message=error_message,
                code=error_code,
                stdout=Path(stdout_path).read_text(),
                stderr=Path(stderr_path).read_text(),
            )
        case "SeleneRuntimeError":
            return SeleneRuntimeError(
                message=error_message,
                stdout=Path(stdout_path).read_text(),
                stderr=Path(stderr_path).read_text(),
            )
        case "SeleneStartupError":
            return SeleneStartupError(
                message=error_message,
                stdout=Path(stdout_path).read_text(),
                stderr=Path(stderr_path).read_text(),
            )
        case "SeleneTimeoutError":
            return SeleneTimeoutError(
                message=error_message,
                stdout=Path(stdout_path).read_text(),
                stderr=Path(stderr_path).read_text(),
            )
        case _:
            raise RuntimeError(f"Unknown exception type: {exception_type}")
