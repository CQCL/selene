from dataclasses import dataclass
from typing import Iterator
from enum import Enum

from ..exceptions import (
    SelenePanicError,
    SeleneRuntimeError,
)

from .result_stream import ResultStream, DataValue, StreamEntry, StreamEntryPart


@dataclass
class ShotExitMessage:
    """
    Signifies that the shot is exiting early with a given message and code,
    e.g. as a result of a user program error that applies only to the current
    shot. The current shot will terminate gracefully with a ShotEnd, possibly
    after some metrics, and the next shot (if any) will proceed as normal.
    """

    message: str
    code: int


@dataclass
class FullPanicMessage:
    """
    Signifies that a panic has occured, either in the user program (e.g. expecting
    an option type to have a value, implying a bug in the user code), or in a selene
    component (e.g. if non-cliffords are attempted to be simulated with a stabilizer
    simulator). This is similar to ShotExitMessage, but indicates that after shot
    wind-down, no further shots are expected.
    """

    message: str
    code: int


@dataclass
class ShotStart:
    """
    Represents the start of a shot in the results stream. User results for a given
    shot are not expected before this entry.
    """

    shot_id: int


@dataclass
class ShotEnd:
    """
    Represents the end of a shot in the results stream. User results for a given
    shot are not expected after this entry.
    """

    shot_id: int


@dataclass
class UserStateResult:
    """
    The user has requested a state result, which is represented as a tag alongside
    a path to a data file containing the state (as it is, in general, too large to
    pass through the results stream directly).

    Each simulator that supports a state result should provide a mechanism for
    interpreting the data in a state result file that it produces, and it is the
    user's responsibility to use it appropriately.
    """

    tag: str
    path: str


@dataclass
class UserResult:
    """
    A user result, represented in the data stream as a tag "USER:TYPE:NAME" alongside
    a data value. This entry results from a user program calling the `result` function,
    in guppylang.
    """

    tag: str
    value: DataValue


@dataclass
class MetricValue:
    """
    A metric value, represented in the data stream as a tag "METRICS:TYPE:NAME" alongside
    a numeric value (int, float, or bool). This entry is optionally provided at the end
    of a shot, e.g. if the MetricStore is provided as an event hook during simulation.
    """

    name: str
    value: bool | int | float


@dataclass
class InstructionLogEntry:
    """
    When a user has passed a CircuitExtractor event hook to the simulator, instruction
    log entries are generated during the shot that describe the instructions executed
    directly from the user's program, as well as those provided by the chosen Runtime
    plugin. This is particularly useful when debugging user programs, as it allows
    users to see exactly what instructions were executed, and in what order, from
    their program as well as from the runtime (which may reorder things).

    `values` is a list of generic StreamEntryPart, as instruction log entries may have
    arbitrary values depending on the instruction being logged. They are not confined
    to a single data value, for example: they can be very large lists of encoded
    information for the CircuitExtractor to interpret.
    """

    tag: str
    values: list[StreamEntryPart]


ShotEntry = (
    UserResult | UserStateResult | ShotExitMessage | MetricValue | InstructionLogEntry
)
ExtractedStreamEntry = ShotStart | ShotEnd | ShotEntry | FullPanicMessage


