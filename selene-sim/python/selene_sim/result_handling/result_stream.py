from __future__ import annotations
from struct import unpack, error
from typing import Iterator, Any

from .data_stream import DataStream

# An individual data entry can be a primitive type
DataPrimitive = int | float | bool
# A full record can be a primitive type or a list of primitive types
DataValue = DataPrimitive | list[DataPrimitive]
# A tagged result is a tuple of a string tag and a data value
TaggedResult = tuple[str, DataValue]


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
        self.done = False
        self.is_mid_shot = False
        self.stream = stream

    def __iter__(self) -> Iterator[tuple]:
        """Results generator"""
        while t := self._parse_row():
            yield t

    def maybe_get_chunk(self, length: int) -> bytes | None:
        """Try to get a chunk of data from the stream.

        Use this method when you expect the stream to either be exhausted or contain
        a complete chunk of data of length `length`. Partial chunks are an error.

        An example use is when checking to see if another record is present, e.g. in
        a loop over shots. If it is not present then it is not a hard error, it is simply
        finished.

        If the chunk is present, returns it.
        If there is no data left to read, returns None.
        If there is partial data left, an exception is raised.
        """
        result = self.stream.read_chunk(length)
        if len(result) < length:
            if len(result) == 0:
                return None
            else:
                raise Exception(
                    f"Truncated result stream: read {len(result)} bytes, "
                    f"expected {length}"
                )
        return result

    def get_chunk(self, length: int) -> bytes:
        """Get a chunk of data from the stream.

        Use this method when the stream should contain the data of length `length`, and
        it not containing this data is considered an error. For example, when in the
        middle of parsing a record, missing data implies a crash or parsing error.
        """
        result = self.stream.read_chunk(length)
        if len(result) < length:
            raise Exception("Parsing error: Unexpected end of stream")
        return result

    def next_shot(self) -> None:
        self.is_mid_shot = False
        self.stream.next_shot()

    def _parse_row(self) -> tuple[Any, ...]:
        """Parse results stream row and return a tuple"""
        res: list[Any] = []
        try:
            # Try to get the time cursor.
            tc_chunk = self.maybe_get_chunk(8)
            # If None is returned, the stream is empty and we are done.
            if tc_chunk is None:
                if self.is_mid_shot:
                    raise ValueError("Unexpected end of stream in the middle of a shot")
                return tuple()
            self.is_mid_shot = True

            (tc,) = unpack("Q", tc_chunk)
            # If the time cursor is EOS, we have reached the end of the stream
            if tc == self.EOS:
                self.is_mid_shot = False
                return tuple()

            # Loop through entries in the row
            while True:
                datatype, size = unpack("HH", self.get_chunk(4))
                match (datatype, size):
                    case (self.UINT_TAG, 0):
                        (val,) = unpack("Q", self.get_chunk(8))
                        res.append(val)
                    case (self.UINT_TAG, sz):
                        vals = unpack(f"{sz}Q", self.get_chunk(sz * 8))
                        res.append(list(vals))
                    case (self.INT_TAG, 0):
                        (val,) = unpack("q", self.get_chunk(8))
                        res.append(val)
                    case (self.INT_TAG, sz):
                        vals = unpack(f"{sz}q", self.get_chunk(sz * 8))
                        res.append(list(vals))
                    case (self.FLT_TAG, 0):
                        (val,) = unpack("d", self.get_chunk(8))
                        res.append(val)
                    case (self.FLT_TAG, sz):
                        vals = unpack(f"{sz}d", self.get_chunk(sz * 8))
                        res.append(list(vals))
                    case (self.BIT_TAG, 0):
                        (val,) = unpack("B", self.get_chunk(1))
                        res.append(val)
                    case (self.BIT_TAG, sz):
                        vals = unpack(f"{sz}B", self.get_chunk(sz))
                        res.append(list(vals))
                    case (self.STR_TAG, sz):
                        val = self.get_chunk(sz).decode("utf-8")
                        res.append(val)
                    case (self.BYTE_TAG, 0):
                        val = self.get_chunk(1)[0]
                        res.append(val)
                    case (self.BYTE_TAG, sz):
                        val = self.get_chunk(sz)
                        res.append(val)
                    case (self.END_TAG, 0):
                        break
                    case _:
                        raise ValueError(
                            f"Unexpected type {datatype} ({datatype:04x}) with length {size} ({size:04x})"
                        )
        except error:
            # struct parsing error, do not update the cursor, need more data
            return tuple()
        return tuple(res)
