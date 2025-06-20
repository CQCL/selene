import pickle
import pytest

from selene_sim.exceptions import (
    SeleneBuildError,
    SeleneStartupError,
    SeleneRuntimeError,
    SelenePanicError,
)


@pytest.mark.parametrize(
    "error_class, kwargs",
    [
        (
            SeleneBuildError,
            {
                "message": "Exception",
                "stdout": "stdout",
                "stderr": "stderr",
            },
        ),
        (
            SeleneStartupError,
            {
                "message": "Exception",
                "stdout": "stdout",
                "stderr": "stderr",
            },
        ),
        (
            SeleneRuntimeError,
            {
                "message": "Exception",
                "stdout": "stdout",
                "stderr": "stderr",
            },
        ),
        (
            SelenePanicError,
            {
                "message": "Exception",
                "code": "foo",
                "stdout": "stdout",
                "stderr": "stderr",
            },
        ),
    ],
)
def test_pickle_and_unpickle_selene_startup_error(error_class, kwargs):
    error = error_class(**kwargs)
    unpickled = pickle.loads(pickle.dumps(error))

    assert isinstance(unpickled, error_class)

    for k, v in kwargs.items():
        assert getattr(unpickled, k) == v
