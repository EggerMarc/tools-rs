from typing import Literal
from tools_rs import tool


@tool()
def greet(name: str, times: int = 1) -> str:
    """Greet someone.

    Args:
        name: Person to greet
        times: How many times to greet
    """
    return ("Hello, " + name + "! ") * times


@tool(requires_approval=True)
def convert(value: float, unit: Literal["celsius", "fahrenheit"]) -> float:
    """Convert temperature.

    Args:
        value: Temperature value
        unit: Target unit
    """
    if unit == "celsius":
        return (value - 32) * 5 / 9
    return value * 9 / 5 + 32
