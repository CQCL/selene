"""
Provides MetricStore, a class that can be used to extract
metrics from the METRICS tag emitted by Selene.

This allows the user to extract statistics from each shot,
including:
    - The number of each type of gate requested by the user program
    - The number of each type of gate output from the runtime
    - The batching behaviour of the runtime
    - Custom metrics reported by the runtime plugin
    - Custom metrics reported by the error model plugin
    - Custom metrics reported by the simulator plugin
"""

from .event_hook import EventHook


class MetricStore(EventHook):
    """
    A simple implementation of MetricsHandler that calls a user-provided
    callback when metrics are output
    """

    shots: list[dict[str, dict[str, float | int | bool | str]]] = []

    def get_selene_flags(self) -> list[str]:
        return ["provide_metrics"]

    def __init__(self):
        self.shots = []

    def try_invoke(self, tag: str, data: list):
        if not tag.startswith("METRICS:"):
            return False
        stripped_tag = ":".join(tag.split(":")[2:])
        category = "DEFAULT"
        if ":" in stripped_tag:
            split = stripped_tag.split(":")
            category = split[0]
            stripped_tag = ":".join(split[1:])
        assert len(data) == 1, (
            f"metric data must be a single element, got {len(data)} elements for {tag}"
        )
        # Store the metric in the latest shot's metrics dictionary
        self.shots[-1].setdefault(category, dict())[stripped_tag] = data[0]

    def on_new_shot(self):
        self.shots.append(dict())
