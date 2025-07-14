"""
Provides CircuitExtractor, a class that can be used to extract
instructions from the INSTRUCTIONLOG tag emitted by Selene.

This allows the user to extract the instructions requested by the
user program as a pytket.Circuit, and the batches of instructions
issued by the runtime as a list of dictionaries, on a shot-by-shot
basis.
"""

from abc import ABC, abstractmethod
from enum import Enum
from dataclasses import dataclass
from collections.abc import Iterator
from typing import Any
import math

from .event_hook import EventHook

PYTKET_AVAILABLE = False
try:
    import pytket

    PYTKET_AVAILABLE = True
except ImportError:
    pass


class Operation(ABC):
    @abstractmethod
    def append_to_circuit(self, circuit: "pytket.Circuit"):
        pass

    @abstractmethod
    def to_dict(self) -> dict:
        pass

    @staticmethod
    @abstractmethod
    def from_iterator(it: Iterator):
        pass


@dataclass
class BatchStart(Operation):
    start_time_ns: int
    duration_ns: int

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        pass

    def to_dict(self) -> dict:
        return {
            "op": "BatchStart",
            "start_time_ns": self.start_time_ns,
            "duration_ns": self.duration_ns,
        }

    @staticmethod
    def from_iterator(it: Iterator):
        start_time_ns = next(it)
        duration_ns = next(it)
        return BatchStart(start_time_ns=start_time_ns, duration_ns=duration_ns)


@dataclass
class CustomOperation(Operation):
    tag: int
    data: bytes

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        pass

    def to_dict(self) -> dict:
        return {"op": "CustomOperation", "tag": self.tag, "data": self.data}

    @staticmethod
    def from_iterator(it: Iterator):
        tag = next(it)
        data = bytes(next(it))
        return CustomOperation(tag=tag, data=data)


@dataclass
class LocalBarrier(Operation):
    qubits: list[int]
    sleep_time: int

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        circuit.add_barrier(self.qubits)

    def to_dict(self) -> dict:
        return {
            "op": "LocalBarrier",
            "qubits": self.qubits,
            "sleep_time": self.sleep_time,
        }

    @staticmethod
    def from_iterator(it: Iterator):
        qubits_len = next(it)
        qubits = []
        for _ in range(qubits_len):
            qubits.append(next(it))
        sleep_time = next(it)
        return LocalBarrier(qubits=qubits, sleep_time=sleep_time)


@dataclass
class GlobalBarrier(Operation):
    sleep_time: int

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        circuit.add_barrier(circuit.qubits)

    def to_dict(self) -> dict:
        return {"op": "GlobalBarrier", "sleep_time": self.sleep_time}

    @staticmethod
    def from_iterator(it: Iterator):
        sleep_time = next(it)
        return GlobalBarrier(sleep_time=sleep_time)


@dataclass
class QAlloc(Operation):
    qubit: int

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        assert PYTKET_AVAILABLE, "pytket is not available"
        if circuit.n_qubits <= self.qubit:
            circuit.add_qubit(pytket.Qubit(self.qubit))

    def to_dict(self) -> dict:
        return {"op": "QAlloc", "qubit": self.qubit}

    @staticmethod
    def from_iterator(it: Iterator):
        return QAlloc(qubit=next(it))


@dataclass
class QFree(Operation):
    qubit: int

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        assert PYTKET_AVAILABLE, "pytket is not available"

    def to_dict(self) -> dict:
        return {"op": "QFree", "qubit": self.qubit}

    @staticmethod
    def from_iterator(it: Iterator):
        return QFree(qubit=next(it))


