from dataclasses import dataclass
from datetime import timedelta
import time


@dataclass
class Timeout:
    backend_startup: timedelta | None = None
    per_result: timedelta | None = None
    per_shot: timedelta | None = None
    overall: timedelta | None = None

    @staticmethod
    def resolve_input(value: "TimeoutInput") -> "Timeout":
        if isinstance(value, Timeout):
            return value
        elif isinstance(value, timedelta):
            return Timeout(overall=value)
        elif isinstance(value, float):
            return Timeout(overall=timedelta(seconds=value))
        elif value is None:
            return Timeout()
        else:
            raise TypeError(f"Invalid type for Timeout: {type(value)}")


TimeoutInput = Timeout | timedelta | float | None


class Timer:
    start_timestamp: float
    # if None, the duration is infinite
    duration_seconds: float | None
    # the start time of the timer, used for calculating elapsed time
    end_timestamp: float | None = None

    def __init__(self, duration: timedelta | float | None):
        if isinstance(duration, timedelta):
            self.duration_seconds = duration.total_seconds()
        elif isinstance(duration, float):
            self.duration_seconds = duration
        elif duration is None:
            self.duration_seconds = None
        else:
            raise TypeError(f"Invalid type for duration: {type(duration)}")
        self.reset()

    def elapsed_seconds(self) -> float:
        return time.perf_counter() - self.start_timestamp

    def remaining_seconds(self) -> float | None:
        """
        If the duration is not None, returns the seconds remaining until
        the timer expires - if the timer has expired, returns <= 0.

        If the timer has no end timestamp, returns None.
        """
        if self.end_timestamp is None:
            return None
        return self.end_timestamp - time.perf_counter()

    def elapsed(self) -> timedelta:
        return timedelta(seconds=self.elapsed_seconds())

    def remaining(self) -> timedelta | None:
        """
        If the duration is not None, returns the timedelta remaining until
        the timer expires - if it has expired, the timedelta will be negative
        or zero.

        If the timer has no end timestamp, returns None.
        """
        remaining_seconds = self.remaining_seconds()
        if remaining_seconds is None:
            return None
        return timedelta(seconds=remaining_seconds)

    def has_expired(self) -> bool:
        remaining_seconds = self.remaining_seconds()
        if remaining_seconds is None:
            return False
        return remaining_seconds <= 0

    def reset(self):
        self.start_timestamp = time.perf_counter()
        if self.duration_seconds is not None:
            self.end_timestamp = self.start_timestamp + self.duration_seconds

    @staticmethod
    def min_remaining_seconds(timers: list["Timer"]) -> float | None:
        result: float | None = None
        for timer in timers:
            remaining = timer.remaining_seconds()
            if remaining is None:
                continue
            if result is None or remaining < result:
                result = remaining
        return result