def extract_single_entry(entry: StreamEntry) -> ExtractedStreamEntry:
    """
    Given a single entry from the results stream, interpret its meaning according
    to its tag, validate where possible, and return the appropriate typed
    representation.
    """
    if entry.tag == "SELENE:SHOT_START":
        assert isinstance(entry.values[0], int), (
            f"Expected shot ID to be an integer, got {type(entry.values[0])}"
        )
        return ShotStart(shot_id=entry.values[0])
    elif entry.tag == "SELENE:SHOT_END":
        assert isinstance(entry.values[0], int), (
            f"Expected shot ID to be an integer, got {type(entry.values[0])}"
        )
        return ShotEnd(shot_id=entry.values[0])
    elif entry.tag.startswith("EXIT:"):
        code = entry.values[0]
        assert isinstance(code, int), (
            f"Expected exit code to be an integer, got {type(code)}"
        )
        if code >= 1000:
            return FullPanicMessage(message=entry.tag, code=code)
        else:
            return ShotExitMessage(message=entry.tag, code=code)
    elif entry.tag.startswith("USER:"):
        if entry.tag.startswith("USER:STATE:"):
            assert len(entry.values) == 1, (
                f"Expected single value for state result, got {type(entry.values)}"
            )
            path = entry.values[0]
            assert isinstance(path, str), (
                f"Expected state result path to be a string, got {type(path)}"
            )
            return UserStateResult(tag=entry.tag, path=path)
        else:
            assert len(entry.values) == 1
            value = entry.values[0]
            assert isinstance(value, (int, bool, float, list)), (
                f"Expected user result value to be a DataValue, got {type(value)}"
            )
            return UserResult(tag=entry.tag, value=value)
    elif entry.tag.startswith("METRICS:"):
        assert len(entry.values) == 1
        value = entry.values[0]
        assert isinstance(value, (int, bool, float)), (
            f"Expected metric value to be an integer, boolean or float, got {type(value)}"
        )
        return MetricValue(name=entry.tag, value=value)
    elif entry.tag == "INSTRUCTIONLOG":
        return InstructionLogEntry(tag=entry.tag, values=entry.values)
    else:
        raise SeleneRuntimeError(f"Unexpected entry in data stream: '{entry.tag}'")


class ShotStatus(Enum):
    NOT_STARTED = 0
    IN_PROGRESS = 1
    ENDING = 2
    COMPLETED = 3


def extract_shot(
    stream: ResultStream,
) -> Iterator[ShotEntry]:
    """
    Filters the results stream for tagged results within one shot, yielding them
    them one by one. A stateful approach is used to ensure that messages are expected
    in the correct order, raising exceptions if unexpected messages are encountered.
    """
    shot_status = ShotStatus.NOT_STARTED
    # We expect the first record to be a shot start (or an error)
    records = iter(stream)
    first_record = next(records)
    first_entry = extract_single_entry(first_record)
    match first_entry:
        case ShotStart(shot_id=reported_shot_id):
            shot_id = reported_shot_id
            shot_status = ShotStatus.IN_PROGRESS
        case FullPanicMessage(message=message, code=_):
            # Seeing panics before a shot start is unexpected, and we consider
            # it to either be a misconfiguration or a bug. As such, we raise it
            # as a runtime error.
            raise SeleneRuntimeError(message)
        case other:
            raise SeleneRuntimeError(f"Unexpected record {other} before shot start")

    # Now we process the rest of the shots.
    for record in records:
        match extract_single_entry(record):
            case ShotStart(shot_id=reported_shot_id):
                raise SeleneRuntimeError(
                    f"Received unexpected shot start for shot ID {reported_shot_id} while shot ID {shot_id} is in progress"
                )
            case ShotEnd(shot_id=reported_shot_id):
                if reported_shot_id == shot_id:
                    shot_status = ShotStatus.COMPLETED
                    break
                else:
                    raise SeleneRuntimeError(
                        f"Received shot end for shot ID {reported_shot_id} while shot ID {shot_id} is in progress"
                    )
            case ShotExitMessage(message=message, code=code) as exit_msg:
                shot_status = ShotStatus.ENDING
                yield exit_msg
            case FullPanicMessage(message=message, code=code):
                shot_status = ShotStatus.ENDING
                raise SelenePanicError(message=message, code=code)
            case other:
                # all other entries, such as user results, state results, metadata, etc
                # are expected strictly within shot boundaries.
                if shot_status not in [ShotStatus.IN_PROGRESS, ShotStatus.ENDING]:
                    raise SeleneRuntimeError(
                        f"User entry received outside of shot boundaries: {other}"
                    )
                yield other

    if shot_status == ShotStatus.COMPLETED:
        stream.next_shot()
    else:
        raise SeleneRuntimeError(f"Shot {shot_id} ended unexpectedly")