@dataclass
class Rxy(Operation):
    qubit: int
    theta: float
    phi: float

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        assert PYTKET_AVAILABLE, "pytket is not available"
        circuit.PhasedX(
            angle0=self.theta / math.pi, angle1=self.phi / math.pi, qubit=self.qubit
        )

    def to_dict(self) -> dict:
        return {"op": "Rxy", "qubit": self.qubit, "theta": self.theta, "phi": self.phi}

    @staticmethod
    def from_iterator(it: Iterator):
        qubit = next(it)
        theta = next(it)
        phi = next(it)
        assert isinstance(qubit, int), (
            f"qubit must be an integer, got {qubit} of type {type(qubit)}"
        )
        assert isinstance(theta, float), (
            f"theta must be a float, got {theta} of type {type(theta)}"
        )
        assert isinstance(phi, float), (
            f"phi must be a float, got {phi} of type {type(phi)}"
        )
        return Rxy(qubit=qubit, theta=theta, phi=phi)


@dataclass
class Rzz(Operation):
    qubit0: int
    qubit1: int
    theta: float

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        assert PYTKET_AVAILABLE, "pytket is not available"
        circuit.ZZPhase(
            angle=self.theta / math.pi, qubit0=self.qubit0, qubit1=self.qubit1
        )

    def to_dict(self) -> dict:
        return {
            "op": "Rzz",
            "qubit0": self.qubit0,
            "qubit1": self.qubit1,
            "theta": self.theta,
        }

    @staticmethod
    def from_iterator(it: Iterator):
        qubit0 = next(it)
        qubit1 = next(it)
        theta = next(it)
        return Rzz(qubit0=qubit0, qubit1=qubit1, theta=theta)


@dataclass
class Rz(Operation):
    qubit: int
    theta: float

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        assert PYTKET_AVAILABLE, "pytket is not available"
        circuit.Rz(angle=self.theta / math.pi, qubit=self.qubit)

    def to_dict(self) -> dict:
        return {"op": "Rz", "qubit": self.qubit, "theta": self.theta}

    @staticmethod
    def from_iterator(it: Iterator):
        return Rz(qubit=next(it), theta=next(it))


@dataclass
class Reset(Operation):
    qubit: int

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        assert PYTKET_AVAILABLE, "pytket is not available"
        circuit.Reset(qubit=self.qubit)

    def to_dict(self) -> dict:
        return {"op": "Reset", "qubit": self.qubit}

    @staticmethod
    def from_iterator(it: Iterator):
        return Reset(qubit=next(it))


@dataclass
class MeasureRequest(Operation):
    qubit: int

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        assert PYTKET_AVAILABLE, "pytket is not available"
        for i in range(circuit.n_bits, self.qubit + 1):
            circuit.add_bit(pytket.Bit(i))
        circuit.Measure(qubit=self.qubit, bit=self.qubit)

    def to_dict(self) -> dict:
        return {"op": "MeasureRequest", "qubit": self.qubit}

    @staticmethod
    def from_iterator(it: Iterator):
        return MeasureRequest(qubit=next(it))


@dataclass
class MeasureLeakedRequest(Operation):
    qubit: int

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        assert PYTKET_AVAILABLE, "pytket is not available"
        for i in range(circuit.n_bits, self.qubit + 1):
            circuit.add_bit(pytket.Bit(i))
        circuit.Measure(qubit=self.qubit, bit=self.qubit)

    def to_dict(self) -> dict:
        return {"op": "MeasureLeakedRequest", "qubit": self.qubit}

    @staticmethod
    def from_iterator(it: Iterator):
        return MeasureLeakedRequest(qubit=next(it))


@dataclass
class FutureRead(Operation):
    qubit: int

    def append_to_circuit(self, circuit: "pytket.Circuit"):
        # only use on the request to prevent duplicate reads
        # becoming duplicate operations
        pass

    def to_dict(self) -> dict:
        return {"op": "FutureRead", "qubit": self.qubit}

    @staticmethod
    def from_iterator(it: Iterator):
        return FutureRead(qubit=next(it))


class Source(Enum):
    """
    Selene provides the source of each instruction as an
    integer index. This enum maps those indices to a more
    human-readable form.
    """

    USER = 0
    OPTIMISER = 1
    ERROR_MODEL = 2


