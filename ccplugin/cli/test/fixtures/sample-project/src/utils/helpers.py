import os
import sys
from os.path import join, exists
from typing import Optional, List

__all__ = ["process_data", "DataProcessor"]


class DataProcessor:
    """Processes data files."""

    def __init__(self, path: str):
        self.path = path

    def run(self) -> bool:
        return True


def process_data(input_path: str, output_path: str) -> Optional[dict]:
    """Process data from input to output."""
    if not exists(input_path):
        return None
    return {"status": "ok"}


def _internal_helper():
    """This is private, but still top-level."""
    pass
