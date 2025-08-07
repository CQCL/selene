from pathlib import Path
import struct
from typing import Callable, Generic, Iterable, TypeVar

import numpy as np
from dataclasses import dataclass


T = TypeVar("T")


@dataclass(frozen=True)
class TracedState(Generic[T]):
    """The result of tracing out qubits from a SeleneQuestState, leaving a probabilistic
    mix of states. This class represents a single state in the mix."""

    #: The probability of this state in the mix
    probability: float
    #: The state vector of remaining qubits after tracing out.
    state: T


@dataclass
class SeleneQuestState:
    """A quantum state in the Selene Quest simulator, as reported by `state_result` calls."""

    #: Complex vector of size 2^total_qubits
    state: np.ndarray
    #: Total number of qubits in the state, i.e. n_qubits param to run_shots
    total_qubits: int
    #: User-specified qubits, in order of their specification
    specified_qubits: list[int]

    def get_density_matrix(self, zero_threshold: float = 1e-12) -> np.ndarray:
        """
        Get the reduced density matrix of the state, tracing out unspecified qubits.

        Parameters:
        ----------
        zero_threshold: float
            The threshold for setting small values to zero. This is used to remove numerical noise.
            Any component that is less than max_magnitude * zero_threshold will be reset to zero.
            Default is 1e-12.

        """
        state_tensor = self.state.reshape([2] * self.total_qubits)

        # move all specified qubits to the end, in the user-specified order
        n_specified = len(self.specified_qubits)
        n_unspecified = self.total_qubits - n_specified
        permutation_lhs = []
        permutation_rhs = []
        # Note: QuEST uses the convention that qubit 0 is the least significant bit.
        # Thus to iterate over qubits and corresponding statevector indices, we need
        # to iterate from left to right in one, right to left in the other.
        for qubit_id, bit_index in enumerate(reversed(range(self.total_qubits))):
            if qubit_id in self.specified_qubits:
                permutation_rhs.append(bit_index)
            else:
                permutation_lhs.append(bit_index)
        permutation = permutation_lhs + permutation_rhs
        permuted = np.transpose(state_tensor, permutation)
        # state_tensor is now in the shape ([2]*n_unspecified + [2]*n_specified).
        # reshape to a matrix
        reshaped = permuted.reshape((2**n_unspecified, 2**n_specified))
        # and trace out the unspecified qubits
        result = np.einsum("ai,aj->ij", reshaped, np.conj(reshaped))
        # the shape is now (2**n_specified, 2**n_specified)
        assert result.shape == (2**n_specified, 2**n_specified)

        if zero_threshold > 0:
            # set small (relative) values to zero for a cleaner output
            max_magnitude = np.max(np.abs(result))
            zero_threshold = max_magnitude * zero_threshold
            im = result.imag
            re = result.real
            im[np.abs(im) < zero_threshold] = 0
            re[np.abs(re) < zero_threshold] = 0
            result = re + 1j * im
        return result

    def get_state_vector_distribution(
        self, zero_threshold=1e-12
    ) -> list[TracedState[np.ndarray]]:
        """
        The reduced density matrix may be written as
        :math:`\\rho = \\sum_i p_i |i\\rangle \\langle i|`,
        where |i\\rangle are state vectors in the Hilbert space of the specified qubits,
        and p_i is the classical probability of the specified qubits being in the respective
        state after others have been measured.

        This is not a unique representation (by the Schrodinger-HJW theorem), but we here use
        a canonical decomposition.
        """
        density_matrix = self.get_density_matrix()
        result = []
        eigenvalues, eigenstates = np.linalg.eig(density_matrix)
        if zero_threshold > 0:
            # set small (relative) values to zero for a cleaner output
            max_magnitude = np.max(np.abs(eigenstates))
            zero_threshold_mag = max_magnitude * zero_threshold
            im = eigenstates.imag
            re = eigenstates.real
            im[np.abs(im) < zero_threshold_mag] = 0
            re[np.abs(re) < zero_threshold_mag] = 0
            eigenstates = re + 1j * im
            # apply a global phase shift to make the first
            # non-zero component real and positive
            for state_idx in range(eigenstates.shape[1]):
                # find phase of the first non-zero component
                phase = 1
                for component in eigenstates[:, state_idx]:
                    if np.abs(component) > 0:
                        phase = component / np.abs(component)
                        break
                # shift the whole state by its conjugate to
                # make the first component real and positive
                eigenstates[:, state_idx] *= np.conj(phase)

        max_eigenvalue = np.max(np.abs(eigenvalues))
        for i, eigenvalue in enumerate(eigenvalues):
            if abs(eigenvalue) < max_eigenvalue * zero_threshold:
                continue
            result.append(
                TracedState(
                    probability=abs(eigenvalue),
                    state=eigenstates[:, i],
                )
            )
        return result

    def get_single_state(self, zero_threshold=1e-12) -> np.ndarray:
        """
        Assume that the state is a pure state and return it.

        This is meant to be used when the user is requesting the state on all
        qubits, or on a subset that is not entangled with the rest.

        This function is a shorthand for ``get_state_vector_distribution`` that checks
        that there is a single vector with non-zero probability in the distribution of
        eigenvectors of the reduced density matrix, implying that it is a pure state.

        Raises ValueError if the state is not a pure state.

        """

        return self._get_single(
            all_getter=self.get_state_vector_distribution,
            zero_threshold=zero_threshold,
        )

    def _get_single(
        self,
        all_getter: Callable[[float], Iterable[TracedState[T]]],
        zero_threshold: float,
    ) -> T:
        """
        Get the single state of the specified qubits, assuming that the state is a pure state.
        This is a helper method for get_single_state.
        """
        all_states = list(all_getter(zero_threshold))

        if len(all_states) != 1:
            raise ValueError("The state is not a pure state.")
        return all_states[0].state

    def get_dirac_notation(self, zero_threshold=1e-12) -> list[TracedState]:
        try:
            from sympy import nsimplify, Add
            from sympy.physics.quantum.state import Ket

            width = len(self.specified_qubits)

            def simplify_state(tr_st: TracedState[np.ndarray]) -> TracedState:
                terms = []
                probability = nsimplify(tr_st.probability)
                max_amplitude = np.max(np.abs(tr_st.state))
                for i, amplitude in enumerate(tr_st.state):
                    if abs(amplitude) < max_amplitude * zero_threshold:
                        continue
                    coefficient = nsimplify(amplitude)
                    basis_str = f"{i:0{width}b}"
                    ket = Ket(basis_str)
                    terms.append(coefficient * ket)
                assert len(terms) > 0, (
                    "At least one ket state must have non-zero amplitude"
                )
                return TracedState(probability=probability, state=Add(*terms))
        except ImportError:
            import sys

            print(
                "Note: Install sympy to see prettier dirac notation output.",
                file=sys.stderr,
            )

            def simplify_state(
                tr_st: TracedState[np.ndarray],
            ) -> TracedState:
                terms = []
                max_amplitude = np.max(np.abs(tr_st.state))
                for i, amplitude in enumerate(tr_st.state):
                    if abs(amplitude) < max_amplitude * zero_threshold:
                        continue
                    ket = f"{amplitude}|{bin(i)[2:]}>"
                    terms.append(ket)
                assert len(terms) > 0, (
                    "At least one ket state must have non-zero amplitude"
                )
                return TracedState(
                    probability=tr_st.probability, state=" + ".join(terms)
                )

        state_vector = self.get_state_vector_distribution(zero_threshold=zero_threshold)
        result = [simplify_state(tr_st) for tr_st in state_vector]
        return result

    def get_single_dirac_notation(self, zero_threshold=1e-12) -> TracedState:
        """
        Get the single state of the specified qubits in Dirac notation,
        assuming that the state is a pure state.
        """
        return self._get_single(
            all_getter=self.get_dirac_notation,
            zero_threshold=zero_threshold,
        )

    @staticmethod
    def parse_from_file(filename: Path, cleanup: bool = True) -> "SeleneQuestState":
        with open(filename, "rb") as f:
            magic = f.read(12)
            if magic != b"selene-quest":
                raise ValueError("Invalid state file format")
            header_head = f.read(16)
            total_qubits, n_specified_qubits = struct.unpack("<QQ", header_head)
            specified_qubits = []
            for i in range(n_specified_qubits):
                specified_qubits.append(struct.unpack("<Q", f.read(8))[0])
            state_size = 2**total_qubits
            state = np.fromfile(
                f,
                dtype=np.complex128,
                count=state_size,
            )
        if cleanup:
            filename.unlink()
        return SeleneQuestState(state, total_qubits, specified_qubits)
