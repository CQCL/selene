"""
Defines the abstract base class EventHook
"""

from abc import ABC, abstractmethod


class EventHook(ABC):
    """
    A base class for configuring events to be sent by Selene, detecting
    related events in the results stream, and handling them.
    """

    @abstractmethod
    def get_selene_flags(self) -> list[str]:
        """
        Define any arguments to the Selene executable that are required
        to enable the output of information pertinent to this EventHook.

        For example, if Selene has a flag `--provide-foo` that enables
        emitting [('FOO:VALUE', value)] events to the result stream,
        this method should return ['--provide-foo'].
        """
        pass

    @abstractmethod
    def try_invoke(self, tag: str, data: list) -> bool:
        """
        Check the tag to see if it is relevant to this EventHook. If so,
        parse the data and return True.
        """
        pass

    @abstractmethod
    def on_new_shot(self) -> None:
        """
        If this EventHook stores information on a shot-by-shot basis,
        implement this method to handle the creation of a new shot.
        """
        pass


class NoEventHook(EventHook):
    """
    A dummy EventHook that does nothing.
    """

    def get_selene_flags(self) -> list[str]:
        return []

    def try_invoke(self, tag: str, data: list) -> bool:
        return False

    def on_new_shot(self) -> None:
        pass


class MultiEventHook(EventHook):
    """
    A class that encapsulates multiple EventHooks, allowing them to be
    treated as a single EventHook. When try_invoke is called, it will
    iterate through all the EventHooks, calling try_invoke on each one.

    If short_circuit is set to True, the first EventHook that returns True
    will cause the MultiEventHook to return True, without calling any further
    EventHooks. If short_circuit is set to False, all EventHooks will be
    called, and the MultiEventHook will return True if any of the EventHooks
    return True.

    If no EventHooks return True, the MultiEventHook will return False.
    """

    event_hooks: list[EventHook]
    short_circuit: bool

    def __init__(
        self, event_hooks: list[EventHook] | None = None, short_circuit: bool = True
    ):
        self.event_hooks = event_hooks if event_hooks is not None else []
        self.short_circuit = short_circuit

    def add_event_hook(self, hook: EventHook):
        self.event_hooks.append(hook)

    def set_short_circuit(self, short_circuit: bool):
        self.short_circuit = short_circuit

    def get_selene_flags(self) -> list[str]:
        args = set()
        for hook in self.event_hooks:
            args.update(hook.get_selene_flags())
        return list(args)

    def try_invoke(self, tag: str, data: list) -> bool:
        success = False
        for hook in self.event_hooks:
            if hook.try_invoke(tag, data):
                if self.short_circuit:
                    return True
                success = True
        return success

    def on_new_shot(self) -> None:
        for hook in self.event_hooks:
            hook.on_new_shot()
