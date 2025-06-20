from pathlib import Path


def get_include_directory():
    """
    Returns the path to the include directory for selene_core.
    This is used to find the headers for the selene_core library.
    """
    return Path(__file__).parent / "_dist/include"
