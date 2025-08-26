from dataclasses import dataclass
from datetime import timedelta
import time


@dataclass
class Timeout:
    """
    A configuration struct for describing timeouts for various aspects
    of a simulation run.

    Where parameters are None, that timeout does not apply - but others
    can take precedence. For instance, if `overall` is set and `per_shot`
    is not, then a shot will still be limited by the overall timeout.

    Attributes:
        backend_startup: The maximum amount of time to wait for a Selene
                         backend process to successfully connect to the
                         frontend TCP server after being started.
        per_result: The maximum amount of time to wait on a results stream
                    read within a shot. This is to be used where results are
                    expected at relatively regular intervals, such as a loop
                    within the user program, where a long pause between results
                    may indicate a problem.
        per_shot: The maximum amount of time to allow a single shot to run.
        overall: The maximum amount of time to allow run_shots to complete.
    """

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
    """
    A helper class for use in tracking the progress of a timer.

    Times are stored as float timestamps from time.perf_counter(),
    which offers the highest available resolution timer on the platform.

    Attributes:
        start_timestamp: The time that the timer was started or last reset.
        duration_seconds: The duration of the timer in seconds, or None
                          for an infinite timer.
        end_timestamp: The time that the timer will expire, or None for
                       an infinite timer.
    """

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
        """
        Returns the number of seconds that have elapsed since the timer
        was started or last reset.
        """
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
        """
        Returns True if the timer has expired, or False if it is still
        running.
        """
        remaining_seconds = self.remaining_seconds()
        if remaining_seconds is None:
            return False
        return remaining_seconds <= 0

    def reset(self):
        """
        Reset the start_timestamp to the current time, and if the duration
        of the timer is not None, set the end_timestamp accordingly.
        """
        self.start_timestamp = time.perf_counter()
        if self.duration_seconds is not None:
            self.end_timestamp = self.start_timestamp + self.duration_seconds

    @staticmethod
    def min_remaining_seconds(timers: list["Timer"]) -> float | None:
        """
        A helper function for returning the minimum remaining time
        from a list of Timer objects, or None if all timers are infinite.

        This is useful when using multiple timers to track different aspects
        of a process, where the shortest remaining time is most relevant.

        Avoids separate calls to time.perf_counter() for each timer.
        """
        result: float | None = None
        current_time = time.perf_counter()
        for timer in timers:
            if timer.end_timestamp is None:
                continue
            remaining = timer.end_timestamp - current_time
            if result is None or remaining < result:
                result = remaining
        return result