@dataclass
class Instruction:
    source: Source
    operation: Operation

    @staticmethod
    def from_iterator(it: Iterator):
        """
        Extract a single instruction from an iterator of the
        data array provided by Selene, and advance the iterator.

        An instruction is of the form:

        ( source: u64 | operation: u64 | data: ... )

        where the length of the data depends on the operation.

        For example:
        - a QAlloc operation would have a data array of length 1,
          containing the qubit index to allocate.
        - An Rzz would have a data array of length 3, containing
          the two qubit indices and the rotation angle.

        The parsing of the data array is delegated to the operation
        class itself, and it is responsible for advancing the iterator
        as it consumes the data.
        """
        source_idx: int = next(it)
        operation_idx: int = next(it)
        source = Source(source_idx)
        operation: Operation | None = None
        match operation_idx:
            case 0:
                operation = BatchStart.from_iterator(it)
            case 1:
                operation = QAlloc.from_iterator(it)
            case 2:
                operation = QFree.from_iterator(it)
            case 3:
                operation = Reset.from_iterator(it)
            case 4:
                operation = MeasureRequest.from_iterator(it)
            case 5:
                operation = FutureRead.from_iterator(it)
            case 6:
                operation = Rxy.from_iterator(it)
            case 7:
                operation = Rz.from_iterator(it)
            case 8:
                operation = Rzz.from_iterator(it)
            case 9:
                operation = CustomOperation.from_iterator(it)
            case 10:
                operation = LocalBarrier.from_iterator(it)
            case 11:
                operation = GlobalBarrier.from_iterator(it)
            case 12:
                operation = MeasureLeakedRequest.from_iterator(it)
        if operation is None:
            raise ValueError(f"Unknown instruction operation index {operation_idx}")
        return Instruction(source=source, operation=operation)


class ShotInstructions:
    instructions: list

    def __init__(self):
        self.instructions = []

    def extend(self, instructions: list):
        self.instructions.extend(instructions)

    def __iter__(self):
        """
        Parses the data provided by Selene.

        The data is in the form of instructions of arbitrary
        length. Instruction.from_iterator is used to parse a
        single record, advancing the iterator as it goes.
        """
        it = iter(self.instructions)
        while True:
            try:
                yield Instruction.from_iterator(it)
            except StopIteration:
                break

    def _get_circuit(
        self, source: Source, init_qubits: int | None = None
    ) -> "pytket.Circuit":
        assert PYTKET_AVAILABLE, "pytket is not available"
        circuit = pytket.Circuit()
        for instruction in self:
            if instruction.source == source:
                instruction.operation.append_to_circuit(circuit)
        return circuit

    def _get_list_of_dicts(self, source: Source) -> list[dict]:
        result = []
        for instruction in self:
            if instruction.source == source:
                result.append(instruction.operation.to_dict())
        return result

    def get_user_circuit(self) -> "pytket.Circuit":
        return self._get_circuit(Source.USER)

    def get_optimiser_output(self) -> list[dict[Any, Any]]:
        return self._get_list_of_dicts(Source.OPTIMISER)

    def dump(self) -> None:
        for instruction in self:
            print(f"{instruction.source}: {instruction.operation}")


class CircuitExtractor(EventHook):
    shots: list[ShotInstructions]

    def get_selene_flags(self) -> list[str]:
        """
        When given --provide-instruction-log, Selene will emit
        an INSTRUCTIONLOG tag to the results stream, followed
        by a dump of all instructions that were logged from
        e.g. the user program or the runtime.
        """
        return ["provide_instruction_log"]

    def __init__(self):
        self.shots = []

    def try_invoke(self, tag: str, data: list) -> bool:
        if tag != "INSTRUCTIONLOG":
            return False
        self.shots[-1].extend(data)
        return True

    def on_new_shot(self):
        self.shots.append(ShotInstructions())
