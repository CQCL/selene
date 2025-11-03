from __future__ import annotations
from dataclasses import dataclass
from selene_sim.exceptions import SeleneRuntimeError, SeleneTimeoutError
from struct import unpack
from typing import Iterator

from .data_stream import DataStream

# An individual data entry can be a primitive type
DataPrimitive = int | float | bool
# A full record can be a primitive type or a list of primitive types
DataValue = DataPrimitive | list[DataPrimitive]
# A tagged result is a tuple of a string tag and a data value
TaggedResult = tuple[str, DataValue]

StreamEntryPart = int | float | bool | str | bytes | list[int | float | bool]


@dataclass
class StreamEntry:
    tag: str
    values: list[StreamEntryPart]


class ResultStream:
    # type tags
    END_TAG = 0
    UINT_TAG = 1
    FLT_TAG = 2
    STR_TAG = 3
    BIT_TAG = 4
    INT_TAG = 5
    BYTE_TAG = 9116

    # end of stream tag
    EOS = 0xFFFFFFFFFFFFFFFF

    def __init__(self, stream: DataStream) -> None:
        super().__init__()
        # Underlying data stream
        self.stream = stream
        # Indicates whether the stream has been fully read
        self.done = False
        # Indicates whether the stream has been tainted by an error by processing
        # code. Once tainted, no further reading should be attempted.
        self.tainted = False

    def taint(self) -> None:
        """Taint the stream to prevent further reading"""
        self.tainted = True

    def __iter__(self) -> Iterator[StreamEntry]:
        """Loop over entry extraction and yield results"""
        if self.tainted:
            raise SeleneRuntimeError("Result stream has been tainted by a prior error")
        if self.done:
            raise SeleneRuntimeError("Result stream is already exhausted")
        while t := self._extract_entry():
            yield t

    def get_chunk(self, length: int) -> bytes:
        """Get a chunk of data from the stream.

        Use this method when the stream should contain the data of length `length`, and
        it not containing this data is considered an error. For example, when in the
        middle of parsing a record, missing data implies a crash or parsing error.
        """
        result = self.stream.read_chunk(length)
        if len(result) < length:
            raise SeleneRuntimeError("Parsing error: Unexpected end of stream")
        return result

    def next_shot(self) -> None:
        self.stream.next_shot()

    def _extract_tag(self) -> str:
        datatype, size = unpack("HH", self.get_chunk(4))
        if datatype != self.STR_TAG:
            raise SeleneRuntimeError(
                f"Expected tag as the first entry in a stream record, but got type {datatype} ({datatype:04x}) for with length {size} ({size:04x})"
            )
        val = self.get_chunk(size).decode("utf-8")
        return val

    def _extract_values(self) -> list[StreamEntryPart]:
        # Loop through entries in the row
        values: list[StreamEntryPart] = []
        while True:
            try:
                # Prefixing any entry is a type tag and a size.
                # Size 0 indicates a single primitive value,
                # size > 0 indicates an array of that many values.
                datatype, size = unpack("HH", self.get_chunk(4))
                match (datatype, size):
                    case (self.UINT_TAG, 0):
                        (val,) = unpack("Q", self.get_chunk(8))
                        values.append(val)
                    case (self.UINT_TAG, sz):
                        vals = unpack(f"{sz}Q", self.get_chunk(sz * 8))
                        values.append(list(vals))
                    case (self.INT_TAG, 0):
                        (val,) = unpack("q", self.get_chunk(8))
                        values.append(val)
                    case (self.INT_TAG, sz):
                        vals = unpack(f"{sz}q", self.get_chunk(sz * 8))
                        values.append(list(vals))
                    case (self.FLT_TAG, 0):
                        (val,) = unpack("d", self.get_chunk(8))
                        values.append(val)
                    case (self.FLT_TAG, sz):
                        vals = unpack(f"{sz}d", self.get_chunk(sz * 8))
                        values.append(list(vals))
                    case (self.BIT_TAG, 0):
                        (val,) = unpack("B", self.get_chunk(1))
                        values.append(val)
                    case (self.BIT_TAG, sz):
                        vals = unpack(f"{sz}B", self.get_chunk(sz))
                        values.append(list(vals))
                    case (self.STR_TAG, sz):
                        val = self.get_chunk(sz).decode("utf-8")
                        values.append(val)
                    case (self.BYTE_TAG, 0):
                        val = self.get_chunk(1)[0]
                        values.append(val)
                    case (self.BYTE_TAG, sz):
                        val = self.get_chunk(sz)
                        values.append(val)
                    case (self.END_TAG, 0):
                        break
                    case _:
                        raise SeleneRuntimeError(
                            f"Unexpected type {datatype} ({datatype:04x}) with length {size} ({size:04x})"
                        )
            except SeleneRuntimeError as e:
                # propagate runtime errors
                raise e
            except SeleneTimeoutError as e:
                # propagate timeout errors
                raise e
            except Exception as e:
                # struct parsing error, do not update the cursor, need more data
                raise SeleneRuntimeError(
                    "Parsing error: Malformed result stream"
                ) from e
        return values

    def _extract_entry(self) -> StreamEntry | None:
        """Parse results stream row and return a tuple"""
        # Get the time cursor.
        tc_chunk = self.get_chunk(8)
        (tc,) = unpack("Q", tc_chunk)
        # If the time cursor is EOS, we have reached the end of the stream
        if tc == self.EOS:
            self.done = True
            return None

        tag = self._extract_tag()
        values = self._extract_values()
        return StreamEntry(tag=tag, values=values)
